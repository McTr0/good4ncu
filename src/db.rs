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

    // Add cancellation_reason column if it doesn't exist (for audit trail)
    sqlx::query("ALTER TABLE orders ADD COLUMN IF NOT EXISTS cancellation_reason TEXT")
        .execute(pool)
        .await
        .ok();

    // Add timestamps for order lifecycle tracking
    sqlx::query("ALTER TABLE orders ADD COLUMN IF NOT EXISTS paid_at TIMESTAMPTZ")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ALTER TABLE orders ADD COLUMN IF NOT EXISTS shipped_at TIMESTAMPTZ")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ALTER TABLE orders ADD COLUMN IF NOT EXISTS completed_at TIMESTAMPTZ")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ALTER TABLE orders ADD COLUMN IF NOT EXISTS cancelled_at TIMESTAMPTZ")
        .execute(pool)
        .await
        .ok();

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS chat_messages (
            id BIGSERIAL PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            listing_id TEXT NOT NULL,
            sender TEXT NOT NULL,
            receiver TEXT,
            is_agent BOOLEAN NOT NULL DEFAULT FALSE,
            content TEXT NOT NULL,
            image_data TEXT,
            audio_data TEXT,
            timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(sender) REFERENCES users(id) ON DELETE CASCADE
        )"#,
    )
    .execute(pool)
    .await?;

    // Add receiver column if it doesn't exist (fixes IDOR: must check both sender and receiver)
    sqlx::query("ALTER TABLE chat_messages ADD COLUMN IF NOT EXISTS receiver TEXT")
        .execute(pool)
        .await
        .ok(); // Ignore error if column already exists

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

    // Documents table for pgvector RAG embeddings.
    // Stores listing content as vectors for semantic search.
    // The id column is TEXT (matches listing UUID strings from the app).
    // VECTOR_DIM defaults to 768 to match the default embedding model.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS documents (\
         id TEXT NOT NULL,\
         document JSONB NOT NULL,\
         embedded_text TEXT NOT NULL,\
         embedding vector(768)\
         )",
    )
    .execute(pool)
    .await?;

    // HNSW index on embeddings for fast cosine similarity queries.
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS document_embeddings_idx \
         ON documents USING hnsw(embedding vector_cosine_ops)",
    )
    .execute(pool)
    .await
    .ok();

    // Notifications table: seller receives an in-app notification when a buyer places an order.
    // Eliminates the need for sellers to poll /api/orders to detect new sales.
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS notifications (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            related_order_id TEXT,
            related_listing_id TEXT,
            is_read BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id, is_read, created_at)")
        .execute(pool)
        .await
        .ok();

    // HITL requests: stores pending seller approval requests for negotiation.
    // The seller responds via PATCH /api/negotiations/{id} with approve/reject/counter.
    // The marketplace agent waits on this record being resolved before proceeding.
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS hitl_requests (
            id TEXT PRIMARY KEY,
            listing_id TEXT NOT NULL,
            buyer_id TEXT NOT NULL,
            seller_id TEXT NOT NULL,
            proposed_price INTEGER NOT NULL,
            reason TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            -- pending: awaiting seller response
            -- approved: deal accepted
            -- rejected: seller declined
            -- countered: seller counter-offered with different price
            counter_price INTEGER,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            resolved_at TIMESTAMPTZ,
            FOREIGN KEY(listing_id) REFERENCES inventory(id) ON DELETE CASCADE,
            FOREIGN KEY(buyer_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY(seller_id) REFERENCES users(id) ON DELETE CASCADE
        )"#,
    )
    .execute(pool)
    .await?;

    // Index for seller's pending approval requests
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_hitl_seller_status ON hitl_requests(seller_id, status)",
    )
    .execute(pool)
    .await?;

    // Add buyer_action column for tracking buyer's response to seller's counter-offer.
    // This is a no-op if the column already exists (e.g., existing dev DBs).
    // accepted: buyer accepted seller's counter → triggers DealReached
    // rejected: buyer declined seller's counter → final rejection
    sqlx::query("ALTER TABLE hitl_requests ADD COLUMN IF NOT EXISTS buyer_action TEXT")
        .execute(pool)
        .await?;

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
