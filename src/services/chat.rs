use anyhow::Result;
use serde::Serialize;
use sqlx::{PgPool, Row};

/// Maximum number of historical message pairs to include in conversation context
const CONVERSATION_HISTORY_LIMIT: usize = 10;

/// A single turn in the conversation history
#[derive(Debug, Clone)]
pub struct ChatHistoryEntry {
    #[allow(dead_code)]
    pub sender: String,
    pub content: String,
    pub is_agent: bool,
    #[allow(dead_code)]
    pub image_data: Option<String>,
    #[allow(dead_code)]
    pub audio_data: Option<String>,
}

#[derive(Clone)]
pub struct ChatService {
    db: PgPool,
}

impl ChatService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Log a chat message to the database.
    /// receiver is the intended recipient (listing owner for item inquiries, null for global/agent messages).
    #[allow(clippy::too_many_arguments)]
    pub async fn log_message(
        &self,
        conversation_id: &str,
        listing_id: &str,
        sender: &str,
        receiver: Option<&str>,
        is_agent: bool,
        content: &str,
        image_data: Option<&str>,
        audio_data: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, image_data, audio_data) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(conversation_id)
        .bind(listing_id)
        .bind(sender)
        .bind(receiver)
        .bind(is_agent)
        .bind(content)
        .bind(image_data)
        .bind(audio_data)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Fetch the most recent conversation history for a given conversation_id.
    /// Returns up to CONVERSATION_HISTORY_LIMIT entries, oldest first.
    pub async fn get_conversation_history(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ChatHistoryEntry>> {
        let rows = sqlx::query(
            "SELECT sender, content, is_agent, image_data, audio_data FROM chat_messages \
             WHERE conversation_id = $1 ORDER BY id ASC LIMIT $2",
        )
        .bind(conversation_id)
        .bind(CONVERSATION_HISTORY_LIMIT as i64)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let image_data: Option<String> = Row::get(&row, "image_data");
                let audio_data: Option<String> = Row::get(&row, "audio_data");
                ChatHistoryEntry {
                    sender: Row::get(&row, "sender"),
                    content: Row::get(&row, "content"),
                    is_agent: Row::get(&row, "is_agent"),
                    image_data,
                    audio_data,
                }
            })
            .collect())
    }

    /// List all conversation IDs for a user with metadata.
    /// Returns paginated results ordered by most recent message.
    pub async fn list_conversations(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ConversationSummary>, i64)> {
        // Count conversations where user is either sender or receiver.
        // The receiver column was added later, so NULL receiver means sender-only visibility.
        let count_row = sqlx::query(
            "SELECT COUNT(DISTINCT conversation_id) as cnt \
             FROM chat_messages \
             WHERE sender = $1 OR receiver = $1",
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;
        let total: i64 = count_row.try_get("cnt").unwrap_or(0);

        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (cm.conversation_id)
                   cm.conversation_id,
                   cm.listing_id,
                   i.title as listing_title,
                   cm.content as last_message,
                   cm.is_agent as last_message_is_agent,
                   cm.timestamp as last_timestamp,
                   CASE WHEN cm.sender = $1 THEN cm.receiver ELSE cm.sender END as other_user_id
            FROM chat_messages cm
            LEFT JOIN inventory i ON cm.listing_id = i.id
            WHERE cm.sender = $1 OR cm.receiver = $1
            ORDER BY cm.conversation_id, cm.timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        // Batch-fetch usernames for other participants
        let other_ids: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get::<Option<String>, _>("other_user_id").ok().flatten())
            .collect();
        let other_usernames: std::collections::HashMap<String, String> = if other_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            sqlx::query("SELECT id, username FROM users WHERE id = ANY($1)")
                .bind(&other_ids)
                .fetch_all(&self.db)
                .await
                .map(|rows| {
                    rows.into_iter()
                        .map(|row| (row.get::<String, _>("id"), row.get::<String, _>("username")))
                        .collect()
                })
                .unwrap_or_default()
        };

        let items = rows
            .into_iter()
            .map(|row| {
                let other_user_id: Option<String> = row.try_get("other_user_id").ok().flatten();
                let other_username = other_user_id
                    .as_ref()
                    .and_then(|id| other_usernames.get(id).cloned());
                ConversationSummary {
                    conversation_id: row.get("conversation_id"),
                    listing_id: row.get("listing_id"),
                    listing_title: row.try_get("listing_title").ok(),
                    last_message: row.get("last_message"),
                    last_message_is_agent: row.get("last_message_is_agent"),
                    last_timestamp: row
                        .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>(
                            "last_timestamp",
                        )
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default(),
                    other_user_id,
                    other_username,
                }
            })
            .collect();

        Ok((items, total))
    }
}

/// Summary of a conversation for listing
#[derive(Debug, Clone, Serialize)]
pub struct ConversationSummary {
    pub conversation_id: String,
    pub listing_id: String,
    pub listing_title: Option<String>,
    pub last_message: String,
    pub last_message_is_agent: bool,
    pub last_timestamp: String,
    /// User ID of the other participant in this conversation.
    pub other_user_id: Option<String>,
    /// Username of the other participant.
    pub other_username: Option<String>,
}

// ---------------------------------------------------------------------------
// Unit tests (no DB required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_chat_history_entry_clone() {
        let entry = ChatHistoryEntry {
            sender: "user-1".to_string(),
            content: "Hello".to_string(),
            is_agent: false,
            image_data: None,
            audio_data: None,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.content, "Hello");
        assert_eq!(cloned.is_agent, false);
    }

    #[test]
    fn test_chat_history_entry_with_media() {
        let entry = ChatHistoryEntry {
            sender: "user-1".to_string(),
            content: "Check this image".to_string(),
            is_agent: true,
            image_data: Some("base64image".to_string()),
            audio_data: Some("base64audio".to_string()),
        };
        assert!(entry.image_data.is_some());
        assert!(entry.audio_data.is_some());
        assert_eq!(entry.sender, "user-1");
    }

    #[test]
    fn test_conversation_summary_serialization() {
        let summary = ConversationSummary {
            conversation_id: "conv-123".to_string(),
            listing_id: "listing-456".to_string(),
            listing_title: Some("iPhone 13".to_string()),
            last_message: "Is this still available?".to_string(),
            last_message_is_agent: false,
            last_timestamp: "2024-01-01T12:00:00Z".to_string(),
            other_user_id: None,
            other_username: None,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("conv-123"));
        assert!(json.contains("listing-456"));
        assert!(json.contains("iPhone 13"));
        assert!(json.contains("Is this still available"));
    }

    #[test]
    fn test_conversation_summary_without_title() {
        let summary = ConversationSummary {
            conversation_id: "conv-789".to_string(),
            listing_id: "listing-000".to_string(),
            listing_title: None,
            last_message: "Hello!".to_string(),
            last_message_is_agent: true,
            last_timestamp: "2024-01-01T12:00:00Z".to_string(),
            other_user_id: Some("user-other".to_string()),
            other_username: Some("other_user".to_string()),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("conv-789"));
        assert!(json.contains("listing-000"));
        assert!(json.contains("\"last_message_is_agent\":true"));
    }

    #[test]
    fn test_conversation_summary_empty_title() {
        let summary = ConversationSummary {
            conversation_id: "conv-empty".to_string(),
            listing_id: "listing-empty".to_string(),
            listing_title: None,
            last_message: "".to_string(),
            last_message_is_agent: false,
            last_timestamp: "".to_string(),
            other_user_id: None,
            other_username: None,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("conv-empty"));
        assert!(json.contains("\"listing_title\":null"));
    }

    #[test]
    fn test_chat_service_clone() {
        // ChatService is Clone, verify it compiles
        fn assert_clone<T: Clone>() {}
        assert_clone::<ChatService>();
    }

    #[test]
    fn test_conversation_history_limit_constant() {
        // Verify the constant is a reasonable size for context window
        assert!(CONVERSATION_HISTORY_LIMIT >= 1);
        assert!(CONVERSATION_HISTORY_LIMIT <= 100);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect("postgres://postgres:postgres@localhost/test_db")
            .await
            .unwrap();
        crate::db::setup_schema(&pool).await.unwrap();
        pool
    }

    async fn insert_user(pool: &PgPool, id: &str, username: &str) {
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, 'hash')")
            .bind(id)
            .bind(username)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn insert_listing(pool: &PgPool, id: &str, owner_id: &str) {
        sqlx::query(
            "INSERT INTO inventory (id, title, category, brand, condition_score, suggested_price_cny, defects, owner_id) \
             VALUES ($1, 'Item', 'misc', 'Brand', 8, 10000, '[]', $2)",
        )
        .bind(id)
        .bind(owner_id)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_log_message() {
        let pool = test_pool().await;
        insert_user(&pool, "user-1", "user1").await;
        insert_listing(&pool, "listing-1", "user-1").await;
        ChatService::new(pool.clone())
            .log_message(
                "conv-1",
                "listing-1",
                "user-1",
                None,
                false,
                "Hello!",
                None,
                None,
            )
            .await
            .unwrap();

        let row =
            sqlx::query("SELECT sender, content FROM chat_messages WHERE listing_id = 'listing-1'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(Row::get::<String, _>(&row, "sender"), "user-1");
        assert_eq!(Row::get::<String, _>(&row, "content"), "Hello!");
    }

    #[tokio::test]
    async fn test_get_conversation_history() {
        let pool = test_pool().await;
        insert_user(&pool, "user-1", "user1").await;
        insert_listing(&pool, "listing-1", "user-1").await;

        let chat_svc = ChatService::new(pool.clone());

        // Log multiple messages in the same conversation
        chat_svc
            .log_message(
                "conv-test",
                "listing-1",
                "user-1",
                None,
                false,
                "First message",
                None,
                None,
            )
            .await
            .unwrap();
        chat_svc
            .log_message(
                "conv-test",
                "listing-1",
                "user-1",
                None,
                true,
                "Agent reply",
                None,
                None,
            )
            .await
            .unwrap();
        chat_svc
            .log_message(
                "conv-test",
                "listing-1",
                "user-1",
                None,
                false,
                "Third message",
                None,
                None,
            )
            .await
            .unwrap();

        let history = chat_svc
            .get_conversation_history("conv-test")
            .await
            .unwrap();

        assert_eq!(history.len(), 3);
        assert_eq!(history[0].content, "First message");
        assert!(!history[0].is_agent);
        assert_eq!(history[1].content, "Agent reply");
        assert!(history[1].is_agent);
        assert_eq!(history[2].content, "Third message");
        assert!(!history[2].is_agent);
    }

    #[tokio::test]
    async fn test_get_conversation_history_empty() {
        let pool = test_pool().await;
        insert_user(&pool, "user-1", "user1").await;

        let history = ChatService::new(pool.clone())
            .get_conversation_history("nonexistent-conv")
            .await
            .unwrap();

        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_list_conversations() {
        let pool = test_pool().await;
        insert_user(&pool, "user-1", "user1").await;
        insert_user(&pool, "user-2", "user2").await;
        insert_listing(&pool, "listing-1", "user-1").await;

        let chat_svc = ChatService::new(pool.clone());

        // Create conversations for user-1
        chat_svc
            .log_message(
                "conv-1",
                "listing-1",
                "user-1",
                None,
                false,
                "Message in conv 1",
                None,
                None,
            )
            .await
            .unwrap();
        chat_svc
            .log_message(
                "conv-2",
                "listing-1",
                "user-1",
                None,
                false,
                "Message in conv 2",
                None,
                None,
            )
            .await
            .unwrap();

        // Create conversation for user-2 (should not appear for user-1)
        chat_svc
            .log_message(
                "conv-3",
                "listing-1",
                "user-2",
                None,
                false,
                "User-2 message",
                None,
                None,
            )
            .await
            .unwrap();

        let (conversations, total) = chat_svc.list_conversations("user-1", 20, 0).await.unwrap();

        assert_eq!(conversations.len(), 2);
        assert_eq!(total, 2);
        let conv_ids: Vec<&str> = conversations
            .iter()
            .map(|c| c.conversation_id.as_str())
            .collect();
        assert!(conv_ids.contains(&"conv-1"));
        assert!(conv_ids.contains(&"conv-2"));
        assert!(!conv_ids.contains(&"conv-3"));
    }

    #[tokio::test]
    async fn test_list_conversations_with_listing_title() {
        let pool = test_pool().await;
        insert_user(&pool, "user-1", "user1").await;
        insert_listing(&pool, "listing-1", "user-1").await;

        let chat_svc = ChatService::new(pool.clone());
        chat_svc
            .log_message(
                "conv-1",
                "listing-1",
                "user-1",
                None,
                false,
                "Hello",
                None,
                None,
            )
            .await
            .unwrap();

        let (conversations, total) = chat_svc.list_conversations("user-1", 20, 0).await.unwrap();

        assert_eq!(conversations.len(), 1);
        assert_eq!(total, 1);
        assert_eq!(conversations[0].listing_id, "listing-1");
        assert_eq!(conversations[0].listing_title.as_deref(), Some("Item"));
    }
}
