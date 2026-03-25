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
use crate::utils::{cents_to_yuan, yuan_to_cents};

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
    let order =
        sqlx::query("SELECT buyer_id, seller_id, status, listing_id FROM orders WHERE id = $1")
            .bind(&order_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .ok_or(ApiError::NotFound)?;

    let buyer_id: String = order.get("buyer_id");
    let seller_id: String = order.get("seller_id");
    let listing_id: String = order.get("listing_id");
    let status: String = order.get("status");

    // Only buyer or seller can cancel
    if buyer_id != user_id && seller_id != user_id {
        return Err(ApiError::Forbidden);
    }

    // Atomic update prevents race condition; store cancellation reason and timestamp
    let updated = sqlx::query(
        "UPDATE orders SET status = 'cancelled', cancellation_reason = $2, cancelled_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND status IN ('pending', 'paid') RETURNING id",
    )
    .bind(&order_id)
    .bind(&payload.reason)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if updated.is_none() {
        return Err(ApiError::BadRequest(format!(
            "Cannot cancel order with status '{}'",
            status
        )));
    }

    // Reactivate the listing
    sqlx::query("UPDATE inventory SET status = 'active' WHERE id = $1")
        .bind(&listing_id)
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
    let order = sqlx::query("SELECT buyer_id, status FROM orders WHERE id = $1")
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

    // Can only confirm received orders — atomic update enforces the shipped→completed transition.
    // State machine: pending → paid → shipped → completed
    let updated = sqlx::query(
        "UPDATE orders SET status = 'completed', completed_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND status = 'shipped' RETURNING id",
    )
    .bind(&order_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if updated.is_none() {
        return Err(ApiError::BadRequest(format!(
            "Cannot confirm order with status '{}'. Order must be shipped by seller first.",
            status
        )));
    }

    tracing::info!(order_id = %order_id, confirmed_by = %user_id, "Order confirmed");

    Ok(Json(serde_json::json!({
        "message": "Order confirmed successfully",
        "order_id": order_id
    })))
}

/// POST /api/orders/:id/pay - initiate payment for an order (buyer only)
pub async fn pay_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let order = sqlx::query("SELECT buyer_id, status FROM orders WHERE id = $1")
        .bind(&order_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let buyer_id: String = order.get("buyer_id");
    let status: String = order.get("status");

    if buyer_id != user_id {
        return Err(ApiError::Forbidden);
    }

    // Atomic update prevents race condition; record payment timestamp
    let updated = sqlx::query(
        "UPDATE orders SET status = 'paid', paid_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND status = 'pending' RETURNING id",
    )
    .bind(&order_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if updated.is_none() {
        return Err(ApiError::BadRequest(format!(
            "Cannot pay order with status '{}'. Order must be pending.",
            status
        )));
    }

    tracing::info!(order_id = %order_id, paid_by = %user_id, "Order paid");

    Ok(Json(serde_json::json!({
        "message": "Payment initiated successfully",
        "order_id": order_id
    })))
}

/// POST /api/orders/:id/ship - mark order as shipped (seller only)
pub async fn ship_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let order = sqlx::query("SELECT seller_id, listing_id, status FROM orders WHERE id = $1")
        .bind(&order_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let seller_id: String = order.get("seller_id");
    let listing_id: String = order.get("listing_id");
    let status: String = order.get("status");

    if seller_id != user_id {
        return Err(ApiError::Forbidden);
    }

    // Atomic update prevents race condition; record ship timestamp
    let updated = sqlx::query(
        "UPDATE orders SET status = 'shipped', shipped_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND status = 'paid' RETURNING id",
    )
    .bind(&order_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if updated.is_none() {
        return Err(ApiError::BadRequest(format!(
            "Cannot ship order with status '{}'. Order must be paid first.",
            status
        )));
    }

    // Mark listing as sold
    sqlx::query("UPDATE inventory SET status = 'sold' WHERE id = $1")
        .bind(&listing_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(order_id = %order_id, shipped_by = %user_id, "Order shipped");

    Ok(Json(serde_json::json!({
        "message": "Order marked as shipped",
        "order_id": order_id
    })))
}

/// Request body for POST /api/orders (direct order creation, bypassing AI agent)
#[derive(Deserialize)]
pub struct CreateOrderRequest {
    pub listing_id: String,
    pub offered_price_cny: f64,
}

/// POST /api/orders - create an order directly (buyer only, bypassing AI agent)
pub async fn create_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let buyer_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Look up the listing
    let listing = sqlx::query("SELECT owner_id, suggested_price_cny, status FROM inventory WHERE id = $1")
        .bind(&payload.listing_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let owner_id: String = listing.get("owner_id");
    let suggested_price: i64 = listing.get("suggested_price_cny");
    let status: String = listing.get("status");

    if status != "active" {
        return Err(ApiError::BadRequest(format!(
            "Listing is not available (status: {}). Only active listings can be purchased.",
            status
        )));
    }

    if owner_id == buyer_id {
        return Err(ApiError::BadRequest(
            "You cannot purchase your own listing.".to_string(),
        ));
    }

    // Validate offered price is within ±50% of suggested price (same rule as AI agent).
    const PRICE_TOLERANCE: f64 = 0.50;
    let min_price = (suggested_price as f64 * (1.0 - PRICE_TOLERANCE)) as i64;
    let max_price = (suggested_price as f64 * (1.0 + PRICE_TOLERANCE)) as i64;
    let offered_price_cents = yuan_to_cents(payload.offered_price_cny);
    if offered_price_cents < min_price || offered_price_cents > max_price {
        return Err(ApiError::BadRequest(format!(
            "Offered price {:.2} CNY is outside the acceptable range ({:.2} - {:.2} CNY). \
             Seller listed this item at {:.2} CNY.",
            payload.offered_price_cny,
            cents_to_yuan(min_price),
            cents_to_yuan(max_price),
            cents_to_yuan(suggested_price),
        )));
    }

    // Create the order
    let order_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO orders (id, listing_id, buyer_id, seller_id, final_price, status) \
         VALUES ($1, $2, $3, $4, $5, 'pending')",
    )
    .bind(&order_id)
    .bind(&payload.listing_id)
    .bind(&buyer_id)
    .bind(&owner_id)
    .bind(offered_price_cents)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Mark listing as sold immediately (no negotiation phase)
    sqlx::query("UPDATE inventory SET status = 'sold' WHERE id = $1")
        .bind(&payload.listing_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Notify seller that their item was purchased (same notification as AI tool path)
    let _ = state
        .notification
        .create(
            &owner_id,
            "deal_reached",
            "订单已创建",
            &format!("买家已购买商品，成交价 ¥{:.2}", payload.offered_price_cny),
            Some(&order_id),
            Some(&payload.listing_id),
        )
        .await;

    tracing::info!(
        order_id = %order_id,
        listing_id = %payload.listing_id,
        buyer_id = %buyer_id,
        seller_id = %owner_id,
        price = %offered_price_cents,
        "Direct order created (REST API)"
    );

    Ok(Json(serde_json::json!({
        "message": "Order created successfully",
        "order_id": order_id,
        "listing_id": payload.listing_id,
        "price_cny": payload.offered_price_cny,
        "status": "pending"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_query_defaults() {
        let query: OrderQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.role, None);
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn test_order_query_with_filters() {
        let query: OrderQuery =
            serde_json::from_str(r#"{"role": "buyer", "limit": 10, "offset": 20}"#).unwrap();
        assert_eq!(query.role, Some("buyer".to_string()));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(20));
    }

    #[test]
    fn test_order_query_role_values() {
        for role in &["buyer", "seller", "all"] {
            let query: OrderQuery =
                serde_json::from_str(&format!(r#"{{"role": "{}"}}"#, role)).unwrap();
            assert_eq!(query.role, Some(role.to_string()));
        }
    }

    #[test]
    fn test_order_summary_serialization() {
        let summary = OrderSummary {
            id: "order-123".to_string(),
            listing_id: "listing-456".to_string(),
            listing_title: "iPhone 13".to_string(),
            buyer_id: "buyer-1".to_string(),
            seller_id: "seller-1".to_string(),
            final_price_cny: 4999.0,
            status: "pending".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            role: "buyer".to_string(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("order-123"));
        assert!(json.contains("iPhone 13"));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"role\":\"buyer\""));
    }

    #[test]
    fn test_orders_response_serialization() {
        let response = OrdersResponse {
            items: vec![],
            total: 0,
            limit: 20,
            offset: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"items\":[]"));
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"limit\":20"));
        assert!(json.contains("\"offset\":0"));
    }

    #[test]
    fn test_order_detail_serialization() {
        let detail = OrderDetail {
            id: "order-789".to_string(),
            listing_id: "listing-123".to_string(),
            listing_title: "MacBook Pro".to_string(),
            buyer_id: "buyer-2".to_string(),
            seller_id: "seller-3".to_string(),
            buyer_username: "buyeruser".to_string(),
            seller_username: "selleruser".to_string(),
            final_price_cny: 12999.0,
            status: "completed".to_string(),
            created_at: "2024-01-15T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("order-789"));
        assert!(json.contains("MacBook Pro"));
        assert!(json.contains("buyeruser"));
        assert!(json.contains("selleruser"));
        assert!(json.contains("\"status\":\"completed\""));
    }

    #[test]
    fn test_order_action_request_deserialization() {
        let req: OrderActionRequest =
            serde_json::from_str(r#"{"reason": "Changed my mind"}"#).unwrap();
        assert_eq!(req.reason, Some("Changed my mind".to_string()));
    }

    #[test]
    fn test_order_action_request_without_reason() {
        let req: OrderActionRequest = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(req.reason, None);
    }

    #[test]
    fn test_order_query_role_all() {
        let query: OrderQuery = serde_json::from_str(r#"{"role": "all"}"#).unwrap();
        assert_eq!(query.role, Some("all".to_string()));
    }

    #[test]
    fn test_order_summary_seller_role() {
        let summary = OrderSummary {
            id: "order-seller-1".to_string(),
            listing_id: "listing-abc".to_string(),
            listing_title: "Nintendo Switch".to_string(),
            buyer_id: "buyer-x".to_string(),
            seller_id: "seller-y".to_string(),
            final_price_cny: 1999.0,
            status: "shipped".to_string(),
            created_at: "2024-02-01T10:00:00Z".to_string(),
            role: "seller".to_string(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("seller"));
        assert!(json.contains("shipped"));
    }

    #[test]
    fn test_orders_response_with_items() {
        let response = OrdersResponse {
            items: vec![OrderSummary {
                id: "order-1".to_string(),
                listing_id: "l1".to_string(),
                listing_title: "Item 1".to_string(),
                buyer_id: "b1".to_string(),
                seller_id: "s1".to_string(),
                final_price_cny: 100.0,
                status: "pending".to_string(),
                created_at: "2024-01-01".to_string(),
                role: "buyer".to_string(),
            }],
            total: 1,
            limit: 10,
            offset: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Item 1"));
        assert!(json.contains("\"total\":1"));
    }
}
