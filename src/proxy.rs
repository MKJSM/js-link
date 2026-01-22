//! Proxy support module for WebSocket connections
//!
//! Supports automatic proxy detection from environment variables and
//! various proxy types: HTTP, HTTPS, SOCKS4, SOCKS5.

use std::env;
use std::io::{Error as IoError, ErrorKind};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks4Stream;
use tokio_socks::tcp::Socks5Stream;

/// Proxy configuration detected from environment
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProxyType {
    Http,
    Https,
    Socks4,
    Socks5,
}

impl ProxyConfig {
    /// Parse a proxy URL into a ProxyConfig
    pub fn from_url(url: &str) -> Option<Self> {
        let url = url.trim();
        if url.is_empty() {
            return None;
        }

        // Determine proxy type from scheme
        let (proxy_type, rest) = if url.starts_with("socks4://") || url.starts_with("socks4a://") {
            (ProxyType::Socks4, url.split_once("://").map(|(_, r)| r)?)
        } else if url.starts_with("socks5://") || url.starts_with("socks5h://") || url.starts_with("socks://") {
            (ProxyType::Socks5, url.split_once("://").map(|(_, r)| r)?)
        } else if url.starts_with("https://") {
            (ProxyType::Https, url.split_once("://").map(|(_, r)| r)?)
        } else if url.starts_with("http://") {
            (ProxyType::Http, url.split_once("://").map(|(_, r)| r)?)
        } else {
            // Default to HTTP proxy if no scheme
            (ProxyType::Http, url)
        };

        // Parse auth and host:port
        let (auth, host_port) = if rest.contains('@') {
            let parts: Vec<&str> = rest.splitn(2, '@').collect();
            (Some(parts[0]), parts[1])
        } else {
            (None, rest)
        };

        // Parse username:password
        let (username, password) = if let Some(auth_str) = auth {
            if auth_str.contains(':') {
                let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
                (Some(parts[0].to_string()), Some(parts[1].to_string()))
            } else {
                (Some(auth_str.to_string()), None)
            }
        } else {
            (None, None)
        };

        // Parse host:port (remove trailing path if any)
        let host_port = host_port.split('/').next()?;
        let (host, port) = if host_port.contains(':') {
            let parts: Vec<&str> = host_port.rsplitn(2, ':').collect();
            let port = parts[0].parse().ok()?;
            (parts[1].to_string(), port)
        } else {
            // Default ports based on proxy type
            let default_port = match proxy_type {
                ProxyType::Http => 80,
                ProxyType::Https => 443,
                ProxyType::Socks4 | ProxyType::Socks5 => 1080,
            };
            (host_port.to_string(), default_port)
        };

        Some(ProxyConfig {
            proxy_type,
            host,
            port,
            username,
            password,
        })
    }
}

/// Detect proxy configuration from environment variables
pub fn detect_proxy(target_url: &str) -> Option<ProxyConfig> {
    let is_wss = target_url.starts_with("wss://");

    // Check NO_PROXY / no_proxy first
    if should_bypass_proxy(target_url) {
        log::debug!("Target URL matches NO_PROXY, using direct connection");
        return None;
    }

    // Priority order for proxy detection:
    // 1. ALL_PROXY / all_proxy (applies to all protocols)
    // 2. HTTPS_PROXY / https_proxy (for wss://)
    // 3. HTTP_PROXY / http_proxy (for ws://)
    // 4. SOCKS_PROXY / socks_proxy (fallback for SOCKS)

    // Check ALL_PROXY first
    if let Some(proxy) = env::var("ALL_PROXY")
        .or_else(|_| env::var("all_proxy"))
        .ok()
        .and_then(|url| ProxyConfig::from_url(&url))
    {
        log::info!("Using ALL_PROXY: {}:{}", proxy.host, proxy.port);
        return Some(proxy);
    }

    // For wss://, prefer HTTPS_PROXY
    if is_wss {
        if let Some(proxy) = env::var("HTTPS_PROXY")
            .or_else(|_| env::var("https_proxy"))
            .ok()
            .and_then(|url| ProxyConfig::from_url(&url))
        {
            log::info!("Using HTTPS_PROXY for wss://: {}:{}", proxy.host, proxy.port);
            return Some(proxy);
        }
    }

    // Check HTTP_PROXY
    if let Some(proxy) = env::var("HTTP_PROXY")
        .or_else(|_| env::var("http_proxy"))
        .ok()
        .and_then(|url| ProxyConfig::from_url(&url))
    {
        log::info!("Using HTTP_PROXY: {}:{}", proxy.host, proxy.port);
        return Some(proxy);
    }

    // Check SOCKS_PROXY as fallback
    if let Some(proxy) = env::var("SOCKS_PROXY")
        .or_else(|_| env::var("socks_proxy"))
        .ok()
        .and_then(|url| ProxyConfig::from_url(&url))
    {
        log::info!("Using SOCKS_PROXY: {}:{}", proxy.host, proxy.port);
        return Some(proxy);
    }

    log::debug!("No proxy detected, will use direct connection");
    None
}

/// Check if a URL should bypass the proxy based on NO_PROXY
fn should_bypass_proxy(target_url: &str) -> bool {
    let no_proxy = env::var("NO_PROXY")
        .or_else(|_| env::var("no_proxy"))
        .unwrap_or_default();

    if no_proxy.is_empty() {
        return false;
    }

    // Extract host from URL
    let host = extract_host(target_url);
    if host.is_empty() {
        return false;
    }

    // Check each entry in NO_PROXY
    for entry in no_proxy.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        // Handle "*" (bypass all)
        if entry == "*" {
            return true;
        }

        // Handle domain suffix matching (e.g., ".example.com")
        if entry.starts_with('.') {
            if host.ends_with(entry) || host == &entry[1..] {
                return true;
            }
        } else if host == entry || host.ends_with(&format!(".{}", entry)) {
            return true;
        }
    }

    false
}

/// Extract host from a WebSocket URL
fn extract_host(url: &str) -> String {
    let url = url
        .strip_prefix("wss://")
        .or_else(|| url.strip_prefix("ws://"))
        .unwrap_or(url);

    // Remove path and query string
    let host_port = url.split('/').next().unwrap_or(url);

    // Remove port
    if let Some(idx) = host_port.rfind(':') {
        // Check if this is an IPv6 address
        if host_port.contains('[') {
            if let Some(end_bracket) = host_port.rfind(']') {
                if idx > end_bracket {
                    return host_port[..idx].to_string();
                }
            }
            return host_port.to_string();
        }
        return host_port[..idx].to_string();
    }

    host_port.to_string()
}

/// Extract host and port from a WebSocket URL
pub fn extract_host_port(url: &str) -> Result<(String, u16), IoError> {
    let is_wss = url.starts_with("wss://");
    let default_port = if is_wss { 443 } else { 80 };

    let url = url
        .strip_prefix("wss://")
        .or_else(|| url.strip_prefix("ws://"))
        .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "Invalid WebSocket URL"))?;

    // Remove path and query string
    let host_port = url.split('/').next().unwrap_or(url);

    // Handle IPv6 addresses
    if host_port.starts_with('[') {
        if let Some(end_bracket) = host_port.find(']') {
            let host = &host_port[1..end_bracket];
            let port = if host_port.len() > end_bracket + 2 && host_port.chars().nth(end_bracket + 1) == Some(':') {
                host_port[end_bracket + 2..]
                    .parse()
                    .map_err(|_| IoError::new(ErrorKind::InvalidInput, "Invalid port"))?
            } else {
                default_port
            };
            return Ok((host.to_string(), port));
        }
    }

    // Regular host:port
    if let Some(idx) = host_port.rfind(':') {
        let host = &host_port[..idx];
        let port = host_port[idx + 1..]
            .parse()
            .map_err(|_| IoError::new(ErrorKind::InvalidInput, "Invalid port"))?;
        Ok((host.to_string(), port))
    } else {
        Ok((host_port.to_string(), default_port))
    }
}

/// A wrapper for different stream types that can be used for WebSocket connections
pub enum ProxyStream {
    Direct(TcpStream),
    Socks4(Socks4Stream<TcpStream>),
    Socks5(Socks5Stream<TcpStream>),
    HttpTunnel(TcpStream),
}

impl AsyncRead for ProxyStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Direct(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            ProxyStream::Socks4(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            ProxyStream::Socks5(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            ProxyStream::HttpTunnel(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ProxyStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            ProxyStream::Direct(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            ProxyStream::Socks4(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            ProxyStream::Socks5(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            ProxyStream::HttpTunnel(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            ProxyStream::Direct(s) => std::pin::Pin::new(s).poll_flush(cx),
            ProxyStream::Socks4(s) => std::pin::Pin::new(s).poll_flush(cx),
            ProxyStream::Socks5(s) => std::pin::Pin::new(s).poll_flush(cx),
            ProxyStream::HttpTunnel(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            ProxyStream::Direct(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            ProxyStream::Socks4(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            ProxyStream::Socks5(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            ProxyStream::HttpTunnel(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

/// Connect through an HTTP CONNECT proxy
async fn connect_http_proxy(
    proxy: &ProxyConfig,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream, IoError> {
    let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
    log::debug!("Connecting to HTTP proxy at {}", proxy_addr);

    let mut stream = TcpStream::connect(&proxy_addr).await?;

    // Build CONNECT request
    let connect_request = if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
        use base64::Engine;
        let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
        format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Authorization: Basic {}\r\nProxy-Connection: Keep-Alive\r\n\r\n",
            target_host, target_port, target_host, target_port, credentials
        )
    } else {
        format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n\r\n",
            target_host, target_port, target_host, target_port
        )
    };

    log::debug!("Sending CONNECT request to proxy");
    stream.write_all(connect_request.as_bytes()).await?;

    // Read response
    let mut reader = BufReader::new(&mut stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    log::debug!("Proxy response: {}", response_line.trim());

    // Check for successful connection (HTTP/1.x 200)
    if !response_line.contains(" 200 ") {
        return Err(IoError::new(
            ErrorKind::ConnectionRefused,
            format!("Proxy CONNECT failed: {}", response_line.trim()),
        ));
    }

    // Read remaining headers until empty line
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line == "\r\n" || line == "\n" || line.is_empty() {
            break;
        }
    }

    log::info!("HTTP CONNECT tunnel established to {}:{}", target_host, target_port);
    Ok(stream)
}

/// Connect through a SOCKS4 proxy
async fn connect_socks4_proxy(
    proxy: &ProxyConfig,
    target_host: &str,
    target_port: u16,
) -> Result<Socks4Stream<TcpStream>, IoError> {
    let proxy_addr: SocketAddr = format!("{}:{}", proxy.host, proxy.port)
        .parse()
        .map_err(|e| IoError::new(ErrorKind::InvalidInput, format!("Invalid proxy address: {}", e)))?;

    log::debug!("Connecting through SOCKS4 proxy at {}", proxy_addr);

    // SOCKS4a supports hostname resolution by the proxy
    let stream = Socks4Stream::connect(proxy_addr, (target_host, target_port))
        .await
        .map_err(|e| IoError::new(ErrorKind::ConnectionRefused, format!("SOCKS4 connection failed: {}", e)))?;

    log::info!("SOCKS4 tunnel established to {}:{}", target_host, target_port);
    Ok(stream)
}

/// Connect through a SOCKS5 proxy
async fn connect_socks5_proxy(
    proxy: &ProxyConfig,
    target_host: &str,
    target_port: u16,
) -> Result<Socks5Stream<TcpStream>, IoError> {
    let proxy_addr: SocketAddr = format!("{}:{}", proxy.host, proxy.port)
        .parse()
        .map_err(|e| IoError::new(ErrorKind::InvalidInput, format!("Invalid proxy address: {}", e)))?;

    log::debug!("Connecting through SOCKS5 proxy at {}", proxy_addr);

    let stream = if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
        Socks5Stream::connect_with_password(proxy_addr, (target_host, target_port), username, password)
            .await
            .map_err(|e| IoError::new(ErrorKind::ConnectionRefused, format!("SOCKS5 auth connection failed: {}", e)))?
    } else {
        Socks5Stream::connect(proxy_addr, (target_host, target_port))
            .await
            .map_err(|e| IoError::new(ErrorKind::ConnectionRefused, format!("SOCKS5 connection failed: {}", e)))?
    };

    log::info!("SOCKS5 tunnel established to {}:{}", target_host, target_port);
    Ok(stream)
}

/// Connect to a target, automatically detecting and using proxy if available
pub async fn connect_with_proxy(target_url: &str) -> Result<ProxyStream, IoError> {
    let (target_host, target_port) = extract_host_port(target_url)?;

    // Detect proxy from environment
    if let Some(proxy) = detect_proxy(target_url) {
        match proxy.proxy_type {
            ProxyType::Http | ProxyType::Https => {
                let stream = connect_http_proxy(&proxy, &target_host, target_port).await?;
                Ok(ProxyStream::HttpTunnel(stream))
            }
            ProxyType::Socks4 => {
                let stream = connect_socks4_proxy(&proxy, &target_host, target_port).await?;
                Ok(ProxyStream::Socks4(stream))
            }
            ProxyType::Socks5 => {
                let stream = connect_socks5_proxy(&proxy, &target_host, target_port).await?;
                Ok(ProxyStream::Socks5(stream))
            }
        }
    } else {
        // Direct connection
        let addr = format!("{}:{}", target_host, target_port);
        log::debug!("Connecting directly to {}", addr);
        let stream = TcpStream::connect(&addr).await?;
        log::info!("Direct connection established to {}", addr);
        Ok(ProxyStream::Direct(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_proxy_url() {
        let config = ProxyConfig::from_url("http://proxy.example.com:8080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Http);
        assert_eq!(config.host, "proxy.example.com");
        assert_eq!(config.port, 8080);
        assert!(config.username.is_none());
    }

    #[test]
    fn test_parse_proxy_with_auth() {
        let config = ProxyConfig::from_url("http://user:pass@proxy.example.com:8080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Http);
        assert_eq!(config.host, "proxy.example.com");
        assert_eq!(config.port, 8080);
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
    }

    #[test]
    fn test_parse_socks5_proxy_url() {
        let config = ProxyConfig::from_url("socks5://127.0.0.1:1080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Socks5);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 1080);
    }

    #[test]
    fn test_parse_socks4_proxy_url() {
        let config = ProxyConfig::from_url("socks4://localhost:1080").unwrap();
        assert_eq!(config.proxy_type, ProxyType::Socks4);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1080);
    }

    #[test]
    fn test_extract_host_port() {
        let (host, port) = extract_host_port("ws://example.com:8080/path").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 8080);

        let (host, port) = extract_host_port("wss://secure.example.com/path").unwrap();
        assert_eq!(host, "secure.example.com");
        assert_eq!(port, 443);

        let (host, port) = extract_host_port("ws://localhost").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 80);
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("ws://example.com:8080/path"), "example.com");
        assert_eq!(extract_host("wss://secure.example.com/path"), "secure.example.com");
    }
}
