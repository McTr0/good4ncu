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
pub struct ConversationListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ConversationListResponse {
    pub items: Vec<ConversationSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

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

/// GET /api/conversations - list user's conversations (paginated)
pub async fn list_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ConversationListQuery>,
) -> Result<Json<ConversationListResponse>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    let chat_svc = ChatService::new(state.db.clone());
    let (conversations, total) = chat_svc
        .list_conversations(&user_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(Json(ConversationListResponse {
        items: conversations,
        total,
        limit,
        offset,
    }))
}

/// GET /api/conversations/:id/messages - get messages in a conversation
pub async fn get_conversation_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Query(params): Query<ConversationMessagesQuery>,
) -> Result<Json<ConversationMessagesResponse>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);

    // Verify user has access to this conversation (IDOR fix)
    let has_access = sqlx::query(
        "SELECT 1 FROM chat_messages WHERE conversation_id = $1 AND sender = $2 LIMIT 1",
    )
    .bind(&conversation_id)
    .bind(&user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if has_access.is_none() {
        return Err(ApiError::Forbidden);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_list_query_defaults() {
        let query: ConversationListQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn test_conversation_list_query_with_pagination() {
        let query: ConversationListQuery =
            serde_json::from_str(r#"{"limit": 10, "offset": 5}"#).unwrap();
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(5));
    }

    #[test]
    fn test_conversation_messages_query_defaults() {
        let query: ConversationMessagesQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn test_conversation_messages_query_with_pagination() {
        let query: ConversationMessagesQuery =
            serde_json::from_str(r#"{"limit": 50, "offset": 100}"#).unwrap();
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(100));
    }

    #[test]
    fn test_message_entry_serialization() {
        let entry = MessageEntry {
            sender: "user-123".to_string(),
            content: "Hello!".to_string(),
            is_agent: false,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("Hello!"));
        assert!(json.contains("\"is_agent\":false"));
    }

    #[test]
    fn test_conversation_messages_response_serialization() {
        let response = ConversationMessagesResponse {
            conversation_id: "conv-1".to_string(),
            messages: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("conv-1"));
        assert!(json.contains("\"messages\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_conversation_list_response_serialization() {
        let response = ConversationListResponse {
            items: vec![],
            total: 0,
            limit: 20,
            offset: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"items\":[]"));
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"limit\":20"));
        assert!(json.contains("\"offset\":0"));
    }
}
