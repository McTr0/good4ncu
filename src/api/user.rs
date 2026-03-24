use axum::{
    extract::{Path, Query, State},
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

#[derive(Deserialize)]
pub struct UserSearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct UserSummary {
    pub user_id: String,
    pub username: String,
    pub listing_count: i64,
}

#[derive(Serialize)]
pub struct UserSearchResponse {
    pub items: Vec<UserSummary>,
    pub total: i64,
}

/// GET /api/users/search?q=keyword - search/browse users
pub async fn search_users(
    State(state): State<AppState>,
    Query(params): Query<UserSearchQuery>,
) -> Result<Json<UserSearchResponse>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(20).min(50);
    let offset = params.offset.unwrap_or(0).max(0);

    let (count_sql, items_sql, bind_params): (&str, &str, Vec<String>) =
        if let Some(ref q) = params.q {
            let pattern = format!("%{}%", q);
            (
                "SELECT COUNT(*) as cnt FROM users WHERE username ILIKE $1",
                r#"
                SELECT u.id as user_id, u.username,
                       COUNT(i.id) as listing_count
                FROM users u
                LEFT JOIN inventory i ON u.id = i.owner_id AND i.status = 'active'
                WHERE u.username ILIKE $1
                GROUP BY u.id, u.username
                ORDER BY listing_count DESC
                LIMIT $2 OFFSET $3
                "#,
                vec![pattern],
            )
        } else {
            (
                "SELECT COUNT(*) as cnt FROM users",
                r#"
                SELECT u.id as user_id, u.username,
                       COUNT(i.id) as listing_count
                FROM users u
                LEFT JOIN inventory i ON u.id = i.owner_id AND i.status = 'active'
                GROUP BY u.id, u.username
                ORDER BY listing_count DESC
                LIMIT $1 OFFSET $2
                "#,
                vec![],
            )
        };

    let mut count_q = sqlx::query(count_sql);
    for p in &bind_params {
        count_q = count_q.bind(p);
    }
    let count_row = count_q.fetch_one(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB error: {}", e),
        )
    })?;
    let total: i64 = count_row.try_get("cnt").unwrap_or(0);

    let mut items_q = sqlx::query(items_sql);
    for p in &bind_params {
        items_q = items_q.bind(p);
    }
    let rows = items_q
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

    let items: Vec<UserSummary> = rows
        .iter()
        .map(|row| UserSummary {
            user_id: row.get("user_id"),
            username: row.get("username"),
            listing_count: row.get("listing_count"),
        })
        .collect();

    Ok(Json(UserSearchResponse { items, total }))
}

#[derive(Serialize)]
pub struct UserPublicProfile {
    pub user_id: String,
    pub username: String,
    pub listing_count: i64,
    pub joined_at: String,
}

/// GET /api/users/:id - public user profile (no auth required)
pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserPublicProfile>, (StatusCode, String)> {
    let row = sqlx::query(
        r#"
        SELECT u.id as user_id, u.username, u.created_at,
               COUNT(i.id) as listing_count
        FROM users u
        LEFT JOIN inventory i ON u.id = i.owner_id AND i.status = 'active'
        WHERE u.id = $1
        GROUP BY u.id, u.username, u.created_at
        "#,
    )
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

    let created_at: String = row
        .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|_| String::new());

    Ok(Json(UserPublicProfile {
        user_id: row.get("user_id"),
        username: row.get("username"),
        listing_count: row.get("listing_count"),
        joined_at: created_at,
    }))
}
