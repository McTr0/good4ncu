//! PostgreSQL implementation of the OrderRepository trait.

use crate::api::error::ApiError;
use crate::repositories::traits::{Order, OrderRepository, OrderSummary};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct PostgresOrderRepository {
    pool: PgPool,
}

impl PostgresOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl OrderRepository for PostgresOrderRepository {
    async fn create(
        &self,
        id: &str,
        listing_id: &str,
        buyer_id: &str,
        seller_id: &str,
        final_price: i64,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            INSERT INTO orders (id, listing_id, buyer_id, seller_id, final_price, status)
            VALUES ($1, $2, $3, $4, $5, 'pending')
            "#,
        )
        .bind(id)
        .bind(listing_id)
        .bind(buyer_id)
        .bind(seller_id)
        .bind(final_price)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Order>, ApiError> {
        let row = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn find_with_details(&self, id: &str) -> Result<Option<OrderSummary>, ApiError> {
        let row = sqlx::query(
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
            WHERE o.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(row.map(|r| OrderSummary {
            id: r.get("id"),
            listing_id: r.get("listing_id"),
            listing_title: r.get("listing_title"),
            buyer_id: r.get("buyer_id"),
            buyer_username: r.get("buyer_username"),
            seller_id: r.get("seller_id"),
            seller_username: r.get("seller_username"),
            final_price: r.get("final_price"),
            status: r.get("status"),
            created_at: r.get("created_at"),
        }))
    }

    async fn list_orders(
        &self,
        user_id: Option<&str>,
        role: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<OrderSummary>, i64), ApiError> {
        let mut where_clause = String::from("WHERE 1=1");
        if let Some(uid) = user_id {
            match role {
                Some("buyer") => where_clause = format!("WHERE o.buyer_id = '{}'", uid),
                Some("seller") => where_clause = format!("WHERE o.seller_id = '{}'", uid),
                _ => {
                    where_clause =
                        format!("WHERE (o.buyer_id = '{}' OR o.seller_id = '{}')", uid, uid)
                }
            }
        }

        let total: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM orders o {}", where_clause))
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let rows = sqlx::query(&format!(
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
            LIMIT $1 OFFSET $2
            "#,
            where_clause
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let items = rows
            .iter()
            .map(|r| OrderSummary {
                id: r.get("id"),
                listing_id: r.get("listing_id"),
                listing_title: r.get("listing_title"),
                buyer_id: r.get("buyer_id"),
                buyer_username: r.get("buyer_username"),
                seller_id: r.get("seller_id"),
                seller_username: r.get("seller_username"),
                final_price: r.get("final_price"),
                status: r.get("status"),
                created_at: r.get("created_at"),
            })
            .collect();

        Ok((items, total))
    }

    async fn update_status(
        &self,
        order_id: &str,
        new_status: &str,
        timestamp_field: &str,
        cancellation_reason: Option<&str>,
    ) -> Result<bool, ApiError> {
        let query = if let Some(reason) = cancellation_reason {
            format!(
                "UPDATE orders SET status = $1, {} = NOW(), cancellation_reason = '{}' WHERE id = $2",
                timestamp_field,
                reason.replace("'", "''")
            )
        } else {
            format!(
                "UPDATE orders SET status = $1, {} = NOW() WHERE id = $2",
                timestamp_field
            )
        };

        let db_result = sqlx::query(&query)
            .bind(new_status)
            .bind(order_id)
            .execute(&self.pool)
            .await;

        let rows = db_result
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .rows_affected();
        Ok(rows > 0)
    }

    async fn count(&self) -> Result<i64, ApiError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(total)
    }
}
