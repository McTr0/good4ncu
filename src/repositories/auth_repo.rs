//! PostgreSQL implementation of the AuthRepository trait.

use crate::api::error::ApiError;
use crate::repositories::{AuthRepository, User};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

#[derive(Clone)]
#[allow(dead_code)]
pub struct PostgresAuthRepository {
    pool: PgPool,
}

impl PostgresAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AuthRepository for PostgresAuthRepository {
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, ApiError> {
        let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, ApiError> {
        let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(row)
    }

    async fn create_user(
        &self,
        username: &str,
        email: Option<&str>,
        password_hash: &str,
    ) -> Result<String, ApiError> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let result = if let Some(e) = email {
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&user_id)
            .bind(username)
            .bind(e)
            .bind(password_hash)
            .bind("user")
            .execute(&self.pool)
            .await
        } else {
            sqlx::query(
                "INSERT INTO users (id, username, password_hash, role) VALUES ($1, $2, $3, $4)",
            )
            .bind(&user_id)
            .bind(username)
            .bind(password_hash)
            .bind("user")
            .execute(&self.pool)
            .await
        };

        match result {
            Ok(_) => Ok(user_id),
            Err(e) => {
                // PostgreSQL unique violation code = "23505"
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.code().as_deref() == Some("23505") {
                        return Err(ApiError::Conflict(
                            "用户名或邮箱已被使用，请换一个".to_string(),
                        ));
                    }
                }
                Err(ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))
            }
        }
    }

    async fn store_refresh_token(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn find_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<(String, Option<DateTime<Utc>>, DateTime<Utc>)>, ApiError> {
        let row = sqlx::query(
            "SELECT user_id, revoked_at, expires_at FROM refresh_tokens WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        Ok(row.map(|r| {
            let user_id: String = r.get("user_id");
            let revoked_at: Option<DateTime<Utc>> = r.get("revoked_at");
            let expires_at: DateTime<Utc> = r.get("expires_at");
            (user_id, revoked_at, expires_at)
        }))
    }

    async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL")
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }

    async fn revoke_all_user_tokens(&self, user_id: &str) -> Result<(), ApiError> {
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
        Ok(())
    }
}
