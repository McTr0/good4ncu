use anyhow::Result;
use sqlx::{PgPool, Row};

/// Maximum number of historical message pairs to include in conversation context
#[allow(dead_code)]
const CONVERSATION_HISTORY_LIMIT: usize = 10;

/// A single turn in the conversation history
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ChatHistoryEntry {
    pub sender: String,
    pub content: String,
    pub is_agent: bool,
    pub image_data: Option<String>,
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
    #[allow(clippy::too_many_arguments)]
    pub async fn log_message(
        &self,
        conversation_id: &str,
        listing_id: &str,
        sender: &str,
        is_agent: bool,
        content: &str,
        image_data: Option<&str>,
        audio_data: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO chat_messages (conversation_id, listing_id, sender, is_agent, content, image_data, audio_data) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(conversation_id)
        .bind(listing_id)
        .bind(sender)
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
    #[allow(dead_code)]
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
            .log_message("conv-1", "listing-1", "user-1", false, "Hello!", None, None)
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
}
