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

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub role: String,
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
    /// Optional filter: "active", "sold", "deleted", or "all" (default: "active")
    pub status: Option<String>,
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
) -> Result<Json<UserProfile>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let row = sqlx::query("SELECT username, role, created_at FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let username: String = row.get("username");
    let role: String = row.get("role");
    let created_at: String = row
        .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(Json(UserProfile {
        user_id,
        username,
        role,
        created_at,
    }))
}

/// GET /api/user/listings?limit=20&offset=0&status=active
pub async fn get_user_listings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedListings>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0).max(0);
    let status_filter = params.status.as_deref().unwrap_or("active");
    if !["active", "sold", "deleted", "all"].contains(&status_filter) {
        return Err(ApiError::BadRequest(
            "无效的 status 参数，可选值：active, sold, deleted, all".to_string(),
        ));
    }

    let (count_sql, items_sql) = match status_filter {
        "all" => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE owner_id = $1",
            r#"
                SELECT id, title, category, brand, condition_score,
                       suggested_price_cny, description, status
                FROM inventory
                WHERE owner_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
            "#,
        ),
        "active" => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE owner_id = $1 AND status = 'active'",
            r#"
                SELECT id, title, category, brand, condition_score,
                       suggested_price_cny, description, status
                FROM inventory
                WHERE owner_id = $1 AND status = 'active'
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
            "#,
        ),
        "sold" => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE owner_id = $1 AND status = 'sold'",
            r#"
                SELECT id, title, category, brand, condition_score,
                       suggested_price_cny, description, status
                FROM inventory
                WHERE owner_id = $1 AND status = 'sold'
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
            "#,
        ),
        "deleted" => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE owner_id = $1 AND status = 'deleted'",
            r#"
                SELECT id, title, category, brand, condition_score,
                       suggested_price_cny, description, status
                FROM inventory
                WHERE owner_id = $1 AND status = 'deleted'
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
            "#,
        ),
        _ => unreachable!(),
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
) -> Result<Json<UserSearchResponse>, ApiError> {
    let limit = params.limit.unwrap_or(20).min(50);
    let offset = params.offset.unwrap_or(0).max(0);

    // Reject oversized search patterns before they can trigger slow ILIKE scans on large tables.
    if let Some(ref q) = params.q {
        if q.len() > 50 {
            return Err(ApiError::BadRequest(
                "搜索关键词不能超过50个字符".to_string(),
            ));
        }
    }

    let (count_row, rows) = if let Some(ref q) = params.q {
        let pattern = format!("%{}%", q);
        let count_row = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE username ILIKE $1")
            .bind(&pattern)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        let rows = sqlx::query(
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
        )
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        (count_row, rows)
    } else {
        let count_row = sqlx::query("SELECT COUNT(*) as cnt FROM users")
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        let rows = sqlx::query(
            r#"
            SELECT u.id as user_id, u.username,
                   COUNT(i.id) as listing_count
            FROM users u
            LEFT JOIN inventory i ON u.id = i.owner_id AND i.status = 'active'
            GROUP BY u.id, u.username
            ORDER BY listing_count DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        (count_row, rows)
    };

    let total: i64 = count_row.try_get("cnt").unwrap_or(0);

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
) -> Result<Json<UserPublicProfile>, ApiError> {
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
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_defaults() {
        let params: PaginationParams = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(params.limit, None);
        assert_eq!(params.offset, None);
    }

    #[test]
    fn test_pagination_params_with_values() {
        let params: PaginationParams =
            serde_json::from_str(r#"{"limit": 10, "offset": 20}"#).unwrap();
        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(20));
    }

    #[test]
    fn test_user_search_query_defaults() {
        let query: UserSearchQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.q, None);
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn test_user_search_query_with_search() {
        let query: UserSearchQuery = serde_json::from_str(r#"{"q": "john", "limit": 5}"#).unwrap();
        assert_eq!(query.q, Some("john".to_string()));
        assert_eq!(query.limit, Some(5));
    }

    #[test]
    fn test_user_profile_serialization() {
        let profile = UserProfile {
            user_id: "user-123".to_string(),
            username: "testuser".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_listing_item_serialization() {
        let item = ListingItem {
            id: "listing-1".to_string(),
            title: "iPhone 13".to_string(),
            category: "electronics".to_string(),
            brand: "Apple".to_string(),
            condition_score: 8,
            suggested_price_cny: 4999.0,
            description: Some("Good condition".to_string()),
            status: "active".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("iPhone 13"));
        assert!(json.contains("Apple"));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn test_listing_item_without_description() {
        let item = ListingItem {
            id: "listing-2".to_string(),
            title: "Book".to_string(),
            category: "books".to_string(),
            brand: "Publisher".to_string(),
            condition_score: 5,
            suggested_price_cny: 99.0,
            description: None,
            status: "active".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("Book"));
        assert!(json.contains("\"description\":null"));
    }

    #[test]
    fn test_paginated_listings_serialization() {
        let response = PaginatedListings {
            items: vec![],
            total: 0,
            limit: 20,
            offset: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"items\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_user_summary_serialization() {
        let summary = UserSummary {
            user_id: "user-456".to_string(),
            username: "seller1".to_string(),
            listing_count: 10,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("user-456"));
        assert!(json.contains("seller1"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_user_search_response_serialization() {
        let response = UserSearchResponse {
            items: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"items\":[]"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_user_public_profile_serialization() {
        let profile = UserPublicProfile {
            user_id: "user-789".to_string(),
            username: "publicuser".to_string(),
            listing_count: 5,
            joined_at: "2024-01-15T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("user-789"));
        assert!(json.contains("publicuser"));
        assert!(json.contains("\"listing_count\":5"));
        assert!(json.contains("joined_at"));
    }
}
