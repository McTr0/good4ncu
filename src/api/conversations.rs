use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token;
use crate::api::error::ApiError;
use crate::api::AppState;
use crate::services::chat::{ChatService, ConversationSummary};

#[derive(Deserialize)]
pub struct ConversationMessagesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ConversationMessagesResponse {
    pub conversation_id: String,
    pub messages: Vec<MessageEntry>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct MessageEntry {
    pub sender: String,
    pub content: String,
    pub is_agent: bool,
    pub timestamp: String,
}

/// GET /api/conversations - list user's conversations
pub async fn list_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ConversationSummary>>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let chat_svc = ChatService::new(state.db.clone());
    let conversations = chat_svc
        .list_conversations(&user_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(Json(conversations))
}

/// GET /api/conversations/:id/messages - get messages in a conversation
pub async fn get_conversation_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Query(params): Query<ConversationMessagesQuery>,
) -> Result<Json<ConversationMessagesResponse>, ApiError> {
    let _user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);

    let rows = sqlx::query(
        r#"
        SELECT sender, content, is_agent, timestamp
        FROM chat_messages
        WHERE conversation_id = $1
        ORDER BY id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&conversation_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total: i64 =
        sqlx::query("SELECT COUNT(*) as cnt FROM chat_messages WHERE conversation_id = $1")
            .bind(&conversation_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .try_get("cnt")
            .unwrap_or(0);

    let messages: Vec<MessageEntry> = rows
        .iter()
        .map(|row| {
            let timestamp: String = row
                .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("timestamp")
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            MessageEntry {
                sender: row.get("sender"),
                content: row.get("content"),
                is_agent: row.get("is_agent"),
                timestamp,
            }
        })
        .collect();

    Ok(Json(ConversationMessagesResponse {
        conversation_id,
        messages,
        total,
    }))
}
