use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Creates all application tables. Extracted so tests can reuse it.
pub async fn setup_schema(pool: &PgPool) -> Result<()> {
    // Note: FK constraints are always enforced in Postgres — no PRAGMA needed.

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS inventory (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            category TEXT NOT NULL,
            brand TEXT NOT NULL,
            condition_score INTEGER NOT NULL CHECK (condition_score >= 1 AND condition_score <= 10),
            suggested_price_cny INTEGER NOT NULL CHECK (suggested_price_cny >= 0),
            defects TEXT NOT NULL,
            description TEXT,
            owner_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(owner_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS orders (
            id TEXT PRIMARY KEY,
            listing_id TEXT NOT NULL,
            buyer_id TEXT NOT NULL,
            seller_id TEXT NOT NULL,
            final_price INTEGER NOT NULL CHECK (final_price >= 0),
            status TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(listing_id) REFERENCES inventory(id) ON DELETE CASCADE,
            FOREIGN KEY(buyer_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY(seller_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS chat_messages (
            id BIGSERIAL PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            listing_id TEXT NOT NULL,
            sender TEXT NOT NULL,
            is_agent BOOLEAN NOT NULL DEFAULT FALSE,
            content TEXT NOT NULL,
            image_data TEXT,
            audio_data TEXT,
            timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(listing_id) REFERENCES inventory(id) ON DELETE CASCADE,
            FOREIGN KEY(sender) REFERENCES users(id) ON DELETE CASCADE
        )"#,
    )
    .execute(pool)
    .await?;

    // Add conversation_id column if it doesn't exist (for existing SQLite-migrated dbs)
    sqlx::query("ALTER TABLE chat_messages ADD COLUMN IF NOT EXISTS conversation_id TEXT")
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

    // Add is_agent column if it doesn't exist
    sqlx::query(
        "ALTER TABLE chat_messages ADD COLUMN IF NOT EXISTS is_agent BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(pool)
    .await
    .ok(); // Ignore error if column already exists

    // Migrate existing rows: set conversation_id based on listing_id
    sqlx::query(
        "UPDATE chat_messages SET conversation_id = listing_id || ':' || sender WHERE conversation_id IS NULL",
    )
    .execute(pool)
    .await
    .ok(); // Ignore error if column doesn't exist yet

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_chat_conversation ON chat_messages(conversation_id, timestamp)",
    )
    .execute(pool)
    .await
    .ok(); // Ignore error if index already exists

    // Index on sender for list_conversations query
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_sender ON chat_messages(sender)")
        .execute(pool)
        .await
        .ok();

    // Order indexes for efficient order history queries
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_buyer ON orders(buyer_id)")
        .execute(pool)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_seller ON orders(seller_id)")
        .execute(pool)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_orders_listing ON orders(listing_id)")
        .execute(pool)
        .await
        .ok();

    // Watchlist table for user favorites
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS watchlist (
            user_id TEXT NOT NULL,
            listing_id TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (user_id, listing_id),
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY(listing_id) REFERENCES inventory(id) ON DELETE CASCADE
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_watchlist_user ON watchlist(user_id)")
        .execute(pool)
        .await
        .ok();

    Ok(())
}

/// Initializes the database: creates the pgvector extension and relational schema.
/// Returns a single PgPool that handles both relational and vector data.
pub async fn init_db(database_url: &str) -> Result<PgPool> {
    // Create a PgPool for relational + vector data
    let db_pool = PgPoolOptions::new()
        .min_connections(2) // Pre-warm pool to reduce cold-start latency
        .max_connections(20)
        .connect(database_url)
        .await?;

    // Enable pgvector extension (creates the vector type and operators)
    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(&db_pool)
        .await?;

    // Initialize tables
    setup_schema(&db_pool).await?;

    Ok(db_pool)
}
