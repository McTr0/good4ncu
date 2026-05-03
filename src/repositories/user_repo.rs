//! PostgreSQL implementation of the UserRepository trait.

use crate::api::error::ApiError;
use crate::repositories::{User, UserProfile, UserRepository};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
#[allow(dead_code)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, ApiError> {
        let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, ApiError> {
        let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, ApiError> {
        let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn create(
        &self,
        username: &str,
        email: Option<&str>,
        password_hash: &str,
        role: &str,
    ) -> Result<String, ApiError> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
            ApiError::Internal(anyhow::anyhow!(
                "Generated user id is not UUID-compatible: {}",
                e
            ))
        })?;
        let result = if let Some(e) = email {
            sqlx::query(
                "INSERT INTO users (id, new_id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&user_id)
            .bind(user_uuid)
            .bind(username)
            .bind(e)
            .bind(password_hash)
            .bind(role)
            .execute(&self.pool)
            .await
        } else {
            sqlx::query(
                "INSERT INTO users (id, new_id, username, password_hash, role) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&user_id)
            .bind(user_uuid)
            .bind(username)
            .bind(password_hash)
            .bind(role)
            .execute(&self.pool)
            .await
        };
        result.map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(user_id)
    }

    async fn get_profile(&self, user_id: &str) -> Result<UserProfile, ApiError> {
        let row = sqlx::query(
            "SELECT username, email, avatar_url, role, created_at FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::NotFound)?;

        let username: String = row.get("username");
        let email: Option<String> = row.get("email");
        let avatar_url: Option<String> = row.get("avatar_url");
        let role: String = row.get("role");
        let created_at: String = row
            .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(UserProfile {
            user_id: user_id.to_string(),
            username,
            email,
            avatar_url,
            role,
            created_at,
        })
    }

    async fn get_user_listings(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
        status_filter: &str,
    ) -> Result<(Vec<crate::repositories::Listing>, i64), ApiError> {
        use crate::repositories::Listing;

        let status_clause = if status_filter == "all" {
            String::new()
        } else {
            format!("AND status = '{}'", status_filter.replace('\'', "''"))
        };

        let query = format!(
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, \
             defects, description, owner_id, status, created_at \
             FROM inventory WHERE owner_id = $1 {} \
             ORDER BY created_at DESC LIMIT {} OFFSET {}",
            status_clause, limit, offset
        );

        let rows = sqlx::query_as::<_, Listing>(&query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let count_query = format!(
            "SELECT COUNT(*) FROM inventory WHERE owner_id = $1 {}",
            status_clause
        );
        let count_row = sqlx::query(&count_query)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let total: i64 = count_row.get(0);
        Ok((rows, total))
    }

    async fn search_users(&self, query: &str, limit: i64) -> Result<Vec<UserProfile>, ApiError> {
        let escaped = query.replace('\'', "''");
        let pattern = format!("%{}%", escaped);

        let rows = sqlx::query(
            "SELECT id, username, email, avatar_url, role, created_at FROM users \
             WHERE username ILIKE $1 LIMIT $2",
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let profiles = rows
            .into_iter()
            .map(|row| {
                let created_at: String = row
                    .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>(
                        "created_at",
                    )
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|_| "unknown".to_string());
                UserProfile {
                    user_id: row.get("id"),
                    username: row.get("username"),
                    email: row.get("email"),
                    avatar_url: row.get("avatar_url"),
                    role: row.get("role"),
                    created_at,
                }
            })
            .collect();

        Ok(profiles)
    }

    async fn search_users_with_listing_count(
        &self,
        query: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<(UserProfile, i64)>, i64), ApiError> {
        let (count_row, rows) = if let Some(q) = query {
            let pattern = format!("%{}%", q.replace('\'', "''"));
            let count_row =
                sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE username ILIKE $1")
                    .bind(&pattern)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
            let rows = sqlx::query(
                r#"
                SELECT u.id as user_id, u.username, u.email, u.avatar_url, u.role, u.created_at,
                       COUNT(i.id) as listing_count
                FROM users u
                LEFT JOIN inventory i ON u.id = i.owner_id AND i.status = 'active'
                WHERE u.username ILIKE $1
                GROUP BY u.id, u.username, u.email, u.avatar_url, u.role, u.created_at
                ORDER BY listing_count DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
            (count_row, rows)
        } else {
            let count_row = sqlx::query("SELECT COUNT(*) as cnt FROM users")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
            let rows = sqlx::query(
                r#"
                SELECT u.id as user_id, u.username, u.email, u.avatar_url, u.role, u.created_at,
                       COUNT(i.id) as listing_count
                FROM users u
                LEFT JOIN inventory i ON u.id = i.owner_id AND i.status = 'active'
                GROUP BY u.id, u.username, u.email, u.avatar_url, u.role, u.created_at
                ORDER BY listing_count DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
            (count_row, rows)
        };

        let total: i64 = count_row.get("cnt");

        let items: Vec<(UserProfile, i64)> = rows
            .iter()
            .map(|row| {
                let created_at: String = row
                    .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>(
                        "created_at",
                    )
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|_| "unknown".to_string());
                let profile = UserProfile {
                    user_id: row.get("user_id"),
                    username: row.get("username"),
                    email: row.get("email"),
                    avatar_url: row.get("avatar_url"),
                    role: row.get("role"),
                    created_at,
                };
                let listing_count: i64 = row.get("listing_count");
                (profile, listing_count)
            })
            .collect();

        Ok((items, total))
    }

    async fn ban_user(&self, user_id: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE users SET status = 'banned' WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn unban_user(&self, user_id: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE users SET status = 'active' WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_role(&self, user_id: &str, role: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_username(&self, user_id: &str, new_username: &str) -> Result<(), ApiError> {
        // Check if username is already taken by another user
        let existing = sqlx::query("SELECT id FROM users WHERE username = $1 AND id != $2")
            .bind(new_username)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        if existing.is_some() {
            return Err(ApiError::Conflict("用户名已被使用".to_string()));
        }

        sqlx::query("UPDATE users SET username = $1 WHERE id = $2")
            .bind(new_username)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn update_avatar(&self, user_id: &str, avatar_url: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
            .bind(avatar_url)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn update_email(&self, user_id: &str, new_email: &str) -> Result<(), ApiError> {
        // Check if email is already taken by another user
        let existing = sqlx::query("SELECT id FROM users WHERE email = $1 AND id != $2")
            .bind(new_email)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        if existing.is_some() {
            return Err(ApiError::Conflict("邮箱已被使用".to_string()));
        }

        sqlx::query("UPDATE users SET email = $1 WHERE id = $2")
            .bind(new_email)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn update_password_hash(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> Result<(), ApiError> {
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(())
    }

    async fn count_users(&self) -> Result<i64, ApiError> {
        let row = sqlx::query("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_infra::with_test_pool;

    #[tokio::test]
    async fn create_dual_writes_shadow_uuid_column() {
        with_test_pool(|pool| async move {
            let repo = PostgresUserRepository::new(pool.clone());

            let user_id = repo
                .create(
                    "shadow_profile_user",
                    Some("profile@example.com"),
                    "hash",
                    "user",
                )
                .await
                .expect("create user");
            let user_uuid = Uuid::parse_str(&user_id).expect("uuid id");

            let row = sqlx::query("SELECT new_id, username, email, role FROM users WHERE id = $1")
                .bind(&user_id)
                .fetch_one(&pool)
                .await
                .expect("select user");

            assert_eq!(row.get::<Uuid, _>("new_id"), user_uuid);
            assert_eq!(row.get::<String, _>("username"), "shadow_profile_user");
            assert_eq!(
                row.get::<Option<String>, _>("email").as_deref(),
                Some("profile@example.com")
            );
            assert_eq!(row.get::<String, _>("role"), "user");
        })
        .await;
    }

    #[tokio::test]
    async fn update_password_hash_persists_new_hash() {
        with_test_pool(|pool| async move {
            let repo = PostgresUserRepository::new(pool.clone());
            let user_id = repo
                .create("password_repo_user", None, "old-hash", "user")
                .await
                .expect("create user");

            repo.update_password_hash(&user_id, "new-hash")
                .await
                .expect("update password hash");

            let password_hash: String =
                sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
                    .bind(&user_id)
                    .fetch_one(&pool)
                    .await
                    .expect("select password hash");
            assert_eq!(password_hash, "new-hash");
        })
        .await;
    }
}
