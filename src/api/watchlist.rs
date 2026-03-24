use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token;
use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Serialize)]
pub struct WatchlistItem {
    pub listing_id: String,
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: i32,
    pub suggested_price_cny: f64,
    pub status: String,
    pub owner_id: String,
    pub created_at: String,
}

/// GET /api/watchlist - get user's watchlist
pub async fn get_watchlist(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WatchlistItem>>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let rows = sqlx::query(
        r#"
        SELECT i.id as listing_id, i.title, i.category, i.brand, i.condition_score,
               i.suggested_price_cny, i.status, i.owner_id, i.created_at
        FROM watchlist w
        JOIN inventory i ON w.listing_id = i.id
        WHERE w.user_id = $1
        ORDER BY w.created_at DESC
        "#,
    )
    .bind(&user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let items: Vec<WatchlistItem> = rows
        .iter()
        .map(|row| {
            let created_at: String = row
                .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|_| String::new());
            WatchlistItem {
                listing_id: row.get("listing_id"),
                title: row.get("title"),
                category: row.get("category"),
                brand: row.get("brand"),
                condition_score: row.get("condition_score"),
                suggested_price_cny: row.get::<i32, _>("suggested_price_cny") as f64 / 100.0,
                status: row.get("status"),
                owner_id: row.get("owner_id"),
                created_at,
            }
        })
        .collect();

    Ok(Json(items))
}

/// POST /api/watchlist/:listing_id - add listing to watchlist
pub async fn add_to_watchlist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(listing_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Verify listing exists
    let exists = sqlx::query("SELECT id FROM inventory WHERE id = $1")
        .bind(&listing_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .is_some();

    if !exists {
        return Err(ApiError::NotFound);
    }

    // Insert into watchlist (ignore if already exists)
    sqlx::query(
        "INSERT INTO watchlist (user_id, listing_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(&user_id)
    .bind(&listing_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(Json(serde_json::json!({
        "message": "Added to watchlist",
        "listing_id": listing_id
    })))
}

/// DELETE /api/watchlist/:listing_id - remove listing from watchlist
pub async fn remove_from_watchlist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(listing_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    sqlx::query("DELETE FROM watchlist WHERE user_id = $1 AND listing_id = $2")
        .bind(&user_id)
        .bind(&listing_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(Json(serde_json::json!({
        "message": "Removed from watchlist",
        "listing_id": listing_id
    })))
}
