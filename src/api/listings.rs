use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::api::auth::extract_user_id_from_token;
use crate::api::error::ApiError;
use crate::api::AppState;

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ListingQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub category: Option<String>,
    pub search: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Summary view returned by GET /api/listings (browse grid)
#[derive(Serialize)]
pub struct ListingSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: i32,
    pub suggested_price_cny: f64,
    pub status: String,
    pub thumbnail_hint: Option<String>,
}

#[derive(Serialize)]
pub struct ListingsResponse {
    pub items: Vec<ListingSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Full detail returned by GET /api/listings/:id
#[derive(Serialize)]
pub struct ListingDetail {
    pub id: String,
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: i32,
    pub suggested_price_cny: f64,
    pub defects: Vec<String>,
    pub description: Option<String>,
    pub owner_id: String,
    pub owner_username: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// Request body for POST /api/listings
#[derive(Deserialize)]
pub struct CreateListingRequest {
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: i32,
    pub suggested_price_cny: f64,
    pub defects: Vec<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct CreateListingResponse {
    pub id: String,
    pub message: String,
}

/// Parses a JSON-encoded defects array string into a Vec<String>.
fn parse_defects(defects_text: &str) -> Vec<String> {
    serde_json::from_str(defects_text).unwrap_or_else(|_| vec![])
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/listings — public browse with optional category filter + full-text search
pub async fn get_listings(
    State(state): State<AppState>,
    Query(params): Query<ListingQuery>,
) -> Result<Json<ListingsResponse>, ApiError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    let category = params.category.as_ref();
    let search = params.search.as_ref();

    // Count query
    let (count_sql, count_bindings): (&str, Vec<String>) = match (category, search) {
        (Some(cat), Some(srch)) => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE status = 'active' AND category = $1 AND (title ILIKE $2 OR brand ILIKE $2 OR description ILIKE $2)",
            vec![cat.clone(), format!("%{}%", srch)],
        ),
        (Some(cat), None) => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE status = 'active' AND category = $1",
            vec![cat.clone()],
        ),
        (None, Some(srch)) => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE status = 'active' AND (title ILIKE $1 OR brand ILIKE $1 OR description ILIKE $1)",
            vec![format!("%{}%", srch)],
        ),
        (None, None) => (
            "SELECT COUNT(*) as cnt FROM inventory WHERE status = 'active'",
            vec![],
        ),
    };

    let mut count_q = sqlx::query(count_sql);
    for binding in &count_bindings {
        count_q = count_q.bind(binding);
    }
    let count_row = count_q
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    let total: i64 = count_row.try_get("cnt").unwrap_or(0);

    // Items query
    let (items_sql, items_bindings): (&str, Vec<String>) = match (category, search) {
        (Some(cat), Some(srch)) => (
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, status, defects FROM inventory WHERE status = 'active' AND category = $1 AND (title ILIKE $2 OR brand ILIKE $2 OR description ILIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4",
            vec![cat.clone(), format!("%{}%", srch)],
        ),
        (Some(cat), None) => (
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, status, defects FROM inventory WHERE status = 'active' AND category = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            vec![cat.clone()],
        ),
        (None, Some(srch)) => (
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, status, defects FROM inventory WHERE status = 'active' AND (title ILIKE $1 OR brand ILIKE $1 OR description ILIKE $1) ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            vec![format!("%{}%", srch)],
        ),
        (None, None) => (
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, status, defects FROM inventory WHERE status = 'active' ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            vec![],
        ),
    };

    let mut items_q = sqlx::query(items_sql);
    for binding in &items_bindings {
        items_q = items_q.bind(binding);
    }
    items_q = items_q.bind(limit).bind(offset);
    let rows = items_q
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let items: Vec<ListingSummary> = rows
        .iter()
        .map(|row| {
            let defects_text: String = row.get("defects");
            let defects = parse_defects(&defects_text);
            let thumbnail_hint = defects.first().cloned();
            ListingSummary {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                brand: row.get("brand"),
                condition_score: row.get("condition_score"),
                // stored as integer cents, display as yuan
                suggested_price_cny: row.get::<i32, _>("suggested_price_cny") as f64 / 100.0,
                status: row.get("status"),
                thumbnail_hint,
            }
        })
        .collect();

    Ok(Json(ListingsResponse {
        items,
        total,
        limit,
        offset,
    }))
}

/// GET /api/listings/:id — requires auth; returns full listing detail
pub async fn get_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ListingDetail>, ApiError> {
    // Auth required to prevent leaking owner contact info to anonymous users
    let _ = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let row = sqlx::query(
        r#"
        SELECT i.id, i.title, i.category, i.brand, i.condition_score,
               i.suggested_price_cny, i.defects, i.description,
               i.owner_id, i.status, i.created_at,
               u.username as owner_username
        FROM inventory i
        JOIN users u ON i.owner_id = u.id
        WHERE i.id = $1
        "#,
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let defects_text: String = row.get("defects");
    let defects = parse_defects(&defects_text);
    let created_at: String = row
        .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|_| String::new());

    Ok(Json(ListingDetail {
        id: row.get("id"),
        title: row.get("title"),
        category: row.get("category"),
        brand: row.get("brand"),
        condition_score: row.get("condition_score"),
        suggested_price_cny: row.get::<i32, _>("suggested_price_cny") as f64 / 100.0,
        defects,
        description: row.try_get("description").ok(),
        owner_id: row.get("owner_id"),
        owner_username: row.try_get("owner_username").ok(),
        status: row.get("status"),
        created_at,
    }))
}

/// POST /api/listings — auth required; bypasses agent for form-based creation
pub async fn create_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateListingRequest>,
) -> Result<Json<CreateListingResponse>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Validate input
    if payload.title.is_empty() {
        return Err(ApiError::BadRequest("title is required".to_string()));
    }
    if payload.condition_score < 1 || payload.condition_score > 10 {
        return Err(ApiError::BadRequest(
            "condition_score must be between 1 and 10".to_string(),
        ));
    }
    if payload.suggested_price_cny < 0.0 {
        return Err(ApiError::BadRequest(
            "suggested_price_cny cannot be negative".to_string(),
        ));
    }

    let listing_id = Uuid::new_v4().to_string();
    // Convert yuan to cents for storage
    let price_cents = (payload.suggested_price_cny * 100.0).round() as i32;
    let defects_json = serde_json::to_string(&payload.defects)
        .map_err(|e| ApiError::BadRequest(format!("invalid defects: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO inventory (id, title, category, brand, condition_score,
                               suggested_price_cny, defects, description, owner_id, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
        "#,
    )
    .bind(&listing_id)
    .bind(&payload.title)
    .bind(&payload.category)
    .bind(&payload.brand)
    .bind(payload.condition_score)
    .bind(price_cents)
    .bind(&defects_json)
    .bind(&payload.description)
    .bind(&user_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    Ok(Json(CreateListingResponse {
        id: listing_id,
        message: "Listing created successfully".to_string(),
    }))
}

/// DELETE /api/listings/:id - delete a listing (owner only)
pub async fn delete_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Check listing exists and belongs to user
    let row = sqlx::query("SELECT owner_id, status FROM inventory WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

    let owner_id: String = row.get("owner_id");
    let status: String = row.get("status");

    if owner_id != user_id {
        return Err(ApiError::Forbidden);
    }

    if status == "sold" {
        return Err(ApiError::BadRequest(
            "Cannot delete a sold listing".to_string(),
        ));
    }

    // Delete the listing (cascade deletes related records)
    sqlx::query("DELETE FROM inventory WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(listing_id = %id, deleted_by = %user_id, "Listing deleted");

    Ok(Json(serde_json::json!({
        "message": "Listing deleted successfully",
        "id": id
    })))
}

// ---------------------------------------------------------------------------
// Item recognition from image
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct RecognizeRequest {
    pub image_base64: String,
}

#[derive(Serialize, Deserialize)]
pub struct RecognizedItem {
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: i32,
    pub defects: Vec<String>,
    pub description: String,
}

/// POST /api/listings/recognize — auth required; uses Gemini Vision to analyze product image
pub async fn recognize_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RecognizeRequest>,
) -> Result<Json<RecognizedItem>, ApiError> {
    let _ = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    if payload.image_base64.is_empty() {
        return Err(ApiError::BadRequest("image_base64 is required".to_string()));
    }

    // Detect image type from magic bytes
    let mime_type = if let Ok(decoded) = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &payload.image_base64[..payload.image_base64.len().min(50)],
    ) {
        if decoded.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            "image/png"
        } else if decoded.starts_with(&[0xFF, 0xD8, 0xFF]) {
            "image/jpeg"
        } else if decoded.starts_with(b"GIF87a") || decoded.starts_with(b"GIF89a") {
            "image/gif"
        } else {
            "image/jpeg" // fallback
        }
    } else {
        "image/jpeg"
    };

    let prompt = r#"You are a secondhand marketplace listing assistant. Analyze the product image and return a JSON object with the following structure (no markdown, just pure JSON):
{
  "title": "Product name in Chinese, e.g. iPhone 13 Pro Max",
  "category": "One of: electronics, books, digitalAccessories, dailyGoods, clothingShoes, other",
  "brand": "Brand name in Chinese, e.g. Apple",
  "condition_score": 1-10 integer estimate (9=new, 7=good, 5=fair, 3=worn),
  "defects": ["defect1", "defect2"] or empty array,
  "description": "Brief description in Chinese about the item condition and features"
}
Be honest about defects. If you cannot identify the item, return category="other" and generic values."#;

    let request_body = serde_json::json!({
        "contents": [{
            "parts": [
                {"text": prompt},
                {
                    "inline_data": {
                        "mime_type": mime_type,
                        "data": payload.image_base64
                    }
                }
            ]
        }],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 1000
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:generateContent?key={}",
        state.gemini_api_key
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to call Gemini: {}", e)))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to read response: {}", e)))?;

    let parsed: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!(
            "Failed to parse response: {} - {}",
            e,
            response_text
        ))
    })?;

    let json_str = parsed["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!("No text in response: {}", response_text))
        })?
        .trim();

    // Parse the JSON response from Gemini
    let recognized: RecognizedItem = serde_json::from_str(json_str).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!(
            "Failed to parse item JSON: {} - JSON was: {}",
            e,
            json_str
        ))
    })?;

    Ok(Json(recognized))
}
