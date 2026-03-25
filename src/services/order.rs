use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrderService {
    db: PgPool,
}

impl OrderService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_order(
        &self,
        listing_id: &str,
        buyer_id: &str,
        seller_id: &str,
        final_price: i64,
    ) -> Result<String> {
        let order_id = Uuid::new_v4().to_string();
        tracing::info!(order_id, listing_id, buyer_id, seller_id, "Creating order");

        sqlx::query(
            "INSERT INTO orders (id, listing_id, buyer_id, seller_id, final_price, status) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&order_id)
        .bind(listing_id)
        .bind(buyer_id)
        .bind(seller_id)
        .bind(final_price)
        .bind("pending")
        .execute(&self.db)
        .await?;

        Ok(order_id)
    }

    pub async fn update_order_status(&self, order_id: &str, status: &str) -> Result<()> {
        tracing::info!(order_id, status, "Updating order status");
        sqlx::query("UPDATE orders SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(order_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Atomically transition order status from expected_current to new_status.
    /// Returns Ok(true) if transition happened, Ok(false) if order was not in expected state.
    #[allow(dead_code)]
    pub async fn transition_order_status(
        &self,
        order_id: &str,
        expected_current: &str,
        new_status: &str,
    ) -> Result<bool> {
        let result =
            sqlx::query("UPDATE orders SET status = $1 WHERE id = $2 AND status = $3 RETURNING id")
                .bind(new_status)
                .bind(order_id)
                .bind(expected_current)
                .fetch_optional(&self.db)
                .await?;

        match result {
            Some(_) => {
                tracing::info!(
                    order_id,
                    expected_current,
                    new_status,
                    "Order status transitioned"
                );
                Ok(true)
            }
            None => {
                tracing::warn!(
                    order_id,
                    expected_current,
                    new_status,
                    "Order status transition failed - not in expected state"
                );
                Ok(false)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests (no DB required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_order_service_clone() {
        // OrderService is Clone, verify it compiles
        fn assert_clone<T: Clone>() {}
        assert_clone::<OrderService>();
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
    async fn test_create_order() {
        let pool = test_pool().await;
        insert_user(&pool, "buyer-1", "buyer").await;
        insert_user(&pool, "seller-1", "seller").await;
        insert_listing(&pool, "listing-1", "seller-1").await;

        let service = OrderService::new(pool.clone());
        let order_id = service
            .create_order("listing-1", "buyer-1", "seller-1", 9500)
            .await
            .unwrap();

        let row = sqlx::query("SELECT status, final_price FROM orders WHERE id = $1")
            .bind(&order_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let status: String = sqlx::Row::get(&row, "status");
        let final_price: i64 = sqlx::Row::get(&row, "final_price");
        assert_eq!(status, "pending");
        assert_eq!(final_price, 9500);
    }

    #[tokio::test]
    async fn test_update_order_status() {
        let pool = test_pool().await;
        insert_user(&pool, "buyer-1", "buyer").await;
        insert_user(&pool, "seller-1", "seller").await;
        insert_listing(&pool, "listing-2", "seller-1").await;

        let service = OrderService::new(pool.clone());
        let order_id = service
            .create_order("listing-2", "buyer-1", "seller-1", 8000)
            .await
            .unwrap();

        service
            .update_order_status(&order_id, "completed")
            .await
            .unwrap();

        let row = sqlx::query("SELECT status FROM orders WHERE id = $1")
            .bind(&order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let status: String = sqlx::Row::get(&row, "status");
        assert_eq!(status, "completed");
    }
}
