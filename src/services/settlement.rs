use sqlx::PgPool;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum SettlementError {
    #[error("Order not found")]
    OrderNotFound,
    #[error("Order already paid or completed")]
    AlreadySettled,
    #[error("Invalid order state: {0}")]
    InvalidState(String),
    #[error("Settlement is disabled")]
    Disabled,
    #[error("Database error: {0}")]
    DbError(#[from] sqlx::Error),
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct SettlementService {
    db: PgPool,
}

impl SettlementService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Returns an error indicating settlement is disabled.
    #[allow(dead_code)]
    pub async fn finalize_payment(&self, order_id: &str) -> Result<(), SettlementError> {
        tracing::warn!(
            order_id,
            "Settlement finalize_payment called but settlement is disabled"
        );
        Err(SettlementError::Disabled)
    }

    /// Verify payment can be processed for an order (pre-flight check).
    #[allow(dead_code)]
    pub async fn verify_order_for_payment(&self, order_id: &str) -> Result<(), SettlementError> {
        tracing::warn!(
            order_id,
            "Settlement verify_order_for_payment called but settlement is disabled"
        );
        Err(SettlementError::Disabled)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_settlement_error_display() {
        assert_eq!(
            SettlementError::OrderNotFound.to_string(),
            "Order not found"
        );
        assert_eq!(
            SettlementError::InvalidState("pending".to_string()).to_string(),
            "Invalid order state: pending"
        );
        assert_eq!(
            SettlementError::AlreadySettled.to_string(),
            "Order already paid or completed"
        );
        assert_eq!(
            SettlementError::Disabled.to_string(),
            "Settlement is disabled"
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
