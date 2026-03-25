use anyhow::Result;
use sqlx::PgPool;

#[derive(Clone)]
pub struct ProductService {
    db: PgPool,
}

impl ProductService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn mark_as_sold(&self, listing_id: &str) -> Result<()> {
        tracing::info!(listing_id, "Marking listing as sold");
        sqlx::query("UPDATE inventory SET status = 'sold' WHERE id = $1")
            .bind(listing_id)
            .execute(&self.db)
            .await?;

        // Delete vector embeddings so sold items don't appear in RAG results.
        // pgvector stores documents in the same 'documents' table as relational data.
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(listing_id)
            .execute(&self.db)
            .await
            .ok(); // Non-fatal if vector cleanup fails

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests (no DB required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_product_service_clone() {
        // ProductService is Clone, verify it compiles
        fn assert_clone<T: Clone>() {}
        assert_clone::<ProductService>();
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

    #[tokio::test]
    async fn test_mark_as_sold() {
        let pool = test_pool().await;
        insert_user(&pool, "owner", "owneruser").await;
        sqlx::query(
            "INSERT INTO inventory (id, title, category, brand, condition_score, suggested_price_cny, defects, owner_id, status) \
             VALUES ('prod-1', 'Test', 'misc', 'Brand', 8, 10000, '[]', 'owner', 'active')",
        )
        .execute(&pool)
        .await
        .unwrap();

        ProductService::new(pool.clone())
            .mark_as_sold("prod-1")
            .await
            .unwrap();

        let row = sqlx::query("SELECT status FROM inventory WHERE id = 'prod-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let status: String = sqlx::Row::get(&row, "status");
        assert_eq!(status, "sold");
    }
}
