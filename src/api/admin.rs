use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::api::auth::generate_access_token;
use crate::api::error::ApiError;
use crate::api::AppState;
use crate::middleware::admin::require_admin;

/// Revoke all refresh tokens for a user (used when banning a user).
async fn revoke_all_refresh_tokens(db: &sqlx::PgPool, user_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

/// GET /api/admin/stats - admin marketplace statistics (requires admin role)
pub async fn get_admin_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminStats>, ApiError> {
    let _admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    let total_listings: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM inventory")
        .fetch_one(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let active_listings: i64 =
        sqlx::query("SELECT COUNT(*) as cnt FROM inventory WHERE status = 'active'")
            .fetch_one(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .try_get("cnt")
            .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let total_users: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM users")
        .fetch_one(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    // Orders are disabled - always return 0
    let total_orders: i64 = 0;

    let admin_users: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE role = 'admin'")
        .fetch_one(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse count")))?;

    let category_rows = sqlx::query(
        "SELECT COALESCE(category, 'Other') as category, COUNT(*) as cnt FROM inventory GROUP BY category ORDER BY cnt DESC LIMIT 20",
    )
    .fetch_all(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let categories: Vec<CategoryCount> = category_rows
        .iter()
        .map(|row| CategoryCount {
            category: row.get("category"),
            count: row.try_get("cnt").unwrap_or(0),
        })
        .collect();

    Ok(Json(AdminStats {
        total_listings,
        active_listings,
        total_users,
        total_orders,
        admin_users,
        categories,
    }))
}

#[derive(Serialize)]
pub struct AdminStats {
    pub total_listings: i64,
    pub active_listings: i64,
    pub total_users: i64,
    pub total_orders: i64,
    pub admin_users: i64,
    pub categories: Vec<CategoryCount>,
}

#[derive(Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
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
    let _admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    let users = sqlx::query(
        r#"
        SELECT u.id, u.username, u.role, u.status, u.created_at,
               COUNT(i.id) as listing_count
        FROM users u
        LEFT JOIN inventory i ON u.id = i.owner_id
        GROUP BY u.id, u.username, u.role, u.status, u.created_at
        ORDER BY u.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM users")
        .fetch_one(&state.infra.db)
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
            status: row.get("status"),
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
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub listing_count: i64,
}

/// GET /api/admin/listings - list all listings (requires admin role)
pub async fn get_admin_listings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminListQuery>,
) -> Result<Json<AdminListingsResponse>, ApiError> {
    let _admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    let listings = sqlx::query(
        "SELECT id, title, category, brand, condition_score, suggested_price_cny, description, status, owner_id, created_at FROM inventory ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM inventory")
        .fetch_one(&state.infra.db)
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

/// GET /api/admin/orders - list all orders (DISABLED)
#[allow(dead_code)]
pub async fn get_admin_orders(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<AdminListQuery>,
) -> Result<Json<AdminOrdersResponse>, ApiError> {
    let _admin_id = require_admin(&headers, &_state.secrets.jwt_secret)?;

    tracing::warn!("Admin order list requested but orders are disabled");
    Err(ApiError::BadRequest("订单功能已禁用".to_string()))
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct AdminOrdersResponse {
    pub total: i64,
    pub orders: Vec<OrderInfo>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct OrderInfo {
    pub id: String,
    pub listing_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub final_price: f64,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/admin/users/:user_id/ban - ban a user (requires admin role)
pub async fn ban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_user_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    // Check target user exists and is not an admin
    let user_row = sqlx::query("SELECT id, role FROM users WHERE id = $1")
        .bind(&target_user_id)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let role: String = user_row.get("role");
    if role == "admin" {
        return Err(ApiError::Forbidden);
    }

    // Ban the user
    let banned = sqlx::query(
        "UPDATE users SET status = 'banned' WHERE id = $1 AND status = 'active' RETURNING id",
    )
    .bind(&target_user_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if banned.is_none() {
        return Err(ApiError::BadRequest("用户已被封禁或不存在".to_string()));
    }

    // Revoke all sessions
    revoke_all_refresh_tokens(&state.infra.db, &target_user_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to revoke sessions: {}", e)))?;

    tracing::info!(
        admin_id = %admin_id,
        target_user_id = %target_user_id,
        "Admin banned user"
    );

    Ok(Json(serde_json::json!({ "message": "用户已被封禁" })))
}

/// POST /api/admin/users/:user_id/unban - unban a user (requires admin role)
pub async fn unban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_user_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    let unbanned = sqlx::query(
        "UPDATE users SET status = 'active' WHERE id = $1 AND status = 'banned' RETURNING id",
    )
    .bind(&target_user_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if unbanned.is_none() {
        return Err(ApiError::BadRequest("用户未被封禁或不存在".to_string()));
    }

    tracing::info!(
        admin_id = %admin_id,
        target_user_id = %target_user_id,
        "Admin unbanned user"
    );

    Ok(Json(serde_json::json!({ "message": "用户已解封" })))
}

/// POST /api/admin/listings/:listing_id/takedown - takedown a listing (requires admin role)
pub async fn takedown_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(listing_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    // Takedown the listing
    let takedown =
        sqlx::query("UPDATE inventory SET status = 'takedown' WHERE id = $1 RETURNING id")
            .bind(&listing_id)
            .fetch_optional(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if takedown.is_none() {
        return Err(ApiError::NotFound);
    }

    // Note: Orders are disabled, so we don't cancel pending orders anymore

    tracing::info!(
        admin_id = %admin_id,
        listing_id = %listing_id,
        "Admin takedown listing"
    );

    Ok(Json(serde_json::json!({ "message": "商品已下架" })))
}

/// POST /api/admin/users/:user_id/impersonate - generate a JWT as any user (admin only)
/// WARNING: This logs an audit trail. Use with caution.
pub async fn impersonate_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_user_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    // Fetch target user info
    let row = sqlx::query("SELECT username, role, status FROM users WHERE id = $1")
        .bind(&target_user_id)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let username: String = row.get("username");
    let role: String = row.get("role");
    let status: String = row.get("status");

    // Admins cannot impersonate other admins (security boundary)
    if role == "admin" {
        return Err(ApiError::Forbidden);
    }

    // Generate JWT for target user (shorter TTL: 30 minutes for impersonation)
    let token = generate_access_token(
        &target_user_id,
        &role,
        &state.secrets.jwt_secret,
        1800, // 30 min TTL
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to generate token: {}", e)))?;

    tracing::info!(
        admin_id = %admin_id,
        target_user_id = %target_user_id,
        target_username = %username,
        "Admin impersonated user"
    );

    Ok(Json(serde_json::json!({
        "token": token,
        "user_id": target_user_id,
        "username": username,
        "role": role,
        "status": status,
        "message": "已以该用户身份登录"
    })))
}

/// POST /api/admin/orders/:order_id/status - admin force-sets order status (DISABLED)
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct UpdateOrderStatusRequest {
    pub status: String,
}

#[allow(dead_code)]
pub async fn update_order_status(
    State(_state): State<AppState>,
    headers: HeaderMap,
    Path(_order_id): Path<String>,
    Json(_payload): Json<UpdateOrderStatusRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin_id = require_admin(&headers, &_state.secrets.jwt_secret)?;

    tracing::warn!("Admin order status update requested but orders are disabled");
    Err(ApiError::BadRequest("订单功能已禁用".to_string()))
}

/// POST /api/admin/users/:user_id/role - admin changes user role
#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

pub async fn update_user_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin_id = require_admin(&headers, &state.secrets.jwt_secret)?;

    let valid_roles = ["buyer", "seller"];
    if !valid_roles.contains(&payload.role.as_str()) {
        return Err(ApiError::BadRequest(
            "Invalid role: must be 'buyer' or 'seller'".to_string(),
        ));
    }

    let updated = sqlx::query("UPDATE users SET role = $1 WHERE id = $2 RETURNING id")
        .bind(&payload.role)
        .bind(&user_id)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if updated.is_none() {
        return Err(ApiError::NotFound);
    }

    tracing::info!(
        admin_id = %admin_id,
        target_user_id = %user_id,
        new_role = %payload.role,
        "Admin changed user role"
    );

    Ok(Json(serde_json::json!({ "message": "用户角色已更新" })))
}
