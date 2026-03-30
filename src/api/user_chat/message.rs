use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token_with_fallback;
use crate::api::error::ApiError;
use crate::api::ws;
use crate::api::AppState;

use super::{
    EditMessageBody, EditMessageResponse, MarkReadResponse, MessageEntry, MessageListQuery,
    MessageListResponse, SendMessageBody, SendMessageResponse, TypingBody, WsMessageReadEvent,
    WsNewMessageEvent, WsTypingEvent,
};

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
        r#"SELECT id, sender, content, is_agent, timestamp, read_at, read_by, image_data, audio_data, image_url, audio_url, edited_at, status
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
                image_url: row.try_get("image_url").ok().flatten(),
                audio_url: row.try_get("audio_url").ok().flatten(),
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
        r#"INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, image_data, audio_data, image_url, audio_url, read_at, read_by, status)
           VALUES ($1::text, 'direct', $2, $3, false, $4, $5, $6, $7, $8, $9, $2, 'sent')
           RETURNING id, timestamp"#,
    )
    .bind(&connection_id)
    .bind(&sender_id)
    .bind(&receiver)
    .bind(&body.content)
    .bind(&body.image_base64)
    .bind(&body.audio_base64)
    .bind(&body.image_url)
    .bind(&body.audio_url)
    .bind(read_at)
    .fetch_one(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let message_id: i64 = row.get("id");
    let timestamp: chrono::DateTime<chrono::Utc> = row.get("timestamp");

    state.infra.metrics.record_chat_message();
    if body.image_url.is_some() || body.audio_url.is_some() {
        state.infra.metrics.record_chat_media_url_message();
    }
    if body.image_base64.is_some() || body.audio_base64.is_some() {
        state.infra.metrics.record_chat_media_base64_message();
    }

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
        image_url: body.image_url.clone(),
        audio_url: body.audio_url.clone(),
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
        image_url: body.image_url,
        audio_url: body.audio_url,
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
                     JOIN chat_connections cc
                         ON cc.id = CASE
                                                        WHEN cm.conversation_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                                                        THEN cm.conversation_id::uuid
                                                        ELSE NULL
                                                END
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
                     JOIN chat_connections cc
                         ON cc.id = CASE
                                                        WHEN cm.conversation_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                                                        THEN cm.conversation_id::uuid
                                                        ELSE NULL
                                                END
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
