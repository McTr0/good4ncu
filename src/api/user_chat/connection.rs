use axum::{extract::State, http::HeaderMap, Json};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token_with_fallback;
use crate::api::error::ApiError;
use crate::api::ws;
use crate::api::AppState;

use super::{
    ConnectAcceptBody, ConnectAcceptResponse, ConnectRejectBody, ConnectRejectResponse,
    ConnectRequestBody, ConnectRequestResponse, ConnectionEntry, ConnectionListResponse,
    WsConnectionEstablishedEvent, WsConnectionRejectedEvent, WsConnectionRequestEvent,
};

/// POST /api/chat/connect/request — initiate a connection request (step 1 of 3-way handshake).
///
/// Creates a pending connection record and pushes a `connection_request` WS event to the receiver.
pub async fn connect_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConnectRequestBody>,
) -> Result<Json<ConnectRequestResponse>, ApiError> {
    let requester_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    if requester_id == body.receiver_id {
        return Err(ApiError::BadRequest("不能向自己发起连接".to_string()));
    }

    let connection_id = state
        .chat_repo
        .upsert_connection_request(&requester_id, &body.receiver_id)
        .await?
        .connection_id;

    let requester_username: Option<String> =
        sqlx::query("SELECT username FROM users WHERE id = $1")
            .bind(&requester_id)
            .fetch_optional(&state.infra.db)
            .await
            .ok()
            .flatten()
            .map(|row| row.get("username"));

    let ws_event = WsConnectionRequestEvent {
        event: "connection_request".to_string(),
        connection_id: connection_id.clone(),
        requester_id: requester_id.clone(),
        requester_username,
        listing_id: body.listing_id,
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    ws::broadcast_to_user(&body.receiver_id, &payload);

    Ok(Json(ConnectRequestResponse {
        connection_id,
        status: "pending".to_string(),
    }))
}

/// POST /api/chat/connect/accept — accept a connection request (step 2 of handshake).
///
/// Updates status to 'connected' and pushes `connection_established` to both parties.
pub async fn connect_accept(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConnectAcceptBody>,
) -> Result<Json<ConnectAcceptResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    tracing::info!(user_id = %user_id, connection_id = %body.connection_id, "ACCEPT_CONNECTION");

    let result = state
        .chat_repo
        .accept_pending_connection(&body.connection_id, &user_id)
        .await?;
    let requester_id = result.requester_id;
    let receiver_id = result.receiver_id;
    let established_at = result
        .established_at
        .expect("accept_pending_connection always returns established_at");

    tracing::info!(receiver_id = %receiver_id, requester_id = %requester_id, "ACCEPT_CONNECTION row found");
    let established_at_str = established_at.to_rfc3339();

    let ws_event = WsConnectionEstablishedEvent {
        event: "connection_established".to_string(),
        connection_id: body.connection_id.clone(),
        established_at: established_at_str.clone(),
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    ws::broadcast_to_user(&requester_id, &payload);
    ws::broadcast_to_user(&receiver_id, &payload);

    Ok(Json(ConnectAcceptResponse {
        status: "connected".to_string(),
        established_at: established_at_str,
    }))
}

/// POST /api/chat/connect/reject — reject a connection request.
pub async fn connect_reject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConnectRejectBody>,
) -> Result<Json<ConnectRejectResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let requester_id = state
        .chat_repo
        .reject_pending_connection(&body.connection_id, &user_id)
        .await?
        .requester_id;

    // Notify the requester so they know the invitation was rejected.
    let ws_event = WsConnectionRejectedEvent {
        event: "connection_rejected".to_string(),
        connection_id: body.connection_id.clone(),
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    ws::broadcast_to_user(&requester_id, &payload);

    Ok(Json(ConnectRejectResponse {
        status: "rejected".to_string(),
    }))
}

/// GET /api/chat/connections — list all connections for the current user.
pub async fn list_connections(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ConnectionListResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let items: Vec<ConnectionEntry> = state
        .chat_repo
        .list_all_connections_for_user(&user_id)
        .await?
        .into_iter()
        .map(|summary| ConnectionEntry {
            id: summary.id,
            requester_id: summary.requester_id,
            other_user_id: summary.other_user_id,
            other_username: summary.other_username,
            status: summary.status,
            established_at: summary.established_at,
            created_at: summary.created_at,
            unread_count: summary.unread_count,
            is_receiver: summary.is_receiver,
        })
        .collect();

    Ok(Json(ConnectionListResponse { items }))
}
