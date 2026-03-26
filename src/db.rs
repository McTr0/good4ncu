use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Initializes the database: creates the pgvector extension and runs migrations.
/// Returns a single PgPool that handles both relational and vector data.
pub async fn init_db(database_url: &str) -> Result<PgPool> {
    // Create a PgPool for relational + vector data
    let db_pool = PgPoolOptions::new()
        .min_connections(2) // Pre-warm pool to reduce cold-start latency
        .max_connections(20)
        .connect(database_url)
        .await?;

    // Enable pgvector extension (creates the vector type and operators)
    // This must be done before running migrations since the vector type is needed
    // by the documents table migration.
    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(&db_pool)
        .await?;

    // Run versioned migrations (includes all CREATE TABLE, CREATE INDEX, etc.)
    // The migrations directory is embedded at compile time via sqlx::migrate!()
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    Ok(db_pool)
}
