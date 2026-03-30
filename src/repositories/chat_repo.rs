//! PostgreSQL implementation of the ChatRepository trait.

use crate::api::error::ApiError;
use crate::repositories::{ChatMessage, ChatRepository, ConversationSummary};
use crate::services::chat::ChatHistoryEntry;
use chrono::Utc;
use sqlx::{PgPool, Row};

#[derive(Clone)]
#[allow(dead_code)]
pub struct PostgresChatRepository {
    pool: PgPool,
}

impl PostgresChatRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ChatRepository for PostgresChatRepository {
    async fn log_message(
        &self,
        conversation_id: &str,
        listing_id: &str,
        sender: &str,
        receiver: Option<&str>,
        is_agent: bool,
        content: &str,
        image_data: Option<&str>,
        audio_data: Option<&str>,
        image_url: Option<&str>,
        audio_url: Option<&str>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, image_data, audio_data, image_url, audio_url) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(conversation_id)
        .bind(listing_id)
        .bind(sender)
        .bind(receiver)
        .bind(is_agent)
        .bind(content)
        .bind(image_data)
        .bind(audio_data)
        .bind(image_url)
        .bind(audio_url)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn get_conversation_history(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ChatHistoryEntry>, ApiError> {
        const LIMIT: i64 = 10;
        let rows = sqlx::query(
            "SELECT sender, content, is_agent, image_data, audio_data, image_url, audio_url FROM chat_messages \
             WHERE conversation_id = $1 ORDER BY id ASC LIMIT $2",
        )
        .bind(conversation_id)
        .bind(LIMIT)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let image_data: Option<String> = row.try_get("image_data").ok().flatten();
                let audio_data: Option<String> = row.try_get("audio_data").ok().flatten();
                let image_url: Option<String> = row.try_get("image_url").ok().flatten();
                let audio_url: Option<String> = row.try_get("audio_url").ok().flatten();
                ChatHistoryEntry {
                    sender: row.get("sender"),
                    content: row.get("content"),
                    is_agent: row.get("is_agent"),
                    image_data,
                    audio_data,
                    image_url,
                    audio_url,
                }
            })
            .collect())
    }

    async fn list_conversations(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ConversationSummary>, i64), ApiError> {
        let rows = sqlx::query(
            r#"
            SELECT cc.id, cc.requester_id, cc.receiver_id, cc.status, cc.established_at, cc.created_at, cc.unread_count,
                   CASE WHEN cc.requester_id = $1 THEN cc.receiver_id ELSE cc.requester_id END as other_user_id,
                   u2.username as other_username,
                   CASE WHEN cc.receiver_id = $1 THEN true ELSE false END as is_receiver
            FROM chat_connections cc
            LEFT JOIN users u2 ON u2.id = CASE WHEN cc.requester_id = $1 THEN cc.receiver_id ELSE cc.requester_id END
            WHERE cc.requester_id = $1 OR cc.receiver_id = $1
            ORDER BY cc.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let summaries: Vec<ConversationSummary> = rows
            .into_iter()
            .map(|row| {
                let established_at: Option<chrono::DateTime<Utc>> = row.get("established_at");
                ConversationSummary {
                    id: row.get("id"),
                    requester_id: row.get("requester_id"),
                    other_user_id: row.get("other_user_id"),
                    other_username: row.get("other_username"),
                    status: row.get("status"),
                    established_at: established_at.map(|dt| dt.to_rfc3339()),
                    created_at: row
                        .get::<chrono::DateTime<Utc>, _>("created_at")
                        .to_rfc3339(),
                    unread_count: row.get("unread_count"),
                    is_receiver: row.get("is_receiver"),
                }
            })
            .collect();

        let count_row = sqlx::query(
            "SELECT COUNT(*) FROM chat_connections WHERE requester_id = $1 OR receiver_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let total: i64 = count_row.get(0);
        Ok((summaries, total))
    }

    async fn get_conversation_messages(
        &self,
        conversation_id: &str,
        before: Option<i64>,
        limit: i64,
    ) -> Result<(Vec<ChatMessage>, i64), ApiError> {
        let query = if before.is_some() {
            "SELECT id, conversation_id, sender, receiver, content, image_data, audio_data, image_url, audio_url, is_agent, edited_at, created_at \
             FROM chat_messages WHERE conversation_id = $1 AND id < $2 ORDER BY id DESC LIMIT $3"
        } else {
            "SELECT id, conversation_id, sender, receiver, content, image_data, audio_data, image_url, audio_url, is_agent, edited_at, created_at \
             FROM chat_messages WHERE conversation_id = $1 ORDER BY id DESC LIMIT $2"
        };

        let rows = if let Some(b) = before {
            sqlx::query_as::<_, ChatMessage>(query)
                .bind(conversation_id)
                .bind(b)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as::<_, ChatMessage>(query)
                .bind(conversation_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let count_row =
            sqlx::query("SELECT COUNT(*) FROM chat_messages WHERE conversation_id = $1")
                .bind(conversation_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let total: i64 = count_row.get(0);
        Ok((rows, total))
    }

    async fn mark_conversation_read(
        &self,
        conversation_id: &str,
        reader_id: &str,
    ) -> Result<(), ApiError> {
        let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = sqlx::Acquire::begin(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        sqlx::query(
            "UPDATE chat_messages SET read_at = NOW() \
             WHERE conversation_id = $1 AND receiver = $2 AND read_at IS NULL",
        )
        .bind(conversation_id)
        .bind(reader_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        sqlx::query(
            "UPDATE chat_connections SET unread_count = 0 WHERE id = $1 AND receiver_id = $2",
        )
        .bind(conversation_id)
        .bind(reader_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("Commit error: {}", e)))?;

        Ok(())
    }

    async fn edit_message(
        &self,
        message_id: &str,
        sender_id: &str,
        new_content: &str,
    ) -> Result<(), ApiError> {
        let _row = sqlx::query(
            "SELECT cm.id FROM chat_messages cm \
                         JOIN chat_connections cc \
                             ON cc.id = CASE \
                                                         WHEN cm.conversation_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' \
                                                         THEN cm.conversation_id::uuid \
                                                         ELSE NULL \
                                                    END \
             WHERE cm.id = $1 AND cm.sender = $2 AND cc.status = 'connected'",
        )
        .bind(message_id)
        .bind(sender_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("消息不存在或无权编辑".to_string()))?;

        let edited_at = Utc::now();
        sqlx::query("UPDATE chat_messages SET content = $1, edited_at = $2 WHERE id = $3")
            .bind(new_content)
            .bind(edited_at)
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn mark_message_read(&self, message_id: &str, reader_id: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE chat_messages SET read_at = NOW() WHERE id = $1 AND receiver = $2")
            .bind(message_id)
            .bind(reader_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn request_connection(
        &self,
        requester_id: &str,
        receiver_id: &str,
        listing_id: &str,
    ) -> Result<String, ApiError> {
        let connection_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO chat_connections (id, requester_id, receiver_id, listing_id, status) \
             VALUES ($1, $2, $3, $4, 'pending')",
        )
        .bind(&connection_id)
        .bind(requester_id)
        .bind(receiver_id)
        .bind(listing_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(connection_id)
    }

    async fn accept_connection(
        &self,
        connection_id: &str,
        acceptor_id: &str,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "UPDATE chat_connections SET status = 'connected', established_at = NOW() \
             WHERE id = $1 AND receiver_id = $2 AND status = 'pending'",
        )
        .bind(connection_id)
        .bind(acceptor_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn reject_connection(
        &self,
        connection_id: &str,
        rejector_id: &str,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "UPDATE chat_connections SET status = 'rejected' \
             WHERE id = $1 AND receiver_id = $2 AND status = 'pending'",
        )
        .bind(connection_id)
        .bind(rejector_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }
}
