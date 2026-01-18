mod db;
mod environments;
mod executor;
mod folders;
mod network;
mod requests;
mod websocket;

use axum::{
    body::Body,
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use dotenvy::dotenv;
use rust_embed::RustEmbed;
use sailfish::TemplateOnce;
use std::env;
use tokio::net::TcpListener;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[derive(TemplateOnce)]
#[template(path = "index.stpl")]
struct IndexTemplate;

#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticAssets;

async fn index() -> Html<String> {
    let ctx = IndexTemplate;
    Html(ctx.render_once().unwrap())
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');

    match StaticAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime.as_ref()).unwrap(),
                )],
                Body::from(content.data),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}

#[tokio::main]
async fn main() {
    // File appender: rotate daily, store in ./logs, prefix with js-link, suffix with log
    let file_appender = rolling::Builder::new()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix("js-link")
        .filename_suffix("log")
        .build("./logs")
        .expect("failed to initialize rolling file appender");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Console layer
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()));

    // File layer
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()));

    // Register subscriber
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    println!(
        r#"
     ██╗███████╗      ██╗      ██╗███╗   ██╗██╗  ██╗
     ██║██╔════╝      ██║      ██║████╗  ██║██║ ██╔╝
     ██║███████╗█████╗██║      ██║██╔██╗ ██║█████╔╝ 
██   ██║╚════██║╚════╝██║      ██║██║╚██╗██║██╔═██╗ 
╚█████╔╝███████║      ███████╗ ██║██║ ╚████║██║  ██╗
 ╚════╝ ╚══════╝      ╚══════╝ ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝
    "#
    );

    let version = env!("CARGO_PKG_VERSION");
    log::info!("JS-Link v{} starting up...", version);

    // Load .env if it exists
    if dotenv().ok().is_some() {
        log::info!("Loaded environment variables from .env");
    }

    log::info!("Initializing database connection...");
    let pool = db::create_pool()
        .await
        .expect("Failed to create database pool");

    let app = Router::new()
        .route("/", get(index))
        .nest(
            "/api",
            folders::routes(pool.clone())
                .merge(requests::routes(pool.clone()))
                .merge(environments::routes(pool.clone()))
                .merge(network::routes(pool.clone()))
                .merge(executor::routes(pool.clone()))
                .merge(websocket::routes(pool)),
        )
        .route("/static/*path", get(static_handler));

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = TcpListener::bind(&addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    log::info!("──────────────────────────────────────────────────");
    log::info!("🚀 Server is ready and listening");
    log::info!("📡 Local:   http://localhost:{}", local_addr.port());
    log::info!(
        "🌐 Remote:  http://{}:{}",
        local_addr.ip(),
        local_addr.port()
    );

    if local_addr.ip().is_unspecified() {
        log::info!("💡 Note: Accessible on all network interfaces");
    }
    log::info!("──────────────────────────────────────────────────");

    axum::serve(listener, app).await.unwrap();
}
