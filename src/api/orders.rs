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

#[derive(Deserialize)]
pub struct OrderQuery {
    pub role: Option<String>,
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
    pub buyer_username: String,
    pub seller_username: String,
    pub final_price_cny: f64,
    pub status: String,
    pub created_at: String,
    pub role: String,
}

impl From<crate::services::order::OrderSummaryRow> for OrderSummary {
    fn from(r: crate::services::order::OrderSummaryRow) -> Self {
        Self {
            id: r.id,
            listing_id: r.listing_id,
            listing_title: r.listing_title,
            buyer_id: r.buyer_id,
            seller_id: r.seller_id,
            buyer_username: r.buyer_username,
            seller_username: r.seller_username,
            final_price_cny: r.final_price as f64 / 100.0,
            status: r.status,
            created_at: r.created_at.to_rfc3339(),
            role: r.role,
        }
    }
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
    pub paid_at: Option<String>,
    pub shipped_at: Option<String>,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub cancellation_reason: Option<String>,
}

impl From<crate::services::order::OrderRow> for OrderDetail {
    fn from(r: crate::services::order::OrderRow) -> Self {
        Self {
            id: r.id,
            listing_id: r.listing_id,
            listing_title: r.listing_title,
            buyer_id: r.buyer_id,
            seller_id: r.seller_id,
            buyer_username: r.buyer_username,
            seller_username: r.seller_username,
            final_price_cny: r.final_price as f64 / 100.0,
            status: r.status,
            created_at: r.created_at.to_rfc3339(),
            paid_at: r.paid_at.map(|dt| dt.to_rfc3339()),
            shipped_at: r.shipped_at.map(|dt| dt.to_rfc3339()),
            completed_at: r.completed_at.map(|dt| dt.to_rfc3339()),
            cancelled_at: r.cancelled_at.map(|dt| dt.to_rfc3339()),
            cancellation_reason: r.cancellation_reason,
        }
    }
}

/// GET /api/orders - list orders for current user
pub async fn get_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<OrderQuery>,
) -> Result<Json<OrdersResponse>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    let (items, total) = state
        .infra
        .order_service
        .list_orders(&user_id, params.role.as_deref(), limit, offset)
        .await
        .map_err(|e| {
            tracing::error!("get_orders error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to fetch orders"))
        })?;

    let items: Vec<OrderSummary> = items.into_iter().map(OrderSummary::from).collect();

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
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let has_access = state
        .infra
        .order_service
        .verify_order_access(&order_id, &user_id)
        .await
        .map_err(|e| {
            tracing::error!("verify_order_access error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to verify order access"))
        })?;

    if !has_access {
        return Err(ApiError::Forbidden);
    }

    let order = state
        .infra
        .order_service
        .get_order_with_details(&order_id)
        .await
        .map_err(|e| {
            tracing::error!("get_order error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to fetch order"))
        })?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(OrderDetail::from(order)))
}

#[derive(Deserialize)]
pub struct CreateOrderRequest {
    pub listing_id: String,
    pub offered_price_cny: f64,
}

/// POST /api/orders - create an order directly (buyer purchases at offered price)
pub async fn create_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let buyer_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Fetch listing to get seller_id and validate
    let listing_row =
        sqlx::query("SELECT owner_id, suggested_price_cny FROM inventory WHERE id = $1")
            .bind(&payload.listing_id)
            .fetch_optional(&state.infra.db)
            .await
            .map_err(|e| {
                tracing::error!("fetch listing error: {}", e);
                ApiError::Internal(anyhow::anyhow!("Failed to fetch listing"))
            })?;

    let (seller_id, _suggested_price): (String, i64) = match listing_row {
        Some(row) => (row.get("owner_id"), row.get("suggested_price_cny")),
        None => return Err(ApiError::NotFound),
    };

    if seller_id == buyer_id {
        return Err(ApiError::BadRequest(
            "Cannot order your own listing".to_string(),
        ));
    }

    let final_price_cents = (payload.offered_price_cny * 100.0).round() as i64;

    let order_id = state
        .infra
        .order_service
        .create_order(
            &payload.listing_id,
            &buyer_id,
            &seller_id,
            final_price_cents,
        )
        .await
        .map_err(|e| {
            tracing::error!("create_order error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to create order"))
        })?;

    Ok(Json(serde_json::json!({ "id": order_id })))
}

#[derive(Deserialize)]
pub struct OrderActionRequest {
    pub reason: Option<String>,
}

/// Transition helper: verifies access + does status transition atomically.
async fn transition_order(
    state: &AppState,
    order_id: &str,
    user_id: &str,
    expected_current: &str,
    new_status: &str,
    cancellation_reason: Option<&str>,
) -> Result<(), ApiError> {
    // Buyer can: pay (pending→paid), confirm (shipped→completed), cancel (pending/paid→cancelled)
    // Seller can: ship (paid→shipped), cancel (pending/paid→cancelled)

    // Verify access
    let has_access = state
        .infra
        .order_service
        .verify_order_access(order_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("verify_order_access error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to verify order access"))
        })?;

    if !has_access {
        return Err(ApiError::Forbidden);
    }

    // Fetch current status
    let (current_status, _) = state
        .infra
        .order_service
        .get_order_meta(order_id)
        .await
        .map_err(|e| {
            tracing::error!("get_order_meta error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to fetch order status"))
        })?
        .ok_or(ApiError::NotFound)?;

    // Role-based permission check
    let (buyer_id, seller_id) = {
        let row = sqlx::query("SELECT buyer_id, seller_id FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(&state.infra.db)
            .await
            .map_err(|e| {
                tracing::error!("fetch order meta error: {}", e);
                ApiError::Internal(anyhow::anyhow!("Failed to fetch order"))
            })?
            .ok_or(ApiError::NotFound)?;
        (
            row.get::<String, _>("buyer_id"),
            row.get::<String, _>("seller_id"),
        )
    };

    let is_buyer = buyer_id == user_id;
    let is_seller = seller_id == user_id;

    let allowed = matches!(
        (new_status, is_buyer, is_seller),
        ("paid", true, _)
            | ("shipped", _, true)
            | ("completed", true, _)
            | ("cancelled", true, _)
            | ("cancelled", _, true)
    );

    if !allowed {
        return Err(ApiError::Forbidden);
    }

    if current_status != expected_current {
        return Err(ApiError::BadRequest(format!(
            "Order status is '{}', expected '{}'",
            current_status, expected_current
        )));
    }

    let success = state
        .infra
        .order_service
        .transition_order_status(order_id, expected_current, new_status, cancellation_reason)
        .await
        .map_err(|e| {
            tracing::error!("transition_order_status error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to update order status"))
        })?;

    if !success {
        return Err(ApiError::BadRequest("Status transition failed".to_string()));
    }

    Ok(())
}

/// POST /api/orders/:id/pay - buyer pays for order
pub async fn pay_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    transition_order(&state, &order_id, &user_id, "pending", "paid", None).await?;

    Ok(Json(serde_json::json!({ "status": "paid" })))
}

/// POST /api/orders/:id/ship - seller ships order
pub async fn ship_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    transition_order(&state, &order_id, &user_id, "paid", "shipped", None).await?;

    Ok(Json(serde_json::json!({ "status": "shipped" })))
}

/// POST /api/orders/:id/confirm - buyer confirms receipt
pub async fn confirm_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    transition_order(&state, &order_id, &user_id, "shipped", "completed", None).await?;

    Ok(Json(serde_json::json!({ "status": "completed" })))
}

/// POST /api/orders/:id/cancel - buyer or seller cancels order
pub async fn cancel_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(payload): Json<OrderActionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Fetch current status to determine which transition to attempt
    let current_status = state
        .infra
        .order_service
        .get_order_meta(&order_id)
        .await
        .map_err(|e| {
            tracing::error!("get_order_meta error: {}", e);
            ApiError::Internal(anyhow::anyhow!("Failed to fetch order status"))
        })?
        .map(|(s, _)| s)
        .ok_or(ApiError::NotFound)?;

    let target_status = match current_status.as_str() {
        "pending" | "paid" => "cancelled",
        _ => {
            return Err(ApiError::BadRequest(
                "Only pending or paid orders can be cancelled".to_string(),
            ))
        }
    };

    transition_order(
        &state,
        &order_id,
        &user_id,
        &current_status,
        target_status,
        payload.reason.as_deref(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_summary_from_row() {
        let row = crate::services::order::OrderSummaryRow {
            id: "order-1".into(),
            listing_id: "listing-1".into(),
            listing_title: "iPhone 13".into(),
            buyer_id: "buyer-1".into(),
            seller_id: "seller-1".into(),
            final_price: 499900, // 4999.00 CNY in cents
            status: "pending".into(),
            created_at: chrono::Utc::now(),
            role: "buyer".into(),
            buyer_username: "buyeruser".into(),
            seller_username: "selleruser".into(),
        };
        let summary = OrderSummary::from(row);
        assert_eq!(summary.final_price_cny, 4999.0);
        assert_eq!(summary.status, "pending");
    }

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
}
