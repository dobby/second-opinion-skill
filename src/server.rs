use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

pub enum RequestResult {
    Response(String),
    Error(String),
}

struct ServerState {
    extension_connected: bool,
    extension_tx: Option<mpsc::Sender<String>>,
    pending: HashMap<String, oneshot::Sender<RequestResult>>,
}

impl ServerState {
    fn new() -> Self {
        ServerState {
            extension_connected: false,
            extension_tx: None,
            pending: HashMap::new(),
        }
    }
}

pub async fn run_server(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    let state = Arc::new(Mutex::new(ServerState::new()));

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(handle_connection(stream, peer_addr, state));
    }
}

async fn handle_connection(
    stream: TcpStream,
    _peer_addr: SocketAddr,
    state: Arc<Mutex<ServerState>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Channel for sending messages to this connection's write half
    let (conn_tx, mut conn_rx) = mpsc::channel::<String>(32);

    // Spawn a task to forward messages from channel to ws writer
    tokio::spawn(async move {
        while let Some(msg) = conn_rx.recv().await {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let mut is_extension = false;

    while let Some(msg_result) = ws_receiver.next().await {
        let msg = match msg_result {
            Ok(m) => m,
            Err(_) => break,
        };

        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            Message::Ping(data) => {
                // pong is handled automatically by tungstenite in older versions;
                // here we just continue
                let _ = data;
                continue;
            }
            _ => continue,
        };

        let parsed: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = match parsed.get("type").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => continue,
        };

        match msg_type.as_str() {
            "extension_ready" => {
                is_extension = true;
                let mut st = state.lock().await;
                st.extension_connected = true;
                st.extension_tx = Some(conn_tx.clone());
            }

            "response" => {
                let id = match parsed.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let text_val = parsed
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let mut st = state.lock().await;
                if let Some(sender) = st.pending.remove(&id) {
                    let _ = sender.send(RequestResult::Response(text_val));
                }
            }

            "error" => {
                let id = match parsed.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let error_val = parsed
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let mut st = state.lock().await;
                if let Some(sender) = st.pending.remove(&id) {
                    let _ = sender.send(RequestResult::Error(error_val));
                }
            }

            "ask" => {
                let id = match parsed.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let message = parsed
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let (resp_tx, resp_rx) = oneshot::channel::<RequestResult>();

                let ext_tx = {
                    let mut st = state.lock().await;
                    if !st.extension_connected {
                        // Respond immediately with error
                        let error_msg = json!({
                            "type": "error",
                            "id": id,
                            "error": "extension_not_connected"
                        })
                        .to_string();
                        let _ = conn_tx.send(error_msg).await;
                        continue;
                    }
                    st.pending.insert(id.clone(), resp_tx);
                    st.extension_tx.clone()
                };

                // Relay the ask to the extension
                if let Some(tx) = ext_tx {
                    let relay_msg = json!({
                        "type": "ask",
                        "id": id.clone(),
                        "message": message
                    })
                    .to_string();
                    if tx.send(relay_msg).await.is_err() {
                        // Extension disconnected
                        let mut st = state.lock().await;
                        st.extension_connected = false;
                        st.extension_tx = None;
                        if let Some(sender) = st.pending.remove(&id) {
                            let _ = sender.send(RequestResult::Error(
                                "extension_not_connected".to_string(),
                            ));
                        }
                    }
                }

                // Wait for response and forward to CLI client
                let conn_tx_clone = conn_tx.clone();
                let id_clone = id.clone();
                tokio::spawn(async move {
                    match resp_rx.await {
                        Ok(RequestResult::Response(text)) => {
                            let resp = json!({
                                "type": "response",
                                "id": id_clone,
                                "text": text
                            })
                            .to_string();
                            let _ = conn_tx_clone.send(resp).await;
                        }
                        Ok(RequestResult::Error(err)) => {
                            let resp = json!({
                                "type": "error",
                                "id": id_clone,
                                "error": err
                            })
                            .to_string();
                            let _ = conn_tx_clone.send(resp).await;
                        }
                        Err(_) => {
                            // Sender dropped
                        }
                    }
                });
            }

            "status_query" => {
                let ext_connected = {
                    let st = state.lock().await;
                    st.extension_connected
                };
                let resp = json!({
                    "type": "status_response",
                    "extension_connected": ext_connected
                })
                .to_string();
                let _ = conn_tx.send(resp).await;
            }

            _ => {}
        }
    }

    // Cleanup on disconnect
    if is_extension {
        let mut st = state.lock().await;
        st.extension_connected = false;
        st.extension_tx = None;
        // Fail all pending requests
        let pending = std::mem::take(&mut st.pending);
        for (_id, sender) in pending {
            let _ = sender.send(RequestResult::Error(
                "extension_not_connected".to_string(),
            ));
        }
    }
}
