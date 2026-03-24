use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token;
use crate::api::AppState;
use crate::utils::cents_to_yuan;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ListingItem {
    pub id: String,
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: i32,
    pub suggested_price_cny: f64,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct PaginatedListings {
    pub items: Vec<ListingItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/user/profile
pub async fn get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

    let row = sqlx::query("SELECT username, created_at FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let username: String = row.get("username");
    let created_at: String = row
        .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(Json(UserProfile {
        user_id,
        username,
        created_at,
    }))
}

/// GET /api/user/listings?limit=20&offset=0
pub async fn get_user_listings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedListings>, (StatusCode, String)> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0).max(0);

    // Get total count
    let count_row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM inventory WHERE owner_id = $1 AND status = 'active'",
    )
    .bind(&user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB error: {}", e),
        )
    })?;

    let total: i64 = count_row.try_get("cnt").unwrap_or(0);

    // Get paginated items
    let rows = sqlx::query(
        r#"
        SELECT id, title, category, brand, condition_score,
               suggested_price_cny, description, status
        FROM inventory
        WHERE owner_id = $1 AND status = 'active'
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB error: {}", e),
        )
    })?;

    let items: Vec<ListingItem> = rows
        .iter()
        .map(|row| ListingItem {
            id: row.get("id"),
            title: row.get("title"),
            category: row.get("category"),
            brand: row.get("brand"),
            condition_score: row.get("condition_score"),
            suggested_price_cny: cents_to_yuan(row.get::<i32, _>("suggested_price_cny") as i64),
            description: row.try_get("description").ok(),
            status: row.get("status"),
        })
        .collect();

    Ok(Json(PaginatedListings {
        items,
        total,
        limit,
        offset,
    }))
}
