use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::http::HeaderMap;
use axum::{extract::State, Json};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // subject (user_id)
    exp: usize,  // expiration time
}

/// Generate a JWT token for the given user_id using the provided secret.
fn generate_token(user_id: &str, jwt_secret: &str) -> String {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + 24 * 3600; // 1 day

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .unwrap_or_default()
}

/// POST /api/auth/register — returns 201 Created on success, 409 Conflict on duplicate.
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Reject oversized inputs before they can trigger CPU-intensive hashing or bloat storage.
    if payload.username.len() > 50 {
        return Err(ApiError::BadRequest(
            "Username must be 50 characters or fewer.".to_string(),
        ));
    }
    if payload.password.len() > 128 {
        return Err(ApiError::BadRequest(
            "Password must be 128 characters or fewer.".to_string(),
        ));
    }

    let password = payload.password.clone();

    // Hash password in a blocking task to avoid starving the tokio runtime
    let hash_result = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await;

    let password_hash = match hash_result {
        Ok(Ok(hash)) => hash,
        Ok(Err(e)) => {
            tracing::error!(err = %e, "Password hashing failed");
            return Err(ApiError::Internal(anyhow::anyhow!(
                "Password hashing failed: {}",
                e
            )));
        }
        Err(e) => {
            tracing::error!(err = %e, "Spawning hashing task failed");
            return Err(ApiError::Internal(anyhow::anyhow!("Internal error: {}", e)));
        }
    };

    let user_id = Uuid::new_v4().to_string();

    // Insert into database — unique constraint on username will trigger CONFLICT
    let result = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(&user_id)
        .bind(&payload.username)
        .bind(&password_hash)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            let token = generate_token(&user_id, &state.jwt_secret);
            Ok(Json(AuthResponse {
                token,
                user_id,
                message: "Registration successful".to_string(),
            }))
        }
        Err(e) => {
            tracing::warn!(err = %e, username = %payload.username, "Registration failed");
            // PostgreSQL unique violation code = "23505"
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.code().as_deref() == Some("23505") {
                    return Err(ApiError::Conflict(
                        "Username already exists. Please choose a different one.".to_string(),
                    ));
                }
            }
            Err(ApiError::Internal(anyhow::anyhow!(
                "Registration failed: {}",
                e
            )))
        }
    }
}

/// POST /api/auth/login — returns 200 OK with token on success, 401 Unauthorized on bad credentials.
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Sanity check before hitting the database — prevents wasteful full-table scans.
    if payload.username.len() > 50 || payload.password.len() > 128 {
        return Err(ApiError::Unauthorized);
    }

    // Fetch user from database
    let user_row = match sqlx::query("SELECT id, password_hash FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            // Return 401 to prevent username enumeration
            tracing::warn!(username = %payload.username, "Login failed — user not found");
            return Err(ApiError::Unauthorized);
        }
        Err(e) => {
            tracing::error!(err = %e, "Database error during login");
            return Err(ApiError::Internal(anyhow::anyhow!("Database error: {}", e)));
        }
    };

    let user_id: String = user_row.get("id");
    let stored_hash: String = user_row.get("password_hash");

    let password = payload.password.clone();
    let hash_clone = stored_hash.clone();

    // Verify password in a blocking task
    let verify_result = tokio::task::spawn_blocking(move || -> bool {
        let parsed_hash = match PasswordHash::new(&hash_clone) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await;

    match verify_result {
        Ok(true) => {
            let token = generate_token(&user_id, &state.jwt_secret);
            Ok(Json(AuthResponse {
                token,
                user_id,
                message: "Login successful".to_string(),
            }))
        }
        Ok(false) => {
            // Return 401 for wrong password — do NOT distinguish from wrong username
            tracing::warn!(username = %payload.username, "Login failed — wrong password");
            Err(ApiError::Unauthorized)
        }
        Err(e) => {
            tracing::error!(err = %e, "Password verification task failed");
            Err(ApiError::Internal(anyhow::anyhow!("Internal error: {}", e)))
        }
    }
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// POST /api/auth/change-password — change password (requires auth)
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    if payload.new_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "New password must be at least 8 characters".to_string(),
        ));
    }

    let user_row = sqlx::query("SELECT password_hash FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::Unauthorized)?;

    let stored_hash: String = user_row.get("password_hash");

    let verify_result = tokio::task::spawn_blocking(move || -> bool {
        let parsed_hash = match PasswordHash::new(&stored_hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(payload.current_password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await;

    match verify_result {
        Ok(true) => {}
        Ok(false) => {
            return Err(ApiError::BadRequest(
                "Current password is incorrect".to_string(),
            ));
        }
        Err(e) => {
            tracing::error!(err = %e, "Password verification task failed");
            return Err(ApiError::Internal(anyhow::anyhow!("Internal error: {}", e)));
        }
    }

    let new_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(payload.new_password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Hashing error: {}", e)))?
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Hashing error: {}", e)))?;

    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&new_hash)
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(user_id = %user_id, "Password changed successfully");

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully"
    })))
}

/// Extract and validate the user_id from the Authorization header using the provided secret.
/// Returns `Ok(user_id)` if the token is valid, or `Err(message)` if invalid/missing.
pub fn extract_user_id_from_token(headers: &HeaderMap, jwt_secret: &str) -> Result<String, String> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Missing Authorization header".to_string())?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| "Invalid Authorization format".to_string())?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    Ok(token_data.claims.sub)
}
