//! WebSocket real-time notification push.
//!
//! Clients connect to GET /ws?token=<jwt>. The JWT is validated server-side
//! to associate the WebSocket connection with a user_id. When a notification is
//! created via NotificationService.create(), it is immediately pushed to all
//! connected clients for that user via the global WS_CONNECTIONS map.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use axum::extract::ws::Message as WsMsg;
use axum::http::StatusCode;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

use crate::api::auth::extract_user_id_from_token_str;
use crate::api::AppState;

/// Connection table: user_id → tx channel to that user's socket.
/// DashMap handles concurrent access from multiple tokio tasks.
pub type WsConnections = DashMap<String, mpsc::Sender<Message>>;

/// Global WebSocket connections registry.
/// Uses LazyLock so it is initialized on first access (at startup, not lazily per request).
static WS_CONNECTIONS: std::sync::LazyLock<Arc<WsConnections>> =
    std::sync::LazyLock::new(|| Arc::new(DashMap::new()));

pub fn new_ws_state() -> Arc<WsConnections> {
    Arc::clone(&WS_CONNECTIONS)
}

/// Global broadcast — pushes a JSON payload to a specific user if they are currently connected.
pub fn broadcast_to_user(user_id: &str, payload: &str) {
    if let Some(tx) = WS_CONNECTIONS.get(user_id) {
        let _ = tx.try_send(Message::Text(payload.into()));
    }
}

// ---------------------------------------------------------------------------
// Axum handler
// ---------------------------------------------------------------------------

/// GET /ws?token=<jwt> — WebSocket upgrade endpoint.
/// Authentication is done via JWT in the query parameter (not headers, since browsers
/// do not send custom headers during WebSocket handshake).
pub async fn ws_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Query(params): axum::extract::Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Validate JWT from query param; reject with 401 if invalid.
    let token = params.token.as_deref().unwrap_or("");
    let user_id = match extract_user_id_from_token_str(token, &state.jwt_secret) {
        Ok(uid) => uid,
        Err(_) => {
            return (StatusCode::UNAUTHORIZED, axum::response::Json(serde_json::json!({
                "error": "Invalid or missing token"
            })))
            .into_response();
        }
    };

    ws.on_upgrade(move |socket| async move {
        handle_socket(socket, user_id).await;
    })
}

#[derive(serde::Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

/// Handle a single WebSocket connection for its lifetime.
///
/// Uses `futures_util::StreamExt::split` to divide the WebSocket into independent
/// send and receive halves that run concurrently in separate spawned tasks:
///
/// - Send task: pulls from `rx` mpsc channel and sends over the wire.
/// - Recv task: receives from the wire and discards (we only push, no echo).
/// - A oneshot channel signals the sender when the receiver closes.
/// - When the recv task exits, it drops `tx` (via WS_CONNECTIONS removal) to signal the sender.
async fn handle_socket(socket: WebSocket, user_id: String) {
    // Split the socket into independent send/receive halves.
    // This consumes `socket` and returns two handles that can be moved into separate tasks.
    let (ws_sender, mut ws_receiver) = socket.split();

    // Create a channel for this connection — buffer up to 64 pending messages.
    let (tx, rx) = mpsc::channel::<Message>(64);

    // Register this user's tx so broadcast_to_user can find them.
    WS_CONNECTIONS.insert(user_id.clone(), tx);

    // Signal sender when recv task exits.
    let (close_tx, close_rx) = oneshot::channel::<()>();

    // Spawn sender task: drives ws_sender until rx closes or close_rx fires.
    tokio::spawn(async move {
        let mut ws_sender = ws_sender;
        let mut rx = rx;
        let mut close_rx = close_rx;
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if ws_sender.send(msg).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = &mut close_rx => {
                    // Receiver closed — graceful shutdown.
                    let _ = ws_sender.close().await;
                    break;
                }
            }
        }
    });

    // Recv task (main): drives ws_receiver, discards all messages.
    // When the socket closes (None), drop tx and signal sender to exit.
    loop {
        match ws_receiver.next().await {
            Some(Ok(WsMsg::Close(_))) | None => break,
            Some(Ok(_)) => {} // Ignore client→server messages.
            Some(Err(e)) => {
                tracing::warn!(%e, "WS receive error");
                break;
            }
        }
    }

    // Socket closed. Clean up: remove from registry, signal sender.
    WS_CONNECTIONS.remove(&user_id);
    let _ = close_tx.send(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_connections_type_compiles() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WsConnections>();
        assert_send_sync::<Arc<WsConnections>>();
    }

    #[test]
    fn test_ws_query_deserialize() {
        let json = r#"{"token": "eyJhbGciOiJIUzI1NiJ9.test"}"#;
        let query: WsQuery = serde_json::from_str(json).unwrap();
        assert!(query.token.is_some());
    }

    #[test]
    fn test_ws_query_missing_token() {
        let json = r#"{}"#;
        let query: WsQuery = serde_json::from_str(json).unwrap();
        assert!(query.token.is_none());
    }
}
