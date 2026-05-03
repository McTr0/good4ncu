use sqlx::{PgPool, Row};

use crate::api::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ConversationAccess {
    Special,
    Direct(DirectConversationAccess),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DirectConversationAccess {
    pub connection_uuid: uuid::Uuid,
    pub requester_id: String,
    pub receiver_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DirectMessageAccess {
    pub sender: String,
    pub receiver: Option<String>,
    pub conversation_id: String,
    pub connection_status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl DirectConversationAccess {
    pub fn other_user_id(&self, user_id: &str) -> Result<String, ApiError> {
        if user_id == self.requester_id {
            Ok(self.receiver_id.clone())
        } else if user_id == self.receiver_id {
            Ok(self.requester_id.clone())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn ensure_connected(&self) -> Result<(), ApiError> {
        if self.status == "connected" {
            Ok(())
        } else {
            Err(ApiError::BadRequest(format!(
                "连接状态不是 connected，当前状态: {}",
                self.status
            )))
        }
    }
}

pub(super) async fn load_conversation_access(
    pool: &PgPool,
    conversation_id: &str,
    user_id: &str,
) -> Result<ConversationAccess, ApiError> {
    if is_special_conversation(conversation_id) {
        return Ok(ConversationAccess::Special);
    }

    let connection_uuid = uuid::Uuid::parse_str(conversation_id)
        .map_err(|_| ApiError::BadRequest("Invalid conversation ID".to_string()))?;

    let row =
        sqlx::query("SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1")
            .bind(connection_uuid)
            .fetch_optional(pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .ok_or(ApiError::NotFound)?;

    let requester_id: String = row.get("requester_id");
    let receiver_id: String = row.get("receiver_id");
    let status: String = row.get("status");

    if user_id != requester_id && user_id != receiver_id {
        return Err(ApiError::Forbidden);
    }

    Ok(ConversationAccess::Direct(DirectConversationAccess {
        connection_uuid,
        requester_id,
        receiver_id,
        status,
    }))
}

pub(super) async fn load_direct_message_access(
    pool: &PgPool,
    message_id: i64,
) -> Result<DirectMessageAccess, ApiError> {
    let row = sqlx::query(
        r#"SELECT cm.sender, cm.receiver, cm.read_at, cm.conversation_id, cm.timestamp, cc.status as conn_status
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
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    Ok(DirectMessageAccess {
        sender: row.get("sender"),
        receiver: row.try_get("receiver").ok().flatten(),
        conversation_id: row.get("conversation_id"),
        connection_status: row.get("conn_status"),
        timestamp: row.get("timestamp"),
        read_at: row.try_get("read_at").ok().flatten(),
    })
}

pub(super) fn is_special_conversation(conversation_id: &str) -> bool {
    matches!(conversation_id, "__agent__" | "global")
}
