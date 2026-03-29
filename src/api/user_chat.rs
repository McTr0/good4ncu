//! User-to-user direct chat with connection handshake.
//!
//! Implements a three-way handshake for establishing chat connections:
//! 1. Requester sends POST /api/chat/connect/request → status=pending
//! 2. Receiver accepts via POST /api/chat/connect/accept → status=connected
//!    (or rejects via POST /api/chat/connect/reject → status=rejected)
//! 3. Once connected, messages can be exchanged via POST /api/chat/conversations/{id}/messages
//!
//! WebSocket events pushed to participants:
//! - `connection_request` — new connection request received
//! - `connection_established` — connection accepted and established
//! - `new_message` — new direct message
//! - `message_read` — a message was marked as read

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token_with_fallback;
use crate::api::error::ApiError;
use crate::api::ws;
use crate::api::AppState;

// ---------------------------------------------------------------------------
// Schema initialization
// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ConnectRequestBody {
    pub receiver_id: String,
    pub listing_id: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectRequestResponse {
    pub connection_id: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct ConnectAcceptBody {
    pub connection_id: String,
}

#[derive(Serialize)]
pub struct ConnectAcceptResponse {
    pub status: String,
    pub established_at: String,
}

#[derive(Deserialize)]
pub struct ConnectRejectBody {
    pub connection_id: String,
}

#[derive(Serialize)]
pub struct ConnectRejectResponse {
    pub status: String,
}

#[derive(Serialize)]
pub struct ConnectionEntry {
    pub id: String,
    pub requester_id: String,
    pub other_user_id: String,
    pub other_username: Option<String>,
    pub status: String,
    pub established_at: Option<String>,
    pub created_at: String,
    /// 未读消息数
    pub unread_count: i32,
    /// Whether the current user is the receiver (can accept/reject this pending request)
    pub is_receiver: bool,
}

#[derive(Serialize)]
pub struct ConnectionListResponse {
    pub items: Vec<ConnectionEntry>,
}

#[derive(Deserialize)]
pub struct SendMessageBody {
    pub content: String,
    pub image_base64: Option<String>,
    pub audio_base64: Option<String>,
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    /// Returned as `id` for frontend compatibility with ConversationMessage.fromJson
    #[serde(rename = "id")]
    pub message_id: i64,
    pub sender: String,
    pub content: String,
    pub conversation_id: String,
    #[serde(rename = "timestamp")]
    pub sent_at: String,
    pub read_at: Option<String>,
    pub image_data: Option<String>,
    pub audio_data: Option<String>,
    /// 消息状态: sending | sent | delivered | read | failed
    pub status: String,
}

#[derive(Deserialize)]
pub struct MessageListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct MessageEntry {
    pub id: i64,
    pub sender: String,
    pub sender_username: Option<String>,
    pub content: String,
    pub is_agent: bool,
    pub timestamp: String,
    pub read_at: Option<String>,
    pub read_by: Option<String>,
    pub image_data: Option<String>,
    pub audio_data: Option<String>,
    /// 编辑状态: sending | sent | delivered | read | failed
    pub status: String,
    /// 已编辑时间
    pub edited_at: Option<String>,
}

#[derive(Serialize)]
pub struct MessageListResponse {
    pub conversation_id: String,
    pub messages: Vec<MessageEntry>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct MarkReadResponse {
    pub message_id: i64,
    pub read_at: String,
}

#[derive(Deserialize)]
pub struct EditMessageBody {
    pub content: String,
}

#[derive(Serialize)]
pub struct EditMessageResponse {
    pub message_id: i64,
    pub content: String,
    pub edited_at: String,
}

#[derive(Deserialize)]
pub struct TypingBody {
    pub conversation_id: String,
}

#[derive(Serialize)]
struct WsTypingEvent {
    event: String,
    conversation_id: String,
    user_id: String,
    username: Option<String>,
}

// ---------------------------------------------------------------------------
// WebSocket event helpers
// ---------------------------------------------------------------------------

/// WS event payloads sent to clients.
#[derive(Serialize)]
struct WsConnectionRequestEvent {
    event: String,
    connection_id: String,
    requester_id: String,
    requester_username: Option<String>,
    listing_id: Option<String>,
}

#[derive(Serialize)]
struct WsConnectionEstablishedEvent {
    event: String,
    connection_id: String,
    established_at: String,
}

#[derive(Serialize)]
struct WsConnectionRejectedEvent {
    event: String,
    connection_id: String,
}

#[derive(Serialize)]
struct WsNewMessageEvent {
    event: String,
    message_id: i64,
    conversation_id: String,
    sender: String,
    sender_username: Option<String>,
    content: String,
    timestamp: String,
    read_at: Option<String>,
    image_data: Option<String>,
    audio_data: Option<String>,
}

#[derive(Serialize)]
struct WsMessageReadEvent {
    event: String,
    message_id: i64,
    read_at: String,
    read_by: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

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

    let receiver_exists = sqlx::query("SELECT 1 FROM users WHERE id = $1")
        .bind(&body.receiver_id)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .is_some();
    if !receiver_exists {
        return Err(ApiError::NotFound);
    }

    let connection_id: String = {
        let row = sqlx::query(
            r#"INSERT INTO chat_connections (requester_id, receiver_id, status)
               VALUES ($1, $2, 'pending')
               ON CONFLICT (requester_id, receiver_id)
               DO UPDATE SET status = 'pending', established_at = NULL
               RETURNING id"#,
        )
        .bind(&requester_id)
        .bind(&body.receiver_id)
        .fetch_one(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        let uuid_val: uuid::Uuid = row
            .try_get("id")
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        uuid_val.to_string()
    };

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

    let row = sqlx::query(
        "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1::uuid",
    )
    .bind(&body.connection_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let requester_id: String = row.get("requester_id");
    let receiver_id: String = row.get("receiver_id");
    let current_status: String = row.get("status");

    tracing::info!(receiver_id = %receiver_id, requester_id = %requester_id, current_status = %current_status, "ACCEPT_CONNECTION row found");

    if receiver_id != user_id {
        tracing::warn!(user_id = %user_id, receiver_id = %receiver_id, "ACCEPT_CONNECTION forbidden - not receiver");
        return Err(ApiError::Forbidden);
    }
    if current_status != "pending" {
        tracing::warn!(connection_id = %body.connection_id, current_status = %current_status, "ACCEPT_CONNECTION bad request - not pending");
        return Err(ApiError::BadRequest(format!(
            "连接状态不是 pending，当前状态: {}",
            current_status
        )));
    }

    let established_at = chrono::Utc::now();
    let established_at_str = established_at.to_rfc3339();

    sqlx::query(
        "UPDATE chat_connections SET status = 'connected', established_at = $1 WHERE id = $2::uuid",
    )
    .bind(established_at)
    .bind(&body.connection_id)
    .execute(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

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

    let row = sqlx::query(
        "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1::uuid",
    )
    .bind(&body.connection_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let requester_id: String = row.get("requester_id");
    let receiver_id: String = row.get("receiver_id");
    let current_status: String = row.get("status");

    if receiver_id != user_id {
        return Err(ApiError::Forbidden);
    }
    if current_status != "pending" {
        return Err(ApiError::BadRequest(format!(
            "连接状态不是 pending，当前状态: {}",
            current_status
        )));
    }

    sqlx::query("UPDATE chat_connections SET status = 'rejected' WHERE id = $1::uuid")
        .bind(&body.connection_id)
        .execute(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

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

    let rows = sqlx::query(
        r#"SELECT
               cc.id,
               cc.status,
               cc.established_at,
               cc.created_at,
               cc.unread_count,
               cc.requester_id,
               (cc.receiver_id = $1) as is_receiver,
               CASE WHEN cc.requester_id = $1 THEN cc.receiver_id ELSE cc.requester_id END as other_user_id
           FROM chat_connections cc
           WHERE cc.requester_id = $1 OR cc.receiver_id = $1
           ORDER BY cc.created_at DESC"#,
    )
    .bind(&user_id)
    .fetch_all(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let other_ids: Vec<String> = rows
        .iter()
        .map(|row| row.get::<String, _>("other_user_id"))
        .collect();
    let usernames: std::collections::HashMap<String, String> = if other_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        sqlx::query("SELECT id, username FROM users WHERE id = ANY($1)")
            .bind(&other_ids)
            .fetch_all(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .into_iter()
            .map(|row| (row.get::<String, _>("id"), row.get::<String, _>("username")))
            .collect()
    };

    let items: Vec<ConnectionEntry> = rows
        .into_iter()
        .map(|row| {
            let other_user_id: String = row.get("other_user_id");
            let established_at: Option<chrono::DateTime<chrono::Utc>> =
                row.try_get("established_at").ok().flatten();
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let unread_count: i32 = row.try_get("unread_count").unwrap_or(0);
            ConnectionEntry {
                id: row
                    .try_get::<uuid::Uuid, _>("id")
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
                requester_id: row.get("requester_id"),
                other_user_id: other_user_id.clone(),
                other_username: usernames.get(&other_user_id).cloned(),
                status: row.get("status"),
                established_at: established_at.map(|dt| dt.to_rfc3339()),
                created_at: created_at.to_rfc3339(),
                unread_count,
                is_receiver: row.get("is_receiver"),
            }
        })
        .collect();

    Ok(Json(ConnectionListResponse { items }))
}

/// GET /api/chat/conversations/{connection_id}/messages — fetch messages in a connection.
pub async fn get_connection_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(connection_id): Path<String>,
    Query(params): Query<MessageListQuery>,
) -> Result<Json<MessageListResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);

    // 1. Check if this is a 'special' conversation ID (like AI agent or global chat)
    // 2. Otherwise, check if it's a valid UUID for a peer-to-peer connection
    let is_special = connection_id == "__agent__" || connection_id == "global";
    let connection_uuid = uuid::Uuid::parse_str(&connection_id).ok();

    if !is_special && connection_uuid.is_none() {
        return Err(ApiError::BadRequest(
            "Invalid conversation ID format".to_string(),
        ));
    }

    if let Some(uuid) = connection_uuid {
        let connection = sqlx::query(
            "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

        let requester_id: String = connection.get("requester_id");
        let receiver_id: String = connection.get("receiver_id");

        if requester_id != user_id && receiver_id != user_id {
            return Err(ApiError::Forbidden);
        }
    }

    let rows = sqlx::query(
        r#"SELECT id, sender, content, is_agent, timestamp, read_at, read_by, image_data, audio_data, edited_at, status
           FROM chat_messages
           WHERE conversation_id = $1::text
           ORDER BY id DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(&connection_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total: i64 = sqlx::query(
        r#"SELECT COUNT(*) as cnt FROM chat_messages
            WHERE conversation_id = $1::text"#,
    )
    .bind(&connection_id)
    .fetch_one(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .try_get("cnt")
    .unwrap_or(0);

    let sender_ids: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let sender: String = row.get("sender");
            if sender == "assistant" {
                None
            } else {
                Some(sender)
            }
        })
        .collect();
    let sender_usernames: std::collections::HashMap<String, String> = if sender_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        sqlx::query("SELECT id, username FROM users WHERE id = ANY($1)")
            .bind(&sender_ids)
            .fetch_all(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let username: String = row.get("username");
                (id, username)
            })
            .collect()
    };

    let messages: Vec<MessageEntry> = rows
        .into_iter()
        .map(|row| {
            let sender: String = row.get("sender");
            let timestamp: chrono::DateTime<chrono::Utc> = row.get("timestamp");
            let read_at: Option<chrono::DateTime<chrono::Utc>> =
                row.try_get("read_at").ok().flatten();
            let edited_at: Option<chrono::DateTime<chrono::Utc>> =
                row.try_get("edited_at").ok().flatten();
            let status: String = row.try_get("status").unwrap_or_else(|_| "sent".to_string());
            let sender_username = if sender == "assistant" {
                None
            } else {
                sender_usernames.get(&sender).cloned()
            };
            MessageEntry {
                id: row.get("id"),
                sender,
                sender_username,
                content: row.get("content"),
                is_agent: row.get("is_agent"),
                timestamp: timestamp.to_rfc3339(),
                read_at: read_at.map(|dt| dt.to_rfc3339()),
                read_by: row.try_get("read_by").ok().flatten(),
                image_data: row.try_get("image_data").ok().flatten(),
                audio_data: row.try_get("audio_data").ok().flatten(),
                edited_at: edited_at.map(|dt| dt.to_rfc3339()),
                status,
            }
        })
        .collect();

    Ok(Json(MessageListResponse {
        conversation_id: connection_id,
        messages,
        total,
    }))
}

/// POST /api/chat/conversations/{connection_id}/messages — send a message in a connection.
pub async fn send_connection_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(connection_id): Path<String>,
    Json(body): Json<SendMessageBody>,
) -> Result<Json<SendMessageResponse>, ApiError> {
    let sender_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    if body.content.is_empty() {
        return Err(ApiError::BadRequest("消息内容不能为空".to_string()));
    }
    if body.content.len() > 2000 {
        return Err(ApiError::BadRequest("消息内容不能超过2000字符".to_string()));
    }

    // Text content moderation — block prohibited content before persisting.
    let mod_result = state.infra.moderation.check_text(&body.content);
    if !mod_result.passed {
        return Err(ApiError::ContentViolation(
            mod_result.reason.unwrap_or_default(),
        ));
    }

    let connection_uuid = uuid::Uuid::parse_str(&connection_id).ok();
    let is_special = connection_id == "__agent__" || connection_id == "global";

    let (receiver_id_opt, status) = if let Some(uuid) = connection_uuid {
        let connection = sqlx::query(
            "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

        let req_id: String = connection.get("requester_id");
        let recv_id: String = connection.get("receiver_id");
        let status: String = connection.get("status");

        if sender_id != req_id && sender_id != recv_id {
            return Err(ApiError::Forbidden);
        }

        let receiver = if sender_id == req_id {
            Some(recv_id)
        } else {
            Some(req_id)
        };
        (receiver, status)
    } else if is_special {
        // Special conversations don't have a specific human receiver or connection record
        (None, "connected".to_string())
    } else {
        return Err(ApiError::BadRequest("Invalid conversation ID".to_string()));
    };

    if status != "connected" {
        return Err(ApiError::BadRequest(format!(
            "连接状态不是 connected，当前状态: {}",
            status
        )));
    }

    let receiver = receiver_id_opt;

    let read_at = Some(chrono::Utc::now());
    let read_at_str = read_at.map(|dt| dt.to_rfc3339());

    let row = sqlx::query(
        r#"INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, image_data, audio_data, read_at, read_by, status)
           VALUES ($1::text, 'direct', $2, $3, false, $4, $5, $6, $7, $2, 'sent')
           RETURNING id, timestamp"#,
    )
    .bind(&connection_id)
    .bind(&sender_id)
    .bind(&receiver)
    .bind(&body.content)
    .bind(&body.image_base64)
    .bind(&body.audio_base64)
    .bind(read_at)
    .fetch_one(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let message_id: i64 = row.get("id");
    let timestamp: chrono::DateTime<chrono::Utc> = row.get("timestamp");

    state.infra.metrics.record_chat_message();

    // Update unread_count for the receiver (if human connection exists)
    if let Some(uuid) = connection_uuid {
        sqlx::query("UPDATE chat_connections SET unread_count = unread_count + 1 WHERE id = $1")
            .bind(uuid)
            .execute(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    }

    let sender_username: Option<String> = sqlx::query("SELECT username FROM users WHERE id = $1")
        .bind(&sender_id)
        .fetch_optional(&state.infra.db)
        .await
        .ok()
        .flatten()
        .map(|row| row.get("username"));

    let ws_event = WsNewMessageEvent {
        event: "new_message".to_string(),
        message_id,
        conversation_id: connection_id.clone(),
        sender: sender_id.clone(),
        sender_username,
        content: body.content.clone(),
        timestamp: timestamp.to_rfc3339(),
        read_at: read_at_str.clone(),
        image_data: body.image_base64.clone(),
        audio_data: body.audio_base64.clone(),
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    if let Some(ref recv) = receiver {
        ws::broadcast_to_user(recv, &payload);
    }

    Ok(Json(SendMessageResponse {
        message_id,
        sender: sender_id,
        content: body.content,
        conversation_id: connection_id,
        sent_at: timestamp.to_rfc3339(),
        read_at: read_at_str,
        image_data: body.image_base64,
        audio_data: body.audio_base64,
        status: "sent".to_string(),
    }))
}

/// POST /api/chat/messages/{id}/read — mark a message as read.
pub async fn mark_message_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<i64>,
) -> Result<Json<MarkReadResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let row = sqlx::query(
        r#"SELECT cm.id, cm.sender, cm.receiver, cm.read_at, cm.conversation_id, cc.status as conn_status
           FROM chat_messages cm
           JOIN chat_connections cc ON cc.id::text = cm.conversation_id
           WHERE cm.id = $1"#,
    )
    .bind(message_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let sender: String = row.get("sender");
    let receiver: Option<String> = row.try_get("receiver").ok().flatten();
    let conversation_id: String = row.get("conversation_id");
    let current_read_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("read_at").ok().flatten();
    let conn_status: String = row.get("conn_status");

    if conn_status != "connected" {
        return Err(ApiError::BadRequest(format!(
            "连接状态不是 connected，无法标记已读: {}",
            conn_status
        )));
    }

    let Some(recv) = receiver else {
        return Err(ApiError::Forbidden);
    };
    if recv != user_id {
        return Err(ApiError::Forbidden);
    }

    if let Some(read_at) = current_read_at {
        let read_at_str = read_at.to_rfc3339();
        return Ok(Json(MarkReadResponse {
            message_id,
            read_at: read_at_str,
        }));
    }

    let read_at = chrono::Utc::now();
    let read_at_str = read_at.to_rfc3339();

    sqlx::query("UPDATE chat_messages SET read_at = $1, read_by = $2 WHERE id = $3")
        .bind(read_at)
        .bind(&user_id)
        .bind(message_id)
        .execute(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Reset unread_count for the receiver (if human connection exists)
    if let Ok(uuid) = uuid::Uuid::parse_str(&conversation_id) {
        sqlx::query("UPDATE chat_connections SET unread_count = 0 WHERE id = $1")
            .bind(uuid)
            .execute(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    }

    let ws_event = WsMessageReadEvent {
        event: "message_read".to_string(),
        message_id,
        read_at: read_at_str.clone(),
        read_by: user_id.clone(),
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    ws::broadcast_to_user(&sender, &payload);

    Ok(Json(MarkReadResponse {
        message_id,
        read_at: read_at_str,
    }))
}

/// PATCH /api/chat/messages/:id — edit a message within 15 minutes of sending.
pub async fn edit_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<i64>,
    Json(body): Json<EditMessageBody>,
) -> Result<Json<EditMessageResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    // Text content moderation — block prohibited content before any DB operation.
    let mod_result = state.infra.moderation.check_text(&body.content);
    if !mod_result.passed {
        return Err(ApiError::ContentViolation(
            mod_result.reason.unwrap_or_default(),
        ));
    }

    if body.content.is_empty() {
        return Err(ApiError::BadRequest("消息内容不能为空".to_string()));
    }
    if body.content.len() > 2000 {
        return Err(ApiError::BadRequest("消息内容不能超过2000字符".to_string()));
    }

    // Fetch the existing message
    let row = sqlx::query(
        r#"SELECT cm.id, cm.sender, cm.timestamp, cc.status as conn_status
           FROM chat_messages cm
           JOIN chat_connections cc ON cc.id::text = cm.conversation_id
           WHERE cm.id = $1"#,
    )
    .bind(message_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let sender: String = row.get("sender");
    if sender != user_id {
        return Err(ApiError::Forbidden);
    }

    let timestamp: chrono::DateTime<chrono::Utc> = row.get("timestamp");
    let conn_status: String = row.get("conn_status");

    if conn_status != "connected" {
        return Err(ApiError::BadRequest(format!(
            "连接状态不是 connected，无法编辑: {}",
            conn_status
        )));
    }

    // Only allow editing within 15 minutes
    let fifteen_minutes = chrono::Duration::minutes(15);
    let now = chrono::Utc::now();
    if now.signed_duration_since(timestamp) > fifteen_minutes {
        return Err(ApiError::BadRequest(
            "消息已超过15分钟，无法编辑".to_string(),
        ));
    }

    let edited_at = chrono::Utc::now();
    let edited_at_str = edited_at.to_rfc3339();

    sqlx::query("UPDATE chat_messages SET content = $1, edited_at = $2 WHERE id = $3")
        .bind(&body.content)
        .bind(edited_at)
        .bind(message_id)
        .execute(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(Json(EditMessageResponse {
        message_id,
        content: body.content,
        edited_at: edited_at_str,
    }))
}

/// POST /api/chat/typing — broadcast a typing indicator to the other party.
pub async fn typing_indicator(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TypingBody>,
) -> Result<(), ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let connection_uuid = uuid::Uuid::parse_str(&body.conversation_id).ok();
    let is_special = body.conversation_id == "__agent__" || body.conversation_id == "global";

    let (req_id, recv_id) = if let Some(uuid) = connection_uuid {
        let row = sqlx::query(
            "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

        let requester_id: String = row.get("requester_id");
        let receiver_id: String = row.get("receiver_id");

        if user_id != requester_id && user_id != receiver_id {
            return Err(ApiError::Forbidden);
        }
        (requester_id, receiver_id)
    } else if is_special {
        (user_id.clone(), "assistant".to_string())
    } else {
        return Err(ApiError::BadRequest("Invalid conversation ID".to_string()));
    };

    let requester_id = req_id;
    let receiver_id = recv_id;

    let username: Option<String> = sqlx::query("SELECT username FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&state.infra.db)
        .await
        .ok()
        .flatten()
        .map(|row| row.get("username"));

    let ws_event = WsTypingEvent {
        event: "typing".to_string(),
        conversation_id: body.conversation_id.clone(),
        user_id: user_id.clone(),
        username,
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();

    // Broadcast to the other party
    let recipient = if user_id == requester_id {
        receiver_id
    } else {
        requester_id
    };
    ws::broadcast_to_user(&recipient, &payload);

    Ok(())
}

/// POST /api/chat/connection/{id}/read — batch mark all messages in a connection as read.
///
/// Replaces the N+1 pattern where the client fetches 50 messages then marks each one read
/// individually. One SQL UPDATE + one unread_count reset.
pub async fn mark_connection_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(connection_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let connection_uuid = uuid::Uuid::parse_str(&connection_id).ok();
    let is_special = connection_id == "__agent__" || connection_id == "global";

    if let Some(uuid) = connection_uuid {
        let row = sqlx::query(
            "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

        let requester_id: String = row.get("requester_id");
        let receiver_id: String = row.get("receiver_id");

        if user_id != requester_id && user_id != receiver_id {
            return Err(ApiError::Forbidden);
        }
    } else if !is_special {
        return Err(ApiError::BadRequest("Invalid conversation ID".to_string()));
    }

    let now = chrono::Utc::now();

    // Batch UPDATE: mark all unread messages sent TO this user as read
    let result = sqlx::query(
        r#"UPDATE chat_messages
           SET read_at = $1, read_by = $2, status = 'read'
           WHERE conversation_id = $3::text
             AND receiver = $2
             AND read_at IS NULL"#,
    )
    .bind(now)
    .bind(&user_id)
    .bind(&connection_id)
    .execute(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Reset unread_count (if human connection exists)
    if let Some(uuid) = connection_uuid {
        sqlx::query("UPDATE chat_connections SET unread_count = 0 WHERE id = $1")
            .bind(uuid)
            .execute(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    }

    Ok(Json(serde_json::json!({
        "marked_count": result.rows_affected(),
        "read_at": now.to_rfc3339()
    })))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Request/Response Serialization Tests
    // ========================================================================

    #[test]
    fn test_connection_entry_serialization() {
        let entry = ConnectionEntry {
            id: "conn-1".to_string(),
            requester_id: "user-1".to_string(),
            other_user_id: "user-2".to_string(),
            other_username: Some("alice".to_string()),
            status: "connected".to_string(),
            established_at: Some("2024-01-01T00:00:00Z".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            unread_count: 3,
            is_receiver: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("conn-1"));
        assert!(json.contains("connected"));
        assert!(json.contains("alice"));
        assert!(json.contains("\"unread_count\":3"));
    }

    #[test]
    fn test_connection_entry_json_structure() {
        // ConnectionEntry only implements Serialize, not Deserialize.
        // Verify the JSON output has correct structure by checking key presence.
        let entry = ConnectionEntry {
            id: "conn-123".to_string(),
            requester_id: "user-a".to_string(),
            other_user_id: "user-b".to_string(),
            other_username: Some("bob".to_string()),
            status: "pending".to_string(),
            established_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            unread_count: 0,
            is_receiver: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        // Verify JSON structure contains expected keys
        assert!(json.contains(r#""id":"conn-123""#));
        assert!(json.contains(r#""requester_id":"user-a""#));
        assert!(json.contains(r#""other_user_id":"user-b""#));
        assert!(json.contains(r#""other_username":"bob""#));
        assert!(json.contains(r#""status":"pending""#));
        assert!(json.contains(r#""established_at":null"#));
        assert!(json.contains(r#""created_at":"2024-01-01T00:00:00Z""#));
        assert!(json.contains(r#""unread_count":0"#));
        assert!(json.contains(r#""is_receiver":true"#));
    }

    #[test]
    fn test_connection_entry_without_username() {
        let entry = ConnectionEntry {
            id: "conn-1".to_string(),
            requester_id: "user-1".to_string(),
            other_user_id: "user-2".to_string(),
            other_username: None,
            status: "pending".to_string(),
            established_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            unread_count: 5,
            is_receiver: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        // other_username should be null in JSON, not omitted
        assert!(json.contains("\"other_username\":null"));
    }

    #[test]
    fn test_message_entry_serialization() {
        let entry = MessageEntry {
            id: 1,
            sender: "user-1".to_string(),
            sender_username: Some("alice".to_string()),
            content: "Hello!".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: None,
            audio_data: None,
            status: "sent".to_string(),
            edited_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Hello!"));
        assert!(json.contains("user-1"));
        assert!(json.contains("\"is_agent\":false"));
        assert!(json.contains("\"status\":\"sent\""));
    }

    #[test]
    fn test_message_entry_json_structure() {
        // MessageEntry only implements Serialize, not Deserialize.
        // Verify the JSON output has correct structure by serializing and checking keys.
        let entry = MessageEntry {
            id: 42,
            sender: "user-1".to_string(),
            sender_username: Some("alice".to_string()),
            content: "Test message".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: None,
            audio_data: None,
            status: "sent".to_string(),
            edited_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        // Verify JSON structure
        assert!(json.contains(r#""id":42"#));
        assert!(json.contains(r#""sender":"user-1""#));
        assert!(json.contains(r#""sender_username":"alice""#));
        assert!(json.contains(r#""content":"Test message""#));
        assert!(json.contains(r#""is_agent":false"#));
        assert!(json.contains(r#""status":"sent""#));
    }

    #[test]
    fn test_message_entry_with_read_status_json() {
        // Verify message with read status serializes correctly
        let entry = MessageEntry {
            id: 100,
            sender: "user-2".to_string(),
            sender_username: Some("bob".to_string()),
            content: "Read message".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: Some("2024-01-01T00:01:00Z".to_string()),
            read_by: Some("user-1".to_string()),
            image_data: None,
            audio_data: None,
            status: "read".to_string(),
            edited_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""id":100"#));
        assert!(json.contains(r#""read_at":"2024-01-01T00:01:00Z""#));
        assert!(json.contains(r#""read_by":"user-1""#));
        assert!(json.contains(r#""status":"read""#));
    }

    #[test]
    fn test_message_entry_agent_message() {
        let entry = MessageEntry {
            id: 1,
            sender: "assistant".to_string(),
            sender_username: None,
            content: "AI response".to_string(),
            is_agent: true,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: None,
            audio_data: None,
            status: "delivered".to_string(),
            edited_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"is_agent\":true"));
        assert!(json.contains("assistant"));
    }

    #[test]
    fn test_message_entry_with_image_data() {
        let entry = MessageEntry {
            id: 1,
            sender: "user-1".to_string(),
            sender_username: Some("alice".to_string()),
            content: "Check this out!".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: Some("data:image/png;base64,abc123".to_string()),
            audio_data: None,
            status: "sent".to_string(),
            edited_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("data:image/png;base64,abc123"));
    }

    #[test]
    fn test_message_entry_with_audio_data() {
        let entry = MessageEntry {
            id: 1,
            sender: "user-1".to_string(),
            sender_username: Some("alice".to_string()),
            content: "Voice message".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: None,
            audio_data: Some("data:audio/webm;base64,xyz789".to_string()),
            status: "sent".to_string(),
            edited_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("data:audio/webm;base64,xyz789"));
    }

    #[test]
    fn test_connect_request_response() {
        let resp = ConnectRequestResponse {
            connection_id: "conn-123".to_string(),
            status: "pending".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("conn-123"));
        assert!(json.contains("pending"));
    }

    #[test]
    fn test_connect_request_response_json_structure() {
        // ConnectRequestResponse only implements Serialize, not Deserialize.
        // Verify the JSON output has correct structure.
        let resp = ConnectRequestResponse {
            connection_id: "conn-abc".to_string(),
            status: "pending".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""connection_id":"conn-abc""#));
        assert!(json.contains(r#""status":"pending""#));
    }

    #[test]
    fn test_connect_accept_response() {
        let resp = ConnectAcceptResponse {
            status: "connected".to_string(),
            established_at: "2024-01-01T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("2024-01-01T12:00:00Z"));
    }

    #[test]
    fn test_connect_reject_response() {
        let resp = ConnectRejectResponse {
            status: "rejected".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("rejected"));
    }

    #[test]
    fn test_connect_request_body_deserialization() {
        let json = r#"{"receiver_id": "user-123", "listing_id": "listing-456"}"#;
        let body: ConnectRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.receiver_id, "user-123");
        assert_eq!(body.listing_id, Some("listing-456".to_string()));
    }

    #[test]
    fn test_connect_request_body_without_listing() {
        let json = r#"{"receiver_id": "user-123"}"#;
        let body: ConnectRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.receiver_id, "user-123");
        assert!(body.listing_id.is_none());
    }

    #[test]
    fn test_connect_accept_body_deserialization() {
        let json = r#"{"connection_id": "conn-xyz"}"#;
        let body: ConnectAcceptBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.connection_id, "conn-xyz");
    }

    #[test]
    fn test_connect_reject_body_deserialization() {
        let json = r#"{"connection_id": "conn-xyz"}"#;
        let body: ConnectRejectBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.connection_id, "conn-xyz");
    }

    #[test]
    fn test_send_message_body_deserialization() {
        let json = r#"{"content": "Hello!", "image_base64": null, "audio_base64": null}"#;
        let body: SendMessageBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.content, "Hello!");
        assert!(body.image_base64.is_none());
        assert!(body.audio_base64.is_none());
    }

    #[test]
    fn test_send_message_body_with_media() {
        let json = r#"{
            "content": "Image message",
            "image_base64": "base64data",
            "audio_base64": null
        }"#;
        let body: SendMessageBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.content, "Image message");
        assert_eq!(body.image_base64, Some("base64data".to_string()));
    }

    #[test]
    fn test_send_message_response() {
        let resp = SendMessageResponse {
            message_id: 42,
            sender: "user-1".to_string(),
            content: "hello".to_string(),
            conversation_id: "conv-1".to_string(),
            sent_at: "2024-01-01T00:00:00Z".to_string(),
            read_at: Some("2024-01-01T00:00:01Z".to_string()),
            image_data: None,
            audio_data: None,
            status: "sent".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("42"));
        assert!(json.contains("2024-01-01T00:00:00Z"));
        assert!(json.contains("\"status\":\"sent\""));
    }

    #[test]
    fn test_send_message_response_id_field_renamed() {
        // SendMessageResponse uses #[serde(rename = "id")] for message_id
        let resp = SendMessageResponse {
            message_id: 99,
            sender: "user-1".to_string(),
            content: "test".to_string(),
            conversation_id: "conv-1".to_string(),
            sent_at: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            image_data: None,
            audio_data: None,
            status: "sending".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        // The message_id field should be serialized as "id" due to #[serde(rename = "id")]
        assert!(json.contains("\"id\":99"));
    }

    #[test]
    fn test_send_message_response_timestamp_field_renamed() {
        // SendMessageResponse uses #[serde(rename = "timestamp")] for sent_at
        let resp = SendMessageResponse {
            message_id: 1,
            sender: "user-1".to_string(),
            content: "test".to_string(),
            conversation_id: "conv-1".to_string(),
            sent_at: "2024-01-01T12:34:56Z".to_string(),
            read_at: None,
            image_data: None,
            audio_data: None,
            status: "sent".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        // The sent_at field should be serialized as "timestamp"
        assert!(json.contains("\"timestamp\":\"2024-01-01T12:34:56Z\""));
    }

    #[test]
    fn test_mark_read_response() {
        let resp = MarkReadResponse {
            message_id: 10,
            read_at: "2024-01-01T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("10"));
        assert!(json.contains("2024-01-01T12:00:00Z"));
    }

    #[test]
    fn test_edit_message_body_deserialization() {
        let json = r#"{"content": "Updated message"}"#;
        let body: EditMessageBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.content, "Updated message");
    }

    #[test]
    fn test_edit_message_response_serialization() {
        let resp = EditMessageResponse {
            message_id: 42,
            content: "Updated".to_string(),
            edited_at: "2024-01-01T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("42"));
        assert!(json.contains("Updated"));
        assert!(json.contains("2024-01-01T12:00:00Z"));
    }

    #[test]
    fn test_typing_body_deserialization() {
        let json = r#"{"conversation_id": "conv-123"}"#;
        let body: TypingBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.conversation_id, "conv-123");
    }

    #[test]
    fn test_message_list_query_defaults() {
        let query = MessageListQuery {
            limit: None,
            offset: None,
        };
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_message_list_query_with_pagination() {
        let query = MessageListQuery {
            limit: Some(100),
            offset: Some(50),
        };
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(50));
    }

    #[test]
    fn test_connection_list_response() {
        let resp = ConnectionListResponse { items: vec![] };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"items\":[]"));
    }

    #[test]
    fn test_connection_list_response_with_items() {
        let entry = ConnectionEntry {
            id: "conn-1".to_string(),
            requester_id: "user-1".to_string(),
            other_user_id: "user-2".to_string(),
            other_username: Some("alice".to_string()),
            status: "connected".to_string(),
            established_at: Some("2024-01-01T00:00:00Z".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            unread_count: 3,
            is_receiver: false,
        };
        let resp = ConnectionListResponse { items: vec![entry] };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("conn-1"));
        assert!(json.contains("\"unread_count\":3"));
    }

    #[test]
    fn test_message_list_response() {
        let resp = MessageListResponse {
            conversation_id: "conn-1".to_string(),
            messages: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("conn-1"));
        assert!(json.contains("\"messages\":[]"));
    }

    #[test]
    fn test_message_list_response_with_total() {
        let resp = MessageListResponse {
            conversation_id: "conn-1".to_string(),
            messages: vec![],
            total: 100,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total\":100"));
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_empty_content_in_send_message_body() {
        // Empty content should be valid for deserialization
        // (validation happens in the handler, not at deserialization)
        let json = r#"{"content": "", "image_base64": null, "audio_base64": null}"#;
        let body: SendMessageBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.content, "");
    }

    #[test]
    fn test_unicode_content_in_message() {
        let entry = MessageEntry {
            id: 1,
            sender: "user-1".to_string(),
            sender_username: Some("中文用户".to_string()),
            content: "你好世界！".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: None,
            audio_data: None,
            status: "sent".to_string(),
            edited_at: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("你好世界！"));
        assert!(json.contains("中文用户"));
    }

    #[test]
    fn test_emoji_in_message_content() {
        let entry = MessageEntry {
            id: 1,
            sender: "user-1".to_string(),
            sender_username: None,
            content: "Hello 👋🎉".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: None,
            audio_data: None,
            status: "sent".to_string(),
            edited_at: None,
        };
        // MessageEntry only implements Serialize, not Deserialize.
        // Verify serialization preserves emoji content.
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Hello 👋🎉"));
        assert!(json.contains(r#""content":"Hello 👋🎉""#));
    }

    #[test]
    fn test_long_content_in_message() {
        let long_content = "a".repeat(2000);
        let entry = MessageEntry {
            id: 1,
            sender: "user-1".to_string(),
            sender_username: None,
            content: long_content.clone(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            read_at: None,
            read_by: None,
            image_data: None,
            audio_data: None,
            status: "sent".to_string(),
            edited_at: None,
        };
        // MessageEntry only implements Serialize, not Deserialize.
        // Verify serialization handles long content.
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.len() > 2000);
        // The content should appear in the JSON
        assert!(json.contains(&"a".repeat(100)));
    }

    #[test]
    fn test_special_characters_in_connection_id() {
        // UUIDs should be serializable
        let entry = ConnectionEntry {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            requester_id: "user-1".to_string(),
            other_user_id: "user-2".to_string(),
            other_username: None,
            status: "connected".to_string(),
            established_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            unread_count: 0,
            is_receiver: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
    }
}
