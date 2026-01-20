use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{
        client::IntoClientRequest,
        http::HeaderValue,
        Message as TungsteniteMessage,
    },
};

use crate::db::DbPool;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    #[serde(rename = "connect")]
    Connect {
        url: String,
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
        #[serde(default)]
        auth_type: Option<String>,
        #[serde(default)]
        auth_token: Option<String>,
        #[serde(default)]
        auth_username: Option<String>,
        #[serde(default)]
        auth_password: Option<String>,
    },
    #[serde(rename = "disconnect")]
    Disconnect,
    #[serde(rename = "send")]
    Send { message: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    #[serde(rename = "connected")]
    Connected { url: String },
    #[serde(rename = "disconnected")]
    Disconnected { reason: String },
    #[serde(rename = "message")]
    Message { data: String, direction: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "info")]
    Info { message: String },
}

// Shared state for WebSocket connection
struct WsConnectionState {
    remote_write_tx: Option<mpsc::Sender<String>>,
    connected_url: Option<String>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(_pool): State<DbPool>) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut client_sender, mut client_receiver) = socket.split();

    // Channel for sending messages to the browser client
    let (to_client_tx, mut to_client_rx) = mpsc::channel::<WsServerMessage>(100);

    // Shared connection state
    let connection_state = Arc::new(Mutex::new(WsConnectionState {
        remote_write_tx: None,
        connected_url: None,
    }));

    // Task to forward messages to the browser client
    let send_to_client_task = tokio::spawn(async move {
        while let Some(msg) = to_client_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if client_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages from the browser client
    let conn_state = Arc::clone(&connection_state);
    let tx = to_client_tx.clone();

    while let Some(Ok(msg)) = client_receiver.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<WsClientMessage>(&text) {
                Ok(client_msg) => {
                    handle_client_message(client_msg, &conn_state, &tx).await;
                }
                Err(e) => {
                    log::error!("Failed to parse client message: {}", e);
                    let _ = tx
                        .send(WsServerMessage::Error {
                            message: format!("Invalid message format: {}", e),
                        })
                        .await;
                }
            }
        }
    }

    // Cleanup
    send_to_client_task.abort();

    // Close remote connection if still open
    let mut state = connection_state.lock().await;
    state.remote_write_tx = None;
    state.connected_url = None;
}

async fn handle_client_message(
    msg: WsClientMessage,
    conn_state: &Arc<Mutex<WsConnectionState>>,
    to_client_tx: &mpsc::Sender<WsServerMessage>,
) {
    match msg {
        WsClientMessage::Connect {
            url,
            headers,
            auth_type,
            auth_token,
            auth_username,
            auth_password,
        } => {
            log::info!("Connecting to WebSocket: {}", url);

            // Close existing connection if any
            {
                let mut state = conn_state.lock().await;
                state.remote_write_tx = None;
                state.connected_url = None;
            }

            // Build request with headers
            let request = match url.clone().into_client_request() {
                Ok(mut req) => {
                    // Add custom headers
                    if let Some(hdrs) = headers {
                        for (key, value) in hdrs {
                            if let (Ok(header_name), Ok(header_value)) = (
                                key.parse::<tokio_tungstenite::tungstenite::http::header::HeaderName>(),
                                HeaderValue::from_str(&value),
                            ) {
                                req.headers_mut().insert(header_name, header_value);
                            }
                        }
                    }

                    // Add auth headers
                    if let Some(auth) = auth_type {
                        match auth.as_str() {
                            "bearer" => {
                                if let Some(token) = auth_token {
                                    if let Ok(header_value) =
                                        HeaderValue::from_str(&format!("Bearer {}", token))
                                    {
                                        req.headers_mut().insert(
                                            tokio_tungstenite::tungstenite::http::header::AUTHORIZATION,
                                            header_value,
                                        );
                                    }
                                }
                            }
                            "basic" => {
                                if let (Some(username), Some(password)) =
                                    (auth_username, auth_password)
                                {
                                    use base64::Engine;
                                    let credentials =
                                        base64::engine::general_purpose::STANDARD.encode(format!(
                                            "{}:{}",
                                            username, password
                                        ));
                                    if let Ok(header_value) =
                                        HeaderValue::from_str(&format!("Basic {}", credentials))
                                    {
                                        req.headers_mut().insert(
                                            tokio_tungstenite::tungstenite::http::header::AUTHORIZATION,
                                            header_value,
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    req
                }
                Err(e) => {
                    log::error!("Failed to create WebSocket request: {}", e);
                    let _ = to_client_tx
                        .send(WsServerMessage::Error {
                            message: format!("Invalid WebSocket URL: {}", e),
                        })
                        .await;
                    return;
                }
            };

            // Connect to the remote WebSocket with headers
            match connect_async_with_config(request, None, false).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();

                    // Create channel for sending to remote
                    let (remote_tx, mut remote_rx) = mpsc::channel::<String>(100);

                    // Store the channel in state
                    {
                        let mut state = conn_state.lock().await;
                        state.remote_write_tx = Some(remote_tx);
                        state.connected_url = Some(url.clone());
                    }

                    // Notify client of successful connection
                    let _ = to_client_tx
                        .send(WsServerMessage::Connected { url: url.clone() })
                        .await;

                    // Task to write messages to remote WebSocket
                    let write_task = tokio::spawn(async move {
                        while let Some(msg) = remote_rx.recv().await {
                            if write
                                .send(TungsteniteMessage::Text(msg.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    });

                    // Task to read messages from remote WebSocket
                    let tx_for_read = to_client_tx.clone();
                    let conn_state_for_read = Arc::clone(conn_state);

                    tokio::spawn(async move {
                        while let Some(msg_result) = read.next().await {
                            match msg_result {
                                Ok(TungsteniteMessage::Text(text)) => {
                                    let _ = tx_for_read
                                        .send(WsServerMessage::Message {
                                            data: text.to_string(),
                                            direction: "received".to_string(),
                                        })
                                        .await;
                                }
                                Ok(TungsteniteMessage::Binary(data)) => {
                                    let _ = tx_for_read
                                        .send(WsServerMessage::Message {
                                            data: format!("[Binary: {} bytes]", data.len()),
                                            direction: "received".to_string(),
                                        })
                                        .await;
                                }
                                Ok(TungsteniteMessage::Close(_)) => {
                                    let _ = tx_for_read
                                        .send(WsServerMessage::Disconnected {
                                            reason: "Remote closed connection".to_string(),
                                        })
                                        .await;

                                    // Clear connection state
                                    let mut state = conn_state_for_read.lock().await;
                                    state.remote_write_tx = None;
                                    state.connected_url = None;
                                    break;
                                }
                                Ok(TungsteniteMessage::Ping(_))
                                | Ok(TungsteniteMessage::Pong(_)) => {
                                    // Handle ping/pong silently
                                }
                                Ok(TungsteniteMessage::Frame(_)) => {
                                    // Ignore raw frames
                                }
                                Err(e) => {
                                    let _ = tx_for_read
                                        .send(WsServerMessage::Error {
                                            message: format!("Connection error: {}", e),
                                        })
                                        .await;

                                    // Clear connection state
                                    let mut state = conn_state_for_read.lock().await;
                                    state.remote_write_tx = None;
                                    state.connected_url = None;
                                    break;
                                }
                            }
                        }
                        write_task.abort();
                    });
                }
                Err(e) => {
                    log::error!("Failed to connect to WebSocket: {}", e);
                    let _ = to_client_tx
                        .send(WsServerMessage::Error {
                            message: format!("Connection failed: {}", e),
                        })
                        .await;
                }
            }
        }
        WsClientMessage::Disconnect => {
            log::info!("Disconnecting WebSocket");

            let mut state = conn_state.lock().await;
            state.remote_write_tx = None;
            state.connected_url = None;

            let _ = to_client_tx
                .send(WsServerMessage::Disconnected {
                    reason: "User disconnected".to_string(),
                })
                .await;
        }
        WsClientMessage::Send { message } => {
            log::debug!("Sending message to remote: {}", message);

            let state = conn_state.lock().await;
            if let Some(ref tx) = state.remote_write_tx {
                // Send to remote WebSocket
                if tx.send(message.clone()).await.is_ok() {
                    // Notify client that message was sent
                    let _ = to_client_tx
                        .send(WsServerMessage::Message {
                            data: message,
                            direction: "sent".to_string(),
                        })
                        .await;
                } else {
                    let _ = to_client_tx
                        .send(WsServerMessage::Error {
                            message: "Failed to send message".to_string(),
                        })
                        .await;
                }
            } else {
                let _ = to_client_tx
                    .send(WsServerMessage::Error {
                        message: "Not connected to a WebSocket server".to_string(),
                    })
                    .await;
            }
        }
    }
}

pub fn routes(pool: DbPool) -> Router {
    Router::new().route("/ws", get(ws_handler)).with_state(pool)
}
