use axum::extract::State;
use axum::Json;
use serde::Serialize;
use sqlx::Row;

use crate::api::AppState;

#[derive(Serialize)]
pub struct MarketplaceStats {
    pub total_listings: i64,
    pub active_listings: i64,
    pub total_users: i64,
    pub total_orders: i64,
    pub categories: Vec<CategoryCount>,
}

#[derive(Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

/// GET /api/stats - public marketplace statistics
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<MarketplaceStats>, crate::api::error::ApiError> {
    let total_listings: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM inventory")
        .fetch_one(&state.db)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .unwrap_or(0);

    let active_listings: i64 =
        sqlx::query("SELECT COUNT(*) as cnt FROM inventory WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .map_err(|e| crate::api::error::ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .try_get("cnt")
            .unwrap_or(0);

    let total_users: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .unwrap_or(0);

    let total_orders: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM orders")
        .fetch_one(&state.db)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .try_get("cnt")
        .unwrap_or(0);

    let category_rows = sqlx::query(
        "SELECT category, COUNT(*) as cnt FROM inventory WHERE status = 'active' GROUP BY category ORDER BY cnt DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| crate::api::error::ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let categories: Vec<CategoryCount> = category_rows
        .iter()
        .map(|row| CategoryCount {
            category: row.get("category"),
            count: row.get("cnt"),
        })
        .collect();

    Ok(Json(MarketplaceStats {
        total_listings,
        active_listings,
        total_users,
        total_orders,
        categories,
    }))
}
