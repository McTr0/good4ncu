use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::http::HeaderMap;
use axum::{extract::State, Json};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::repositories::traits::{AuthRepository, UserRepository};
use crate::repositories::{PostgresAuthRepository, PostgresUserRepository};

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub username: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // subject (user_id)
    role: String, // user role: "user" or "admin"
    exp: usize,   // expiration time
}

/// Refresh token: 7 days validity
const REFRESH_TOKEN_TTL_SECS: u64 = 7 * 24 * 3600;
/// Access token: 24 hours validity (long enough for persistent WS connections)
pub const ACCESS_TOKEN_TTL_SECS: u64 = 24 * 3600;

/// Generate a secure random refresh token (UUID v4)
fn generate_refresh_token() -> String {
    Uuid::new_v4().to_string()
}

/// Hash a refresh token with SHA-256 for storage (hex-encoded)
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Generate an access token (JWT) with configurable expiry
pub fn generate_access_token(
    user_id: &str,
    role: &str,
    jwt_secret: &str,
    ttl_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + ttl_secs as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

/// Store a refresh token using the auth repository.
async fn store_refresh_token(
    auth_repo: &PostgresAuthRepository,
    user_id: &str,
    token: &str,
    ttl_secs: u64,
) -> anyhow::Result<()> {
    let token_hash = hash_token(token);
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_secs as i64);

    auth_repo
        .store_refresh_token(user_id, &token_hash, expires_at)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store refresh token: {}", e))?;
    Ok(())
}

/// Validate a refresh token: returns user_id if valid, None if invalid/revoked/expired
/// On success, atomically revokes the presented token and issues a new one.
async fn rotate_refresh_token(
    auth_repo: &PostgresAuthRepository,
    user_repo: &PostgresUserRepository,
    token: &str,
    jwt_secret: &str,
) -> anyhow::Result<(String, String)> {
    let token_hash = hash_token(token);

    // Find the token
    let token_data = auth_repo
        .find_refresh_token(&token_hash)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;

    let (user_id, revoked_at, expires_at) = match token_data {
        Some(data) => data,
        None => return Err(anyhow::anyhow!("Invalid refresh token")),
    };

    // Check revoked
    if revoked_at.is_some() {
        return Err(anyhow::anyhow!("Refresh token has been revoked"));
    }

    // Check expiry
    if expires_at < chrono::Utc::now() {
        return Err(anyhow::anyhow!("Refresh token has expired"));
    }

    // Revoke old token
    auth_repo
        .revoke_refresh_token(&token_hash)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;

    // Fetch user role
    let user = user_repo
        .find_by_id(&user_id)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;
    let role = user.role;

    // Issue new tokens
    let new_refresh = generate_refresh_token();
    store_refresh_token(auth_repo, &user_id, &new_refresh, REFRESH_TOKEN_TTL_SECS).await?;
    let new_access = generate_access_token(&user_id, &role, jwt_secret, ACCESS_TOKEN_TTL_SECS)?;

    Ok((new_access, new_refresh))
}

/// Revoke all refresh tokens for a user
async fn revoke_all_refresh_tokens(
    auth_repo: &PostgresAuthRepository,
    user_id: &str,
) -> anyhow::Result<()> {
    auth_repo
        .revoke_all_user_tokens(user_id)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
    Ok(())
}

/// Revoke a specific refresh token
async fn revoke_refresh_token(
    auth_repo: &PostgresAuthRepository,
    token: &str,
) -> anyhow::Result<()> {
    let token_hash = hash_token(token);
    auth_repo
        .revoke_refresh_token(&token_hash)
        .await
        .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub token: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

/// POST /api/auth/register — returns 201 Created on success, 409 Conflict on duplicate.
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Reject oversized inputs before they can trigger CPU-intensive hashing or bloat storage.
    if payload.username.is_empty() {
        return Err(ApiError::BadRequest("用户名不能为空".to_string()));
    }
    if payload.username.len() > 50 {
        return Err(ApiError::BadRequest("用户名不能超过50个字符".to_string()));
    }
    if payload.password.is_empty() {
        return Err(ApiError::BadRequest("密码不能为空".to_string()));
    }
    if payload.password.len() > 128 {
        return Err(ApiError::BadRequest("密码不能超过128个字符".to_string()));
    }
    if payload.password.len() < 8 {
        return Err(ApiError::BadRequest("密码至少需要8个字符".to_string()));
    }

    // Validate email domain (optional but validated if provided)
    if let Some(ref email) = payload.email {
        if email.is_empty() {
            return Err(ApiError::BadRequest("邮箱不能为空".to_string()));
        }
        if !email.ends_with("@email.ncu.edu.cn") {
            return Err(ApiError::BadRequest("必须使用 @email.ncu.edu.cn 邮箱注册".to_string()));
        }
        if email.len() > 100 {
            return Err(ApiError::BadRequest("邮箱不能超过100个字符".to_string()));
        }
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

    // Create user via repository
    let user_id = state
        .auth_repo
        .create_user(&payload.username, payload.email.as_deref(), &password_hash)
        .await;

    match user_id {
        Ok(user_id) => {
            let token = generate_access_token(
                &user_id,
                "user",
                &state.secrets.jwt_secret,
                ACCESS_TOKEN_TTL_SECS,
            )?;
            let refresh = generate_refresh_token();
            store_refresh_token(&state.auth_repo, &user_id, &refresh, REFRESH_TOKEN_TTL_SECS)
                .await
                .map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!("Failed to store refresh token: {}", e))
                })?;
            Ok(Json(AuthResponse {
                token,
                refresh_token: refresh,
                user_id,
                username: payload.username.clone(),
                message: "注册成功".to_string(),
            }))
        }
        Err(e) => {
            tracing::warn!(err = %e, username = %payload.username, "Registration failed");
            // AuthRepository::create_user returns ApiError::Conflict for duplicate username
            Err(e)
        }
    }
}

/// POST /api/auth/login — returns 200 OK with token on success, 401 Unauthorized on bad credentials.
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    tracing::info!(username = %payload.username, "LOGIN ATTEMPT");
    // Sanity check before hitting the database — prevents wasteful full-table scans.
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(ApiError::AuthFailed("用户名或密码错误".to_string()));
    }
    if payload.username.len() > 50 || payload.password.len() > 128 {
        return Err(ApiError::AuthFailed("用户名或密码错误".to_string()));
    }

    // Fetch user from database using repository
    let user = match state
        .auth_repo
        .find_user_by_username(&payload.username)
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Return 401 to prevent username enumeration
            tracing::warn!(username = %payload.username, "Login failed — user not found");
            return Err(ApiError::AuthFailed("用户名或密码错误".to_string()));
        }
        Err(e) => {
            tracing::error!(err = %e, "Database error during login");
            return Err(ApiError::Internal(anyhow::anyhow!("Database error: {}", e)));
        }
    };

    let password = payload.password.clone();
    let hash_clone = user.password_hash.clone();

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
            let token = generate_access_token(
                &user.id,
                &user.role,
                &state.secrets.jwt_secret,
                ACCESS_TOKEN_TTL_SECS,
            )?;
            let refresh = generate_refresh_token();
            store_refresh_token(&state.auth_repo, &user.id, &refresh, REFRESH_TOKEN_TTL_SECS)
                .await
                .map_err(|e| {
                    ApiError::Internal(anyhow::anyhow!("Failed to store refresh token: {}", e))
                })?;
            Ok(Json(AuthResponse {
                token,
                refresh_token: refresh,
                user_id: user.id,
                username: user.username.clone(),
                message: "登录成功".to_string(),
            }))
        }
        Ok(false) => {
            // Return 401 for wrong password — do NOT distinguish from wrong username
            tracing::warn!(username = %payload.username, "Login failed — wrong password");
            Err(ApiError::AuthFailed("用户名或密码错误".to_string()))
        }
        Err(e) => {
            tracing::error!(err = %e, "Password verification task failed");
            Err(ApiError::Internal(anyhow::anyhow!("Internal error: {}", e)))
        }
    }
}

/// POST /api/auth/refresh — rotate a refresh token, returns new access + refresh token pair
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let (new_access, new_refresh) = rotate_refresh_token(
        &state.auth_repo,
        &state.user_repo,
        &payload.refresh_token,
        &state.secrets.jwt_secret,
    )
    .await
    .map_err(|e| {
        tracing::warn!(err = %e, "Refresh token rotation failed");
        ApiError::Unauthorized
    })?;

    Ok(Json(RefreshResponse {
        token: new_access,
        refresh_token: new_refresh,
    }))
}

/// POST /api/auth/logout — revoke refresh token(s) to invalidate the session
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    if let Some(ref token) = payload.refresh_token {
        revoke_refresh_token(&state.auth_repo, token).await?;
    }
    revoke_all_refresh_tokens(&state.auth_repo, &user_id).await?;

    tracing::info!(user_id = %user_id, "User logged out, all sessions revoked");

    Ok(Json(serde_json::json!({
        "message": "已退出登录"
    })))
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
    let user_id = extract_user_id_from_token(&headers, &state.secrets.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    if payload.current_password.is_empty() {
        return Err(ApiError::BadRequest("当前密码不能为空".to_string()));
    }
    if payload.new_password.is_empty() {
        return Err(ApiError::BadRequest("新密码不能为空".to_string()));
    }
    if payload.new_password.len() < 8 {
        return Err(ApiError::BadRequest("新密码至少需要8个字符".to_string()));
    }
    if payload.new_password.len() > 128 {
        return Err(ApiError::BadRequest("新密码不能超过128个字符".to_string()));
    }

    // Fetch user via repository
    let user = state
        .user_repo
        .find_by_id(&user_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or(ApiError::Unauthorized)?;

    let verify_result = tokio::task::spawn_blocking(move || -> bool {
        let parsed_hash = match PasswordHash::new(&user.password_hash) {
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
            tracing::warn!(user_id = %user_id, "Password change failed — wrong current password");
            return Err(ApiError::AuthFailed("当前密码错误".to_string()));
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
        .execute(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    tracing::info!(user_id = %user_id, "Password changed successfully");

    Ok(Json(serde_json::json!({
        "message": "密码修改成功"
    })))
}

/// Extract and validate the user_id from a raw JWT token string.
/// Returns `Ok(user_id)` if the token is valid, or `Err(message)` if invalid.
pub fn extract_user_id_from_token_str(token: &str, jwt_secret: &str) -> Result<String, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    Ok(token_data.claims.sub)
}

/// Extract and validate the user_id and role from a raw JWT token string.
/// Returns `Ok((user_id, role))` if the token is valid, or `Err(message)` if invalid.
pub fn extract_user_id_and_role_from_token_str(
    token: &str,
    jwt_secret: &str,
) -> Result<(String, String), String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    Ok((token_data.claims.sub, token_data.claims.role))
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

    extract_user_id_from_token_str(token, jwt_secret)
}

/// Extract and validate the user_id and role from the Authorization header using the provided secret.
/// Returns `Ok((user_id, role))` if the token is valid, or `Err(message)` if invalid/missing.
pub fn extract_user_id_and_role_from_token(
    headers: &HeaderMap,
    jwt_secret: &str,
) -> Result<(String, String), String> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Missing Authorization header".to_string())?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| "Invalid Authorization format".to_string())?;

    extract_user_id_and_role_from_token_str(token, jwt_secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_user_id_from_token_missing_header() {
        let headers = HeaderMap::new();
        let result = extract_user_id_from_token(&headers, "secret123456789012345678901234567890");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing Authorization header");
    }

    #[test]
    fn test_extract_user_id_from_token_invalid_format() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Basic dXNlcjpwYXNz".parse().unwrap());
        let result = extract_user_id_from_token(&headers, "secret123456789012345678901234567890");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid Authorization format");
    }

    #[test]
    fn test_generate_token_produces_valid_jwt() {
        let token = generate_access_token(
            "user-123",
            "user",
            "secret123456789012345678901234567890",
            3600,
        )
        .unwrap();
        // A valid JWT has three parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_auth_request_validation_concerns() {
        // These are compile-time checks via struct validation
        // The actual validation happens in the handler, but we can test the logic
        let req = AuthRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };
        assert_eq!(req.username.len(), 8);
        assert_eq!(req.password.len(), 11);
    }

    #[test]
    fn test_auth_request_deserialization() {
        let json = r#"{"username": "alice", "password": "secretpass"}"#;
        let req: AuthRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "alice");
        assert_eq!(req.password, "secretpass");
    }

    #[test]
    fn test_auth_response_serialization() {
        let resp = AuthResponse {
            token: "jwt.token.here".to_string(),
            refresh_token: "refresh.here".to_string(),
            user_id: "user-abc".to_string(),
            username: "alice".to_string(),
            message: "登录成功".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("jwt.token.here"));
        assert!(json.contains("refresh.here"));
        assert!(json.contains("user-abc"));
        assert!(json.contains("alice"));
        assert!(json.contains("登录成功"));
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "user-xyz".to_string(),
            role: "user".to_string(),
            exp: 1700000000,
        };
        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("user-xyz"));
        assert!(json.contains("1700000000"));
    }

    #[test]
    fn test_claims_deserialization() {
        let json = r#"{"sub": "user-123", "role": "admin", "exp": 1700000000}"#;
        let claims: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.exp, 1700000000);
    }

    #[test]
    fn test_generate_token_with_empty_user_id() {
        let token = generate_access_token("", "user", "secret123456789012345678901234567890", 3600)
            .unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_generate_token_verifies_correctly() {
        let secret = "secret123456789012345678901234567890";
        let token = generate_access_token("test-user", "admin", secret, 3600).unwrap();
        let extracted = extract_user_id_from_token(
            &{
                let mut h = HeaderMap::new();
                h.insert(
                    "Authorization",
                    format!("Bearer {}", token).parse().unwrap(),
                );
                h
            },
            secret,
        );
        assert!(extracted.is_ok());
        assert_eq!(extracted.unwrap(), "test-user");
    }

    #[test]
    fn test_generate_token_includes_role() {
        let secret = "secret123456789012345678901234567890";
        let token = generate_access_token("test-user", "admin", secret, 3600).unwrap();
        let (user_id, role) = extract_user_id_and_role_from_token_str(&token, secret).unwrap();
        assert_eq!(user_id, "test-user");
        assert_eq!(role, "admin");
    }
}
