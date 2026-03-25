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
use crate::utils::cents_to_yuan;

/// Valid marketplace categories for listings.
pub const MARKETPLACE_CATEGORIES: &[&str] = &[
    "electronics",
    "books",
    "digitalAccessories",
    "dailyGoods",
    "clothingShoes",
    "other",
];

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ListingQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Single category filter.
    pub category: Option<String>,
    /// Multiple categories filter, comma-separated (e.g. "electronics,books").
    pub categories: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>, // "newest" (default), "price_asc", "price_desc", "condition_desc"
    /// Minimum price in CNY (inclusive).
    pub min_price_cny: Option<f64>,
    /// Maximum price in CNY (inclusive).
    pub max_price_cny: Option<f64>,
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
    /// First defect description, useful as a quick condition hint for buyers.
    pub defect_hint: Option<String>,
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
    /// Only visible to the listing owner; None for other viewers.
    pub owner_id: Option<String>,
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

#[derive(Deserialize)]
pub struct UpdateListingRequest {
    pub title: Option<String>,
    pub category: Option<String>,
    pub brand: Option<String>,
    pub condition_score: Option<i32>,
    pub suggested_price_cny: Option<f64>,
    pub defects: Option<Vec<String>>,
    pub description: Option<String>,
}

/// Parses a JSON-encoded defects array string into a Vec<String>.
fn parse_defects(defects_text: &str) -> Vec<String> {
    serde_json::from_str(defects_text).unwrap_or_else(|_| vec![])
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/listings — public browse with optional category/categories filter,
/// full-text search, price range, and sort.
pub async fn get_listings(
    State(state): State<AppState>,
    Query(params): Query<ListingQuery>,
) -> Result<Json<ListingsResponse>, ApiError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    // Determine sort order
    let order_by = match params.sort.as_deref() {
        Some("price_asc") => "suggested_price_cny ASC",
        Some("price_desc") => "suggested_price_cny DESC",
        Some("condition_desc") => "condition_score DESC",
        _ => "created_at DESC", // default: newest
    };

    // Build WHERE conditions and collect bound parameter values in the same order.
    let mut conds = vec!["status = 'active'".to_string()];
    let mut binds: Vec<String> = Vec::new(); // tracks type info only, not used for binding

    // Single category (preferred when both are provided)
    let use_single_cat = params.category.is_some() && params.categories.is_none();
    if let Some(ref cat) = params.category {
        if params.categories.is_none() {
            conds.push(format!("category = ${}", binds.len() + 1));
            binds.push(cat.clone());
        }
    }

    // Multi-category: comma-separated, e.g. "electronics,books" → category IN ('electronics','books')
    if let Some(ref cats) = params.categories {
        if !cats.is_empty() && params.category.is_none() {
            let parts: Vec<String> = cats
                .split(',')
                .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                .collect();
            conds.push(format!("category IN ({})", parts.join(",")));
        }
    }

    // Full-text search
    if let Some(ref srch) = params.search {
        if srch.len() > 200 {
            return Err(ApiError::BadRequest(
                "搜索关键词不能超过200个字符".to_string(),
            ));
        }
        if !srch.is_empty() {
            conds.push(format!(
                "(title ILIKE ${} OR brand ILIKE ${} OR description ILIKE ${})",
                binds.len() + 1, binds.len() + 1, binds.len() + 1
            ));
            binds.push(srch.clone());
        }
    }

    // Price range (prices stored as cents, params are in yuan)
    let min_cents = params.min_price_cny.filter(|&p| p > 0.0).map(|p| (p * 100.0) as i32);
    let max_cents = params.max_price_cny.filter(|&p| p > 0.0).map(|p| (p * 100.0) as i32);
    if min_cents.is_some() {
        conds.push(format!("suggested_price_cny >= ${}", binds.len() + 1));
        binds.push("min_price".to_string());
    }
    if max_cents.is_some() {
        conds.push(format!("suggested_price_cny <= ${}", binds.len() + 1));
        binds.push("max_price".to_string());
    }

    let where_clause = conds.join(" AND ");

    // Count query
    let count_sql = format!("SELECT COUNT(*) as cnt FROM inventory WHERE {}", where_clause);
    let mut count_q = sqlx::query(&count_sql);
    if use_single_cat {
        count_q = count_q.bind(params.category.as_deref().unwrap());
    }
    if let Some(ref srch) = params.search {
        if !srch.is_empty() {
            count_q = count_q.bind(format!("%{}%", srch));
        }
    }
    if let Some(mc) = min_cents {
        count_q = count_q.bind(mc);
    }
    if let Some(mc) = max_cents {
        count_q = count_q.bind(mc);
    }

    let count_row = count_q
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    let total: i64 = count_row.get("cnt");

    // Select query
    let select_sql = format!(
        "SELECT id, title, category, brand, condition_score, suggested_price_cny, status, defects \
         FROM inventory WHERE {} ORDER BY {} LIMIT ${} OFFSET ${}",
        where_clause, order_by, binds.len() + 1, binds.len() + 2
    );
    let mut select_q = sqlx::query(&select_sql);
    if use_single_cat {
        select_q = select_q.bind(params.category.as_deref().unwrap());
    }
    if let Some(ref srch) = params.search {
        if !srch.is_empty() {
            select_q = select_q.bind(format!("%{}%", srch));
        }
    }
    if let Some(mc) = min_cents {
        select_q = select_q.bind(mc);
    }
    if let Some(mc) = max_cents {
        select_q = select_q.bind(mc);
    }
    select_q = select_q.bind(limit).bind(offset);

    let rows = select_q
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let items: Vec<ListingSummary> = rows
        .iter()
        .map(|row| {
            let defects_text: String = row.get("defects");
            let defects = parse_defects(&defects_text);
            let defect_hint = defects.first().cloned();
            ListingSummary {
                id: row.get("id"),
                title: row.get("title"),
                category: row.get("category"),
                brand: row.get("brand"),
                condition_score: row.get("condition_score"),
                // stored as integer cents, display as yuan
                suggested_price_cny: cents_to_yuan(row.get::<i32, _>("suggested_price_cny") as i64),
                status: row.get("status"),
                defect_hint,
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

/// GET /api/listings/:id — public; listing info is not sensitive
pub async fn get_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ListingDetail>, ApiError> {
    // Auth optional — guests can browse listing details. The only owner info
    // exposed is username (no email/phone), which is appropriate for a marketplace.
    let viewer_id = extract_user_id_from_token(&headers, &state.jwt_secret).ok();

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
        suggested_price_cny: cents_to_yuan(row.get::<i32, _>("suggested_price_cny") as i64),
        defects,
        description: row.try_get("description").ok(),
        // Only reveal owner_id to the listing owner; everyone else sees None
        owner_id: viewer_id.filter(|vid| vid == row.get::<String, _>("owner_id").as_str()),
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
    if payload.title.len() > 200 {
        return Err(ApiError::BadRequest(
            "title must be 200 characters or fewer".to_string(),
        ));
    }
    if payload.brand.is_empty() {
        return Err(ApiError::BadRequest("brand is required".to_string()));
    }
    if payload.brand.len() > 100 {
        return Err(ApiError::BadRequest(
            "brand must be 100 characters or fewer".to_string(),
        ));
    }
    if !MARKETPLACE_CATEGORIES.contains(&payload.category.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "category must be one of: {}",
            MARKETPLACE_CATEGORIES.join(", ")
        )));
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
    // Max 10 million yuan (100 million cents) — prevents i32 overflow in storage
    if payload.suggested_price_cny > 10_000_000.0 {
        return Err(ApiError::BadRequest(
            "suggested_price_cny cannot exceed 10,000,000 CNY".to_string(),
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
        message: "商品发布成功".to_string(),
    }))
}

/// PUT /api/listings/:id - update a listing (owner only)
pub async fn update_listing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateListingRequest>,
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
            "无法修改已售出的商品".to_string(),
        ));
    }

    // Build dynamic update query
    let mut set_clauses: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();
    let mut param_idx = 1;

    if let Some(ref title) = payload.title {
        if title.is_empty() {
            return Err(ApiError::BadRequest("title cannot be empty".to_string()));
        }
        if title.len() > 200 {
            return Err(ApiError::BadRequest(
                "title must be 200 characters or fewer".to_string(),
            ));
        }
        set_clauses.push(format!("title = ${}", param_idx));
        params.push(title.clone());
        param_idx += 1;
    }
    if let Some(ref category) = payload.category {
        if !MARKETPLACE_CATEGORIES.contains(&category.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "category must be one of: {}",
                MARKETPLACE_CATEGORIES.join(", ")
            )));
        }
        set_clauses.push(format!("category = ${}", param_idx));
        params.push(category.clone());
        param_idx += 1;
    }
    if let Some(ref brand) = payload.brand {
        if brand.is_empty() {
            return Err(ApiError::BadRequest("brand cannot be empty".to_string()));
        }
        if brand.len() > 100 {
            return Err(ApiError::BadRequest(
                "brand must be 100 characters or fewer".to_string(),
            ));
        }
        set_clauses.push(format!("brand = ${}", param_idx));
        params.push(brand.clone());
        param_idx += 1;
    }
    if let Some(score) = payload.condition_score {
        if !(1..=10).contains(&score) {
            return Err(ApiError::BadRequest(
                "condition_score must be between 1 and 10".to_string(),
            ));
        }
        set_clauses.push(format!("condition_score = ${}", param_idx));
        params.push(score.to_string());
        param_idx += 1;
    }
    if let Some(price) = payload.suggested_price_cny {
        if price < 0.0 {
            return Err(ApiError::BadRequest(
                "suggested_price_cny cannot be negative".to_string(),
            ));
        }
        if price > 10_000_000.0 {
            return Err(ApiError::BadRequest(
                "suggested_price_cny cannot exceed 10,000,000 CNY".to_string(),
            ));
        }
        set_clauses.push(format!("suggested_price_cny = ${}", param_idx));
        params.push(((price * 100.0).round() as i32).to_string());
        param_idx += 1;
    }
    if let Some(ref defects) = payload.defects {
        let defects_json = serde_json::to_string(defects)
            .map_err(|e| ApiError::BadRequest(format!("invalid defects: {}", e)))?;
        set_clauses.push(format!("defects = ${}", param_idx));
        params.push(defects_json);
        param_idx += 1;
    }
    if payload.description.is_some() {
        set_clauses.push(format!("description = ${}", param_idx));
        params.push(payload.description.clone().unwrap_or_default());
        param_idx += 1;
    }

    if set_clauses.is_empty() {
        return Err(ApiError::BadRequest("没有要更新的字段".to_string()));
    }

    let sql = format!(
        "UPDATE inventory SET {} WHERE id = ${}",
        set_clauses.join(", "),
        param_idx
    );

    let mut query = sqlx::query(&sql);
    for param in &params {
        query = query.bind(param);
    }
    query = query.bind(&id);

    query
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(listing_id = %id, updated_by = %user_id, "Listing updated");

    Ok(Json(serde_json::json!({
        "message": "商品更新成功",
        "id": id
    })))
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
            "无法删除已售出的商品".to_string(),
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
        "message": "商品已删除",
        "id": id
    })))
}

/// POST /api/listings/:id/relist — reactivate a sold or deleted listing (seller only)
pub async fn relist_listing(
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

    if status == "active" {
        return Err(ApiError::BadRequest(
            "商品已经是上架状态，无需重复操作".to_string(),
        ));
    }

    if status != "sold" && status != "deleted" {
        return Err(ApiError::BadRequest(format!(
            "无法重新上架，当前状态为'{}'，只能重新上架已售出或已删除的商品",
            status
        )));
    }

    // Reactivate the listing
    sqlx::query("UPDATE inventory SET status = 'active' WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(listing_id = %id, relisted_by = %user_id, previous_status = %status, "Listing relisted");

    Ok(Json(serde_json::json!({
        "message": "商品已重新上架",
        "id": id,
        "status": "active"
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

/// GET /api/categories - returns valid marketplace categories
pub async fn get_categories() -> Json<Vec<&'static str>> {
    Json(MARKETPLACE_CATEGORIES.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_categories_defined() {
        assert_eq!(MARKETPLACE_CATEGORIES.len(), 6);
        assert!(MARKETPLACE_CATEGORIES.contains(&"electronics"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"books"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"digitalAccessories"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"dailyGoods"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"clothingShoes"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"other"));
    }

    #[test]
    fn test_valid_category_strings() {
        for cat in MARKETPLACE_CATEGORIES {
            assert!(!cat.is_empty());
            assert!(cat.len() < 50);
        }
    }

    #[test]
    fn test_listing_query_defaults() {
        let query: ListingQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
        assert_eq!(query.category, None);
        assert_eq!(query.search, None);
        assert_eq!(query.sort, None);
    }

    #[test]
    fn test_listing_query_with_sort() {
        let query: ListingQuery = serde_json::from_str(r#"{"sort": "price_asc"}"#).unwrap();
        assert_eq!(query.sort, Some("price_asc".to_string()));

        let query2: ListingQuery = serde_json::from_str(r#"{"sort": "price_desc"}"#).unwrap();
        assert_eq!(query2.sort, Some("price_desc".to_string()));

        let query3: ListingQuery = serde_json::from_str(r#"{"sort": "condition_desc"}"#).unwrap();
        assert_eq!(query3.sort, Some("condition_desc".to_string()));
    }

    #[test]
    fn test_listing_query_with_all_params() {
        let query: ListingQuery = serde_json::from_str(
            r#"{"limit": 10, "offset": 20, "category": "electronics", "search": "iphone", "sort": "newest"}"#,
        )
        .unwrap();
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(20));
        assert_eq!(query.category, Some("electronics".to_string()));
        assert_eq!(query.search, Some("iphone".to_string()));
        assert_eq!(query.sort, Some("newest".to_string()));
    }

    #[test]
    fn test_create_listing_request_deserialization() {
        let json = r#"{
            "title": "iPhone 13",
            "category": "electronics",
            "brand": "Apple",
            "condition_score": 8,
            "suggested_price_cny": 4999.0,
            "defects": ["Minor scratch"],
            "description": "Like new"
        }"#;
        let req: CreateListingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "iPhone 13");
        assert_eq!(req.category, "electronics");
        assert_eq!(req.brand, "Apple");
        assert_eq!(req.condition_score, 8);
        assert_eq!(req.suggested_price_cny, 4999.0);
        assert_eq!(req.defects.len(), 1);
        assert_eq!(req.description, Some("Like new".to_string()));
    }

    #[test]
    fn test_create_listing_request_without_optional_fields() {
        let json = r#"{
            "title": "Book",
            "category": "books",
            "brand": "Publisher",
            "condition_score": 7,
            "suggested_price_cny": 99.0,
            "defects": []
        }"#;
        let req: CreateListingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "Book");
        assert_eq!(req.description, None);
        assert!(req.defects.is_empty());
    }

    #[test]
    fn test_create_listing_response_serialization() {
        let resp = CreateListingResponse {
            id: "listing-123".to_string(),
            message: "商品发布成功".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("listing-123"));
        assert!(json.contains("商品发布成功"));
    }

    #[test]
    fn test_listing_summary_serialization() {
        let summary = ListingSummary {
            id: "listing-456".to_string(),
            title: "MacBook Pro".to_string(),
            category: "electronics".to_string(),
            brand: "Apple".to_string(),
            condition_score: 9,
            suggested_price_cny: 12999.0,
            status: "active".to_string(),
            defect_hint: Some("屏幕有轻微划痕".to_string()),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("MacBook Pro"));
        assert!(json.contains("Apple"));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("12999"));
        assert!(json.contains("defect_hint"));
        assert!(json.contains("屏幕有轻微划痕"));
    }

    #[test]
    fn test_listing_summary_without_defect_hint() {
        let summary = ListingSummary {
            id: "listing-789".to_string(),
            title: "Book".to_string(),
            category: "books".to_string(),
            brand: "Publisher".to_string(),
            condition_score: 5,
            suggested_price_cny: 99.0,
            status: "active".to_string(),
            defect_hint: None,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("Book"));
        assert!(json.contains("\"defect_hint\":null"));
    }

    #[test]
    fn test_listings_response_serialization() {
        let response = ListingsResponse {
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
    fn test_listing_detail_serialization() {
        let detail = ListingDetail {
            id: "listing-detail-1".to_string(),
            title: "iPhone 15".to_string(),
            category: "electronics".to_string(),
            brand: "Apple".to_string(),
            condition_score: 10,
            suggested_price_cny: 7999.0,
            defects: vec!["None".to_string()],
            description: Some("Brand new".to_string()),
            owner_id: Some("user-owner".to_string()),
            owner_username: Some("seller1".to_string()),
            status: "active".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("iPhone 15"));
        assert!(json.contains("seller1"));
        assert!(json.contains("\"defects\":[\"None\"]"));
    }

    #[test]
    fn test_update_listing_request_deserialization() {
        let json = r#"{"title": "Updated Title", "description": "New description"}"#;
        let req: UpdateListingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, Some("Updated Title".to_string()));
        assert_eq!(req.description, Some("New description".to_string()));
        assert_eq!(req.category, None);
        assert_eq!(req.brand, None);
    }

    #[test]
    fn test_update_listing_request_partial() {
        let json = r#"{"suggested_price_cny": 4500.0}"#;
        let req: UpdateListingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.suggested_price_cny, Some(4500.0));
        assert_eq!(req.title, None);
        assert_eq!(req.description, None);
    }

    #[test]
    fn test_update_listing_request_all_fields() {
        let json = r#"{
            "title": "New Title",
            "category": "electronics",
            "brand": "Apple",
            "condition_score": 9,
            "suggested_price_cny": 5999.0,
            "defects": ["Scratched"],
            "description": "Updated desc"
        }"#;
        let req: UpdateListingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, Some("New Title".to_string()));
        assert_eq!(req.category, Some("electronics".to_string()));
        assert_eq!(req.brand, Some("Apple".to_string()));
        assert_eq!(req.condition_score, Some(9));
        assert_eq!(req.suggested_price_cny, Some(5999.0));
        assert_eq!(req.defects, Some(vec!["Scratched".to_string()]));
        assert_eq!(req.description, Some("Updated desc".to_string()));
    }
}
