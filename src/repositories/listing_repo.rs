//! PostgreSQL implementation of the ListingRepository trait.

use crate::api::error::ApiError;
use crate::repositories::{CreateListingInput, Listing, ListingRepository, UpdateListingInput};
use sqlx::{PgPool, Row};

/// Escape special characters for PostgreSQL LIKE patterns.
///
/// The following characters are escaped:
/// - `\` becomes `\\`
/// - `'` becomes `''`
/// - `%` becomes `\%`
/// - `_` becomes `\_`
///
/// This ensures user search input is treated as literal characters in LIKE queries.
pub fn escape_like_pattern(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\'', "''")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct PostgresListingRepository {
    pool: PgPool,
}

impl PostgresListingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ListingRepository for PostgresListingRepository {
    async fn find_listings(
        &self,
        category: Option<&str>,
        categories: Option<&str>,
        search: Option<&str>,
        min_price_cny: Option<f64>,
        max_price_cny: Option<f64>,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Listing>, i64), ApiError> {
        let mut query = String::from(
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, \
             defects, description, owner_id, status, created_at \
             FROM inventory WHERE status = 'active'",
        );
        let mut count_query =
            String::from("SELECT COUNT(*) FROM inventory WHERE status = 'active'");

        // Single category filter (preferred when both are provided)
        if let Some(cat) = category {
            if !cat.is_empty() && cat != "all" && categories.is_none() {
                query = format!("{} AND category = '{}'", query, cat.replace('\'', "''"));
                count_query = format!(
                    "{} AND category = '{}'",
                    count_query,
                    cat.replace('\'', "''")
                );
            }
        }

        // Multi-category: comma-separated, e.g. "electronics,books" -> category IN ('electronics','books')
        if let Some(cats) = categories {
            if !cats.is_empty() && category.is_none() {
                let parts: Vec<String> = cats
                    .split(',')
                    .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                    .collect();
                query = format!("{} AND category IN ({})", query, parts.join(","));
                count_query = format!("{} AND category IN ({})", count_query, parts.join(","));
            }
        }

        if let Some(s) = search {
            if !s.is_empty() {
                // Escape LIKE wildcards: % matches any sequence, _ matches single char
                let escaped = escape_like_pattern(s);
                query = format!(
                    "{} AND (title ILIKE '%{}%' OR description ILIKE '%{}%')",
                    query, escaped, escaped
                );
                count_query = format!(
                    "{} AND (title ILIKE '%{}%' OR description ILIKE '%{}%')",
                    count_query, escaped, escaped
                );
            }
        }

        // Price range filter
        if let Some(min) = min_price_cny {
            if min > 0.0 {
                let min_cents = (min * 100.0).round() as i32;
                query = format!("{} AND suggested_price_cny >= {}", query, min_cents);
                count_query = format!("{} AND suggested_price_cny >= {}", count_query, min_cents);
            }
        }
        if let Some(max) = max_price_cny {
            if max > 0.0 {
                let max_cents = (max * 100.0).round() as i32;
                query = format!("{} AND suggested_price_cny <= {}", query, max_cents);
                count_query = format!("{} AND suggested_price_cny <= {}", count_query, max_cents);
            }
        }

        // Sorting
        let order_by = match sort {
            "price_asc" => "suggested_price_cny ASC",
            "price_desc" => "suggested_price_cny DESC",
            "condition_desc" => "condition_score DESC",
            _ => "created_at DESC", // default: newest
        };
        query = format!(
            "{} ORDER BY {} LIMIT {} OFFSET {}",
            query, order_by, limit, offset
        );

        let rows = sqlx::query_as::<_, Listing>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let count_row = sqlx::query(&count_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let total: i64 = count_row.get(0);
        Ok((rows, total))
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Listing>, ApiError> {
        let row = sqlx::query_as::<_, Listing>(
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, \
             defects, description, owner_id, status, created_at \
             FROM inventory WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn find_by_id_with_owner(
        &self,
        id: &str,
    ) -> Result<Option<(Listing, Option<String>)>, ApiError> {
        let row = sqlx::query(
            "SELECT i.id, i.title, i.category, i.brand, i.condition_score, i.suggested_price_cny, \
             i.defects, i.description, i.owner_id, i.status, i.created_at, \
             u.username as owner_username \
             FROM inventory i \
             LEFT JOIN users u ON i.owner_id = u.id \
             WHERE i.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        match row {
            Some(r) => {
                let listing = Listing {
                    id: r.get("id"),
                    title: r.get("title"),
                    category: r.get("category"),
                    brand: r.get("brand"),
                    condition_score: r.get("condition_score"),
                    suggested_price_cny: r.get("suggested_price_cny"),
                    defects: r.get("defects"),
                    description: r.get("description"),
                    owner_id: r.get("owner_id"),
                    status: r.get("status"),
                    created_at: r.get("created_at"),
                };
                let owner_username: Option<String> = r.get("owner_username");
                Ok(Some((listing, owner_username)))
            }
            None => Ok(None),
        }
    }

    async fn create(&self, input: CreateListingInput) -> Result<String, ApiError> {
        let listing_id = uuid::Uuid::new_v4().to_string();
        let price_cents = (input.suggested_price_cny * 100.0).round() as i32;
        let defects_json = serde_json::to_string(&input.defects)
            .map_err(|e| ApiError::BadRequest(format!("invalid defects: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO inventory (id, title, category, brand, condition_score,
                                   suggested_price_cny, defects, description, owner_id, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
            "#,
        )
        .bind(&listing_id)
        .bind(&input.title)
        .bind(&input.category)
        .bind(&input.brand)
        .bind(input.condition_score)
        .bind(price_cents)
        .bind(&defects_json)
        .bind(&input.description)
        .bind(&input.owner_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(listing_id)
    }

    async fn update(
        &self,
        id: &str,
        owner_id: &str,
        input: UpdateListingInput,
    ) -> Result<(), ApiError> {
        let row = sqlx::query("SELECT owner_id, status FROM inventory WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .ok_or(ApiError::NotFound)?;

        let current_owner: String = row.get("owner_id");
        let current_status: String = row.get("status");

        if current_owner != owner_id {
            return Err(ApiError::Forbidden);
        }
        if current_status == "sold" {
            return Err(ApiError::BadRequest("无法修改已售出的商品".to_string()));
        }

        if input.title.is_none()
            && input.category.is_none()
            && input.brand.is_none()
            && input.condition_score.is_none()
            && input.suggested_price_cny.is_none()
            && input.defects.is_none()
            && input.description.is_none()
            && input.status.is_none()
        {
            return Err(ApiError::BadRequest("没有要更新的字段".to_string()));
        }

        // Build query dynamically based on provided fields
        let mut set_clauses = Vec::new();
        let mut param_idx = 1;
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();

        if let Some(ref v) = input.title {
            set_clauses.push(format!("title = ${}", param_idx));
            params.push(Box::new(v.clone()));
            param_idx += 1;
        }
        if let Some(ref v) = input.category {
            set_clauses.push(format!("category = ${}", param_idx));
            params.push(Box::new(v.clone()));
            param_idx += 1;
        }
        if let Some(ref v) = input.brand {
            set_clauses.push(format!("brand = ${}", param_idx));
            params.push(Box::new(v.clone()));
            param_idx += 1;
        }
        if let Some(v) = input.condition_score {
            set_clauses.push(format!("condition_score = ${}", param_idx));
            params.push(Box::new(v));
            param_idx += 1;
        }
        if let Some(v) = input.suggested_price_cny {
            let cents = (v * 100.0).round() as i32;
            set_clauses.push(format!("suggested_price_cny = ${}", param_idx));
            params.push(Box::new(cents));
            param_idx += 1;
        }
        if let Some(ref v) = input.defects {
            let json = serde_json::to_string(v)
                .map_err(|e| ApiError::BadRequest(format!("invalid defects: {}", e)))?;
            set_clauses.push(format!("defects = ${}", param_idx));
            params.push(Box::new(json));
            param_idx += 1;
        }
        if let Some(ref v) = input.description {
            set_clauses.push(format!("description = ${}", param_idx));
            params.push(Box::new(v.clone()));
            param_idx += 1;
        }
        if let Some(ref v) = input.status {
            set_clauses.push(format!("status = ${}", param_idx));
            params.push(Box::new(v.clone()));
            param_idx += 1;
        }

        let query = format!(
            "UPDATE inventory SET {} WHERE id = ${} AND owner_id = ${}",
            set_clauses.join(", "),
            param_idx,
            param_idx + 1
        );

        // Build and execute query with bindings
        let mut q = sqlx::query(&query);
        if let Some(ref v) = input.title {
            q = q.bind(v);
        }
        if let Some(ref v) = input.category {
            q = q.bind(v);
        }
        if let Some(ref v) = input.brand {
            q = q.bind(v);
        }
        if let Some(v) = input.condition_score {
            q = q.bind(v);
        }
        if let Some(v) = input.suggested_price_cny {
            q = q.bind((v * 100.0).round() as i32);
        }
        if let Some(ref v) = input.defects {
            let defects_json = serde_json::to_string(v)
                .map_err(|e| ApiError::BadRequest(format!("invalid defects: {}", e)))?;
            q = q.bind(defects_json);
        }
        if let Some(ref v) = input.description {
            q = q.bind(v);
        }
        if let Some(ref v) = input.status {
            q = q.bind(v);
        }
        q = q.bind(id).bind(owner_id);

        q.execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: &str, owner_id: &str) -> Result<(), ApiError> {
        let row = sqlx::query("SELECT status FROM inventory WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .ok_or(ApiError::NotFound)?;

        let status: String = row.get("status");
        if status == "sold" {
            return Err(ApiError::BadRequest("无法删除已售出的商品".to_string()));
        }

        sqlx::query("UPDATE inventory SET status = 'deleted' WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn relist(&self, id: &str, owner_id: &str) -> Result<(), ApiError> {
        let row = sqlx::query("SELECT status FROM inventory WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .ok_or(ApiError::NotFound)?;

        let status: String = row.get("status");
        if status != "sold" && status != "deleted" {
            return Err(ApiError::BadRequest(format!(
                "无法重新上架，当前状态为'{}'，只能重新上架已售出或已删除的商品",
                status
            )));
        }

        sqlx::query("UPDATE inventory SET status = 'active' WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn mark_sold(&self, id: &str, owner_id: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE inventory SET status = 'sold' WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn count(&self, status: Option<&str>) -> Result<i64, ApiError> {
        let query = if let Some(s) = status {
            format!(
                "SELECT COUNT(*) FROM inventory WHERE status = '{}'",
                s.replace('\'', "''")
            )
        } else {
            "SELECT COUNT(*) FROM inventory".to_string()
        };

        let row = sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(row.get(0))
    }

    async fn get_category_stats(&self) -> Result<Vec<(String, i64)>, ApiError> {
        let rows = sqlx::query(
            "SELECT COALESCE(category, 'Other') as category, COUNT(*) as cnt \
             FROM inventory GROUP BY category ORDER BY cnt DESC LIMIT 50",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let stats = rows
            .iter()
            .map(|r| (r.get("category"), r.get(1))) // 1 is cnt
            .collect();

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_like_pattern_escapes_backslash() {
        assert_eq!(escape_like_pattern(r"a\b"), r"a\\b");
    }

    #[test]
    fn test_escape_like_pattern_escapes_single_quote() {
        assert_eq!(escape_like_pattern("a'b"), "a''b");
    }

    #[test]
    fn test_escape_like_pattern_escapes_percent() {
        assert_eq!(escape_like_pattern("100%"), r"100\%");
    }

    #[test]
    fn test_escape_like_pattern_escapes_underscore() {
        assert_eq!(escape_like_pattern("a_b"), r"a\_b");
    }

    #[test]
    fn test_escape_like_pattern_escapes_all_special_chars() {
        // Input: a%b_c'd\e
        // - ' -> ''
        // - \ -> \\
        // - % -> \%
        // - _ -> \_
        assert_eq!(escape_like_pattern(r#"a%b_c'd\e"#), r#"a\%b\_c''d\\e"#);
    }

    #[test]
    fn test_escape_like_pattern_100_percent() {
        // "100%" should be escaped to "100\%"
        assert_eq!(escape_like_pattern("100%"), r"100\%");
    }

    #[test]
    fn test_escape_like_pattern_a_b() {
        // "a_b" should be escaped to "a\_b"
        assert_eq!(escape_like_pattern("a_b"), r"a\_b");
    }

    #[test]
    fn test_escape_like_pattern_with_backslash() {
        // "test\" should be escaped to "test\\"
        assert_eq!(escape_like_pattern(r"test\"), r"test\\");
    }

    #[test]
    fn test_escape_like_pattern_empty_string() {
        assert_eq!(escape_like_pattern(""), "");
    }

    #[test]
    fn test_escape_like_pattern_plain_text() {
        // Plain text with no special characters should be unchanged
        assert_eq!(escape_like_pattern("hello world"), "hello world");
    }

    #[test]
    fn test_escape_like_pattern_unicode() {
        // Unicode characters should pass through unchanged
        assert_eq!(escape_like_pattern("你好世界"), "你好世界");
    }

    #[test]
    fn test_escape_like_pattern_emoji() {
        // Emojis should pass through unchanged
        assert_eq!(escape_like_pattern("hello 👋"), "hello 👋");
    }

    #[test]
    fn test_escape_like_pattern_multiple_percent_signs() {
        // Input is "100% off %%%" which is: 100 %   off   % % %
        // After escaping: 100\% off\%\%\%
        assert_eq!(escape_like_pattern("100% off %%%"), "100\\% off \\%\\%\\%");
    }

    #[test]
    fn test_escape_like_pattern_sql_injection_attempt() {
        // Simulate SQL injection-like input: ' ; DROP TABLE users ; --
        // The input has a backslash before the semicolon in the test string
        // Actually the input "'; DROP TABLE users; --" has no backslash
        // After escaping: '' ; DROP TABLE users ; --
        assert_eq!(
            escape_like_pattern("'; DROP TABLE users; --"),
            "''; DROP TABLE users; --"
        );
    }

    #[test]
    fn test_escape_like_pattern_multiple_underscores() {
        assert_eq!(escape_like_pattern("a_b_c_d"), r"a\_b\_c\_d");
    }
}
