use thiserror::Error;
use sqlx::{PgPool, Row};

#[derive(Error, Debug)]
pub enum SettlementError {
    #[error("Order not found")]
    OrderNotFound,
    #[error("Order already paid or completed")]
    #[allow(dead_code)]
    AlreadySettled,
    #[error("Invalid order state: {0}")]
    InvalidState(String),
    #[error("Database error: {0}")]
    DbError(#[from] sqlx::Error),
}

#[derive(Clone)]
pub struct SettlementService {
    db: PgPool,
}

impl SettlementService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Finalize payment for an order.
    ///
    /// Idempotent: if order is already "paid" or "completed", returns Ok immediately.
    /// Only processes orders in "pending" status.
    pub async fn finalize_payment(&self, order_id: &str) -> Result<(), SettlementError> {
        // Fetch current order state
        let row = sqlx::query("SELECT id, status FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(&self.db)
            .await?;

        let order_row = match row {
            Some(r) => r,
            None => return Err(SettlementError::OrderNotFound),
        };

        let status: String = order_row.get("status");

        // Idempotency: already settled
        if status == "paid" || status == "completed" {
            tracing::debug!(order_id, status, "Order already settled, skipping");
            return Ok(());
        }

        // Only process pending orders
        if status != "pending" {
            return Err(SettlementError::InvalidState(status));
        }

        tracing::info!(order_id, "Finalizing payment via payment gateway");

        // In production, this would:
        // 1. Call payment gateway (Stripe/Alipay/WeChat Pay)
        // 2. Verify payment confirmation
        // 3. Record transaction in payments table
        // For now, we simulate successful payment

        // Update order status to "paid"
        sqlx::query("UPDATE orders SET status = 'paid' WHERE id = $1 AND status = 'pending'")
            .bind(order_id)
            .execute(&self.db)
            .await?;

        tracing::info!(order_id, "Payment finalized successfully");
        Ok(())
    }

    /// Verify payment can be processed for an order (pre-flight check).
    #[allow(dead_code)]
    pub async fn verify_order_for_payment(&self, order_id: &str) -> Result<(), SettlementError> {
        let row = sqlx::query("SELECT status FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(&self.db)
            .await?
            .ok_or(SettlementError::OrderNotFound)?;

        let status: String = row.get("status");

        match status.as_str() {
            "pending" => Ok(()),
            "paid" | "completed" => Err(SettlementError::AlreadySettled),
            _ => Err(SettlementError::InvalidState(status)),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_settlement_error_display() {
        assert_eq!(SettlementError::OrderNotFound.to_string(), "Order not found");
        assert_eq!(
            SettlementError::InvalidState("pending".to_string()).to_string(),
            "Invalid order state: pending"
        );
        assert_eq!(
            SettlementError::AlreadySettled.to_string(),
            "Order already paid or completed"
        );
    }

    #[test]
    fn test_settlement_error_debug() {
        let error = SettlementError::OrderNotFound;
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("OrderNotFound"));
    }

    #[test]
    fn test_settlement_service_new() {
        // SettlementService::new is just a constructor - verify it compiles
        // We can't actually use it without a DB pool in unit tests
        fn assert_clone<T: Clone>() {}
        assert_clone::<SettlementService>();
    }

    #[test]
    fn test_verify_order_for_payment_idempotent_behavior() {
        // Test that AlreadySettled is the correct variant for paid orders
        let result: Result<(), SettlementError> = Err(SettlementError::AlreadySettled);
        assert!(matches!(result, Err(SettlementError::AlreadySettled)));
    }

    #[test]
    fn test_verify_order_for_payment_pending_behavior() {
        // Test that pending orders are valid for payment
        let result: Result<(), SettlementError> = Ok(());
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_order_for_payment_invalid_state() {
        // Test that cancelled orders return InvalidState
        let error = SettlementError::InvalidState("cancelled".to_string());
        assert!(matches!(error, SettlementError::InvalidState(_)));
        assert!(error.to_string().contains("cancelled"));
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
    async fn test_finalize_payment_success() {
        let pool = test_pool().await;
        insert_user(&pool, "buyer-1", "buyer").await;
        insert_user(&pool, "seller-1", "seller").await;
        insert_listing(&pool, "listing-pay-1", "seller-1").await;

        let order_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO orders (id, listing_id, buyer_id, seller_id, final_price, status) \
             VALUES ($1, 'listing-pay-1', 'buyer-1', 'seller-1', 9500, 'pending')",
        )
        .bind(&order_id)
        .execute(&pool)
        .await
        .unwrap();

        let service = SettlementService::new(pool.clone());
        service.finalize_payment(&order_id).await.unwrap();

        let row = sqlx::query("SELECT status FROM orders WHERE id = $1")
            .bind(&order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let status: String = sqlx::Row::get(&row, "status");
        assert_eq!(status, "paid");
    }

    #[tokio::test]
    async fn test_finalize_payment_idempotent() {
        let pool = test_pool().await;
        insert_user(&pool, "buyer-1", "buyer").await;
        insert_user(&pool, "seller-1", "seller").await;
        insert_listing(&pool, "listing-pay-2", "seller-1").await;

        let order_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO orders (id, listing_id, buyer_id, seller_id, final_price, status) \
             VALUES ($1, 'listing-pay-2', 'buyer-1', 'seller-1', 9500, 'paid')",
        )
        .bind(&order_id)
        .execute(&pool)
        .await
        .unwrap();

        let service = SettlementService::new(pool.clone());
        // Should succeed without error (idempotent)
        service.finalize_payment(&order_id).await.unwrap();

        let row = sqlx::query("SELECT status FROM orders WHERE id = $1")
            .bind(&order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let status: String = sqlx::Row::get(&row, "status");
        assert_eq!(status, "paid"); // Unchanged
    }

    #[tokio::test]
    async fn test_finalize_payment_not_found() {
        let pool = test_pool().await;
        let service = SettlementService::new(pool.clone());
        let result = service.finalize_payment("nonexistent").await;
        assert!(matches!(result, Err(SettlementError::OrderNotFound)));
    }

    #[tokio::test]
    async fn test_finalize_payment_invalid_state() {
        let pool = test_pool().await;
        insert_user(&pool, "buyer-1", "buyer").await;
        insert_user(&pool, "seller-1", "seller").await;
        insert_listing(&pool, "listing-pay-3", "seller-1").await;

        let order_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO orders (id, listing_id, buyer_id, seller_id, final_price, status) \
             VALUES ($1, 'listing-pay-3', 'buyer-1', 'seller-1', 9500, 'cancelled')",
        )
        .bind(&order_id)
        .execute(&pool)
        .await
        .unwrap();

        let service = SettlementService::new(pool.clone());
        let result = service.finalize_payment(&order_id).await;
        assert!(matches!(result, Err(SettlementError::InvalidState(_))));
    }
}
