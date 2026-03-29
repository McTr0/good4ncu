use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct OrderService {
    db: PgPool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
    Completed,
    Cancelled,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Pending => write!(f, "pending"),
            OrderStatus::Paid => write!(f, "paid"),
            OrderStatus::Shipped => write!(f, "shipped"),
            OrderStatus::Completed => write!(f, "completed"),
            OrderStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl OrderStatus {
    pub fn parse_status(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "paid" => Some(Self::Paid),
            "shipped" => Some(Self::Shipped),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Valid transitions: pending→paid, paid→shipped, shipped→completed, pending→cancelled, paid→cancelled
    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        matches!(
            (self, next),
            (OrderStatus::Pending, OrderStatus::Paid)
                | (OrderStatus::Pending, OrderStatus::Cancelled)
                | (OrderStatus::Paid, OrderStatus::Shipped)
                | (OrderStatus::Paid, OrderStatus::Cancelled)
                | (OrderStatus::Shipped, OrderStatus::Completed)
        )
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum OrderError {
    #[error("Order not found")]
    NotFound,
    #[error("Invalid status transition: {0}")]
    InvalidTransition(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
}

pub struct OrderRow {
    pub id: String,
    pub listing_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub final_price: i64,
    pub status: String,
    pub cancellation_reason: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub buyer_username: String,
    pub seller_username: String,
    pub listing_title: String,
}

pub struct OrderSummaryRow {
    pub id: String,
    pub listing_id: String,
    pub listing_title: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub final_price: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub role: String,
    pub buyer_username: String,
    pub seller_username: String,
}

impl OrderService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Create a new order. Returns the order ID.
    pub async fn create_order(
        &self,
        listing_id: &str,
        buyer_id: &str,
        seller_id: &str,
        final_price: i64,
    ) -> Result<String, OrderError> {
        let order_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO orders (id, listing_id, buyer_id, seller_id, final_price, status)
            VALUES ($1, $2, $3, $4, $5, 'pending')
            "#,
        )
        .bind(&order_id)
        .bind(listing_id)
        .bind(buyer_id)
        .bind(seller_id)
        .bind(final_price)
        .execute(&self.db)
        .await
        .map_err(OrderError::Db)?;

        Ok(order_id)
    }

    /// Get order with buyer/seller info.
    pub async fn get_order_with_details(
        &self,
        order_id: &str,
    ) -> Result<Option<OrderRow>, OrderError> {
        let row = sqlx::query_as::<_, SqlxOrderRow>(
            r#"
            SELECT o.id, o.listing_id, o.buyer_id, o.seller_id, o.final_price,
                   o.status, o.cancellation_reason,
                   o.paid_at, o.shipped_at, o.completed_at, o.cancelled_at, o.created_at,
                   i.title as listing_title,
                   buyer.username as buyer_username,
                   seller.username as seller_username
            FROM orders o
            JOIN inventory i ON i.id = o.listing_id
            JOIN users buyer ON buyer.id = o.buyer_id
            JOIN users seller ON seller.id = o.seller_id
            WHERE o.id = $1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.db)
        .await
        .map_err(OrderError::Db)?;

        Ok(row.map(|r| OrderRow {
            id: r.id,
            listing_id: r.listing_id,
            buyer_id: r.buyer_id,
            seller_id: r.seller_id,
            final_price: r.final_price,
            status: r.status,
            cancellation_reason: r.cancellation_reason,
            paid_at: r.paid_at,
            shipped_at: r.shipped_at,
            completed_at: r.completed_at,
            cancelled_at: r.cancelled_at,
            created_at: r.created_at,
            buyer_username: r.buyer_username,
            seller_username: r.seller_username,
            listing_title: r.listing_title,
        }))
    }

    /// Get paginated orders for a user.
    pub async fn list_orders(
        &self,
        user_id: &str,
        role: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<OrderSummaryRow>, i64), OrderError> {
        let where_clause = match role {
            Some("buyer") => "WHERE o.buyer_id = $1",
            Some("seller") => "WHERE o.seller_id = $1",
            _ => "WHERE (o.buyer_id = $1 OR o.seller_id = $1)",
        };

        let total: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM orders o {}", where_clause))
                .bind(user_id)
                .fetch_one(&self.db)
                .await
                .map_err(OrderError::Db)?;

        let rows: Vec<SqlxOrderSummaryRow> = sqlx::query_as(&format!(
            r#"
            SELECT o.id, o.listing_id, o.buyer_id, o.seller_id, o.final_price,
                   o.status, o.created_at,
                   i.title as listing_title,
                   buyer.username as buyer_username,
                   seller.username as seller_username
            FROM orders o
            JOIN inventory i ON i.id = o.listing_id
            JOIN users buyer ON buyer.id = o.buyer_id
            JOIN users seller ON seller.id = o.seller_id
            {}
            ORDER BY o.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            where_clause
        ))
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
        .map_err(OrderError::Db)?;

        let items = rows
            .into_iter()
            .map(|r| {
                let role = if r.buyer_id == user_id {
                    "buyer"
                } else {
                    "seller"
                };
                OrderSummaryRow {
                    id: r.id,
                    listing_id: r.listing_id,
                    listing_title: r.listing_title,
                    buyer_id: r.buyer_id,
                    seller_id: r.seller_id,
                    final_price: r.final_price,
                    status: r.status,
                    created_at: r.created_at,
                    role: role.to_string(),
                    buyer_username: r.buyer_username,
                    seller_username: r.seller_username,
                }
            })
            .collect();

        Ok((items, total))
    }

    /// Transition order to a new status atomically.
    pub async fn transition_order_status(
        &self,
        order_id: &str,
        expected_current: &str,
        new_status: &str,
        cancellation_reason: Option<&str>,
    ) -> Result<bool, OrderError> {
        let current = OrderStatus::parse_status(expected_current).ok_or_else(|| {
            OrderError::InvalidTransition(format!("unknown current status: {expected_current}"))
        })?;
        let next = OrderStatus::parse_status(new_status).ok_or_else(|| {
            OrderError::InvalidTransition(format!("unknown new status: {new_status}"))
        })?;

        if !current.can_transition_to(&next) {
            return Ok(false);
        }

        let (timestamp_field, reason_set) = match next {
            OrderStatus::Paid => ("paid_at", false),
            OrderStatus::Shipped => ("shipped_at", false),
            OrderStatus::Completed => ("completed_at", false),
            OrderStatus::Cancelled => ("cancelled_at", true),
            OrderStatus::Pending => return Ok(false),
        };

        let result = if reason_set {
            sqlx::query(&format!(
                "UPDATE orders SET status = $1, {} = NOW(), cancellation_reason = $2 WHERE id = $3 AND status = $4",
                timestamp_field
            ))
            .bind(new_status)
            .bind(cancellation_reason.unwrap_or(""))
            .bind(order_id)
            .bind(expected_current)
            .execute(&self.db)
            .await
        } else {
            sqlx::query(&format!(
                "UPDATE orders SET status = $1, {} = NOW() WHERE id = $2 AND status = $3",
                timestamp_field
            ))
            .bind(new_status)
            .bind(order_id)
            .bind(expected_current)
            .execute(&self.db)
            .await
        };

        let rows = result.map_err(OrderError::Db)?.rows_affected();
        Ok(rows > 0)
    }

    /// Verify user is buyer or seller of the order.
    pub async fn verify_order_access(
        &self,
        order_id: &str,
        user_id: &str,
    ) -> Result<bool, OrderError> {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT buyer_id, seller_id FROM orders WHERE id = $1")
                .bind(order_id)
                .fetch_optional(&self.db)
                .await
                .map_err(OrderError::Db)?;

        match row {
            Some((buyer_id, seller_id)) => Ok(buyer_id == user_id || seller_id == user_id),
            None => Ok(false),
        }
    }

    /// Get order status and price.
    pub async fn get_order_meta(
        &self,
        order_id: &str,
    ) -> Result<Option<(String, i64)>, OrderError> {
        let row = sqlx::query("SELECT status, final_price FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(&self.db)
            .await
            .map_err(OrderError::Db)?;

        Ok(row.map(|r| (r.get("status"), r.get("final_price"))))
    }
}

// sqlx row types — use FromRow derive + column name aliases
#[derive(sqlx::FromRow)]
struct SqlxOrderRow {
    id: String,
    listing_id: String,
    buyer_id: String,
    seller_id: String,
    final_price: i64,
    status: String,
    cancellation_reason: Option<String>,
    paid_at: Option<DateTime<Utc>>,
    shipped_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    buyer_username: String,
    seller_username: String,
    listing_title: String,
}

#[derive(sqlx::FromRow)]
struct SqlxOrderSummaryRow {
    id: String,
    listing_id: String,
    buyer_id: String,
    seller_id: String,
    final_price: i64,
    status: String,
    created_at: DateTime<Utc>,
    buyer_username: String,
    seller_username: String,
    listing_title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_display() {
        assert_eq!(OrderStatus::Pending.to_string(), "pending");
        assert_eq!(OrderStatus::Paid.to_string(), "paid");
        assert_eq!(OrderStatus::Shipped.to_string(), "shipped");
        assert_eq!(OrderStatus::Completed.to_string(), "completed");
        assert_eq!(OrderStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_order_status_from_str() {
        assert_eq!(
            OrderStatus::parse_status("pending"),
            Some(OrderStatus::Pending)
        );
        assert_eq!(OrderStatus::parse_status("paid"), Some(OrderStatus::Paid));
        assert_eq!(OrderStatus::parse_status("invalid"), None);
    }

    #[test]
    fn test_order_status_valid_transitions() {
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Paid));
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Cancelled));
        assert!(OrderStatus::Paid.can_transition_to(&OrderStatus::Shipped));
        assert!(OrderStatus::Paid.can_transition_to(&OrderStatus::Cancelled));
        assert!(OrderStatus::Shipped.can_transition_to(&OrderStatus::Completed));
    }

    #[test]
    fn test_order_status_invalid_transitions() {
        assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::Shipped));
        assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::Completed));
        assert!(!OrderStatus::Shipped.can_transition_to(&OrderStatus::Cancelled));
        assert!(!OrderStatus::Completed.can_transition_to(&OrderStatus::Cancelled));
        assert!(!OrderStatus::Cancelled.can_transition_to(&OrderStatus::Paid));
    }
}
