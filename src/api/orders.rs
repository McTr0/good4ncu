use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token;
use crate::api::error::ApiError;
use crate::api::AppState;
use crate::utils::cents_to_yuan;

#[derive(Deserialize)]
pub struct OrderQuery {
    pub role: Option<String>, // "buyer" | "seller" | "all" (default: all)
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct OrderSummary {
    pub id: String,
    pub listing_id: String,
    pub listing_title: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub final_price_cny: f64,
    pub status: String,
    pub created_at: String,
    pub role: String, // "buyer" or "seller" from current user's perspective
}

#[derive(Serialize)]
pub struct OrdersResponse {
    pub items: Vec<OrderSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Serialize)]
pub struct OrderDetail {
    pub id: String,
    pub listing_id: String,
    pub listing_title: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub buyer_username: String,
    pub seller_username: String,
    pub final_price_cny: f64,
    pub status: String,
    pub created_at: String,
}

/// GET /api/orders - list orders for current user
pub async fn get_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<OrderQuery>,
) -> Result<Json<OrdersResponse>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    let (count_sql, items_sql) = match params.role.as_deref() {
        Some("buyer") => (
            "SELECT COUNT(*) as cnt FROM orders WHERE buyer_id = $1",
            r#"
                SELECT o.id, o.listing_id, o.buyer_id, o.seller_id, o.final_price,
                       o.status, o.created_at, i.title as listing_title,
                       'buyer' as role
                FROM orders o
                JOIN inventory i ON o.listing_id = i.id
                WHERE o.buyer_id = $1
                ORDER BY o.created_at DESC
                LIMIT $2 OFFSET $3
            "#,
        ),
        Some("seller") => (
            "SELECT COUNT(*) as cnt FROM orders WHERE seller_id = $1",
            r#"
                SELECT o.id, o.listing_id, o.buyer_id, o.seller_id, o.final_price,
                       o.status, o.created_at, i.title as listing_title,
                       'seller' as role
                FROM orders o
                JOIN inventory i ON o.listing_id = i.id
                WHERE o.seller_id = $1
                ORDER BY o.created_at DESC
                LIMIT $2 OFFSET $3
            "#,
        ),
        _ => (
            "SELECT COUNT(*) as cnt FROM orders WHERE buyer_id = $1 OR seller_id = $1",
            r#"
                SELECT o.id, o.listing_id, o.buyer_id, o.seller_id, o.final_price,
                       o.status, o.created_at, i.title as listing_title,
                       CASE WHEN o.buyer_id = $1 THEN 'buyer' ELSE 'seller' END as role
                FROM orders o
                JOIN inventory i ON o.listing_id = i.id
                WHERE o.buyer_id = $1 OR o.seller_id = $1
                ORDER BY o.created_at DESC
                LIMIT $2 OFFSET $3
            "#,
        ),
    };

    let count_row = sqlx::query(count_sql)
        .bind(&user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    let total: i64 = count_row.try_get("cnt").unwrap_or(0);

    let rows = sqlx::query(items_sql)
        .bind(&user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let items: Vec<OrderSummary> = rows
        .iter()
        .map(|row| {
            let created_at: String = row
                .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|_| String::new());
            OrderSummary {
                id: row.get("id"),
                listing_id: row.get("listing_id"),
                listing_title: row.get("listing_title"),
                buyer_id: row.get("buyer_id"),
                seller_id: row.get("seller_id"),
                final_price_cny: cents_to_yuan(row.get::<i32, _>("final_price") as i64),
                status: row.get("status"),
                created_at,
                role: row.get("role"),
            }
        })
        .collect();

    Ok(Json(OrdersResponse {
        items,
        total,
        limit,
        offset,
    }))
}

/// GET /api/orders/:id - get order details
pub async fn get_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<OrderDetail>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let row = sqlx::query(
        r#"
        SELECT o.id, o.listing_id, o.buyer_id, o.seller_id, o.final_price,
               o.status, o.created_at, i.title as listing_title,
               b.username as buyer_username, s.username as seller_username
        FROM orders o
        JOIN inventory i ON o.listing_id = i.id
        JOIN users b ON o.buyer_id = b.id
        JOIN users s ON o.seller_id = s.id
        WHERE o.id = $1 AND (o.buyer_id = $2 OR o.seller_id = $2)
        "#,
    )
    .bind(&order_id)
    .bind(&user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let created_at: String = row
        .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|_| String::new());

    Ok(Json(OrderDetail {
        id: row.get("id"),
        listing_id: row.get("listing_id"),
        listing_title: row.get("listing_title"),
        buyer_id: row.get("buyer_id"),
        seller_id: row.get("seller_id"),
        buyer_username: row.get("buyer_username"),
        seller_username: row.get("seller_username"),
        final_price_cny: cents_to_yuan(row.get::<i32, _>("final_price") as i64),
        status: row.get("status"),
        created_at,
    }))
}

#[derive(Deserialize)]
pub struct OrderActionRequest {
    pub reason: Option<String>,
}

/// POST /api/orders/:id/cancel - cancel an order
pub async fn cancel_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(payload): Json<OrderActionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Fetch order and verify ownership
    let order = sqlx::query("SELECT id, buyer_id, seller_id, status FROM orders WHERE id = $1")
        .bind(&order_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let buyer_id: String = order.get("buyer_id");
    let seller_id: String = order.get("seller_id");
    let status: String = order.get("status");

    // Only buyer or seller can cancel
    if buyer_id != user_id && seller_id != user_id {
        return Err(ApiError::Forbidden);
    }

    // Can only cancel pending or paid orders
    if status != "pending" && status != "paid" {
        return Err(ApiError::BadRequest(format!(
            "Cannot cancel order with status '{}'",
            status
        )));
    }

    sqlx::query("UPDATE orders SET status = 'cancelled' WHERE id = $1")
        .bind(&order_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Reactivate the listing
    sqlx::query("UPDATE inventory SET status = 'active' WHERE id = (SELECT listing_id FROM orders WHERE id = $1)")
        .bind(&order_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(
        order_id = %order_id,
        cancelled_by = %user_id,
        reason = ?payload.reason,
        "Order cancelled"
    );

    Ok(Json(serde_json::json!({
        "message": "Order cancelled successfully",
        "order_id": order_id
    })))
}

/// POST /api/orders/:id/confirm - confirm order receipt
pub async fn confirm_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Fetch order and verify buyer is confirming
    let order = sqlx::query("SELECT id, buyer_id, seller_id, status FROM orders WHERE id = $1")
        .bind(&order_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let buyer_id: String = order.get("buyer_id");
    let status: String = order.get("status");

    // Only buyer can confirm receipt
    if buyer_id != user_id {
        return Err(ApiError::Forbidden);
    }

    // Can only confirm paid orders
    if status != "paid" {
        return Err(ApiError::BadRequest(format!(
            "Cannot confirm order with status '{}'. Order must be paid first.",
            status
        )));
    }

    sqlx::query("UPDATE orders SET status = 'completed' WHERE id = $1")
        .bind(&order_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(order_id = %order_id, confirmed_by = %user_id, "Order confirmed");

    Ok(Json(serde_json::json!({
        "message": "Order confirmed successfully",
        "order_id": order_id
    })))
}
