use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::middleware::admin::require_admin;

/// GET /api/admin/stats - admin marketplace statistics (requires admin role)
pub async fn get_admin_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminStats>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let total_listings: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM inventory")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let active_listings: i64 =
        sqlx::query("SELECT COUNT(*) as cnt FROM inventory WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .try_get("cnt")
            .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let total_users: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let total_orders: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM orders")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let admin_users: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE role = 'admin'")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    Ok(Json(AdminStats {
        total_listings,
        active_listings,
        total_users,
        total_orders,
        admin_users,
    }))
}

#[derive(Serialize)]
pub struct AdminStats {
    pub total_listings: i64,
    pub active_listings: i64,
    pub total_users: i64,
    pub total_orders: i64,
    pub admin_users: i64,
}

/// Query parameters for admin list endpoints
#[derive(Deserialize)]
pub struct AdminListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

impl AdminListQuery {
    fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).min(100)
    }
    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

/// GET /api/admin/users - list all users (requires admin role)
pub async fn get_admin_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminListQuery>,
) -> Result<Json<AdminUsersResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let users = sqlx::query(
        r#"
        SELECT u.id, u.username, u.role, u.created_at,
               COUNT(i.id) as listing_count
        FROM users u
        LEFT JOIN inventory i ON u.id = i.owner_id
        GROUP BY u.id, u.username, u.role, u.created_at
        ORDER BY u.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let users: Vec<UserInfo> = users
        .iter()
        .map(|row| UserInfo {
            id: row.get("id"),
            username: row.get("username"),
            role: row.get("role"),
            created_at: row.get("created_at"),
            listing_count: row.try_get("listing_count").unwrap_or(0),
        })
        .collect();

    Ok(Json(AdminUsersResponse { total, users }))
}

#[derive(Serialize)]
pub struct AdminUsersResponse {
    pub total: i64,
    pub users: Vec<UserInfo>,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub listing_count: i64,
}

impl UserInfo {
    pub fn joined_at(&self) -> String {
        self.created_at.to_rfc3339()
    }
}

/// GET /api/admin/listings - list all listings (requires admin role)
pub async fn get_admin_listings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminListQuery>,
) -> Result<Json<AdminListingsResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let listings = sqlx::query(
        "SELECT id, title, category, brand, condition_score, suggested_price_cny, description, status, owner_id, created_at FROM inventory ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM inventory")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let listings: Vec<ListingInfo> = listings
        .iter()
        .map(|row| ListingInfo {
            id: row.get("id"),
            title: row.get("title"),
            category: row.get("category"),
            brand: row.get("brand"),
            condition_score: row.get("condition_score"),
            suggested_price_cny: row.get::<i32, _>("suggested_price_cny") as f64 / 100.0,
            description: row.try_get("description").ok(),
            status: row.get("status"),
            owner_id: row.get("owner_id"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(AdminListingsResponse { total, listings }))
}

#[derive(Serialize)]
pub struct AdminListingsResponse {
    pub total: i64,
    pub listings: Vec<ListingInfo>,
}

#[derive(Serialize)]
pub struct ListingInfo {
    pub id: String,
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: i32,
    pub suggested_price_cny: f64,
    pub description: Option<String>,
    pub status: String,
    pub owner_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/orders - list all orders (requires admin role)
pub async fn get_admin_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminListQuery>,
) -> Result<Json<AdminOrdersResponse>, ApiError> {
    require_admin(&headers, &state.jwt_secret)?;

    let orders = sqlx::query(
        "SELECT id, listing_id, buyer_id, seller_id, final_price, status, created_at FROM orders ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM orders")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let orders: Vec<OrderInfo> = orders
        .iter()
        .map(|row| OrderInfo {
            id: row.get("id"),
            listing_id: row.get("listing_id"),
            buyer_id: row.get("buyer_id"),
            seller_id: row.get("seller_id"),
            final_price: row.get("final_price"),
            status: row.get("status"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(AdminOrdersResponse { total, orders }))
}

#[derive(Serialize)]
pub struct AdminOrdersResponse {
    pub total: i64,
    pub orders: Vec<OrderInfo>,
}

#[derive(Serialize)]
pub struct OrderInfo {
    pub id: String,
    pub listing_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub final_price: i64,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
