use crate::api::error::ApiError;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::{extract::Request, middleware::Next};

/// Middleware that enforces admin role for protected routes.
/// Returns 403 Forbidden if the user is not an admin.
///
/// NOTE: This middleware is reserved for future use. Admin protection is currently handled
/// directly in handlers via `require_admin()` function.
#[allow(dead_code)]
pub async fn admin_middleware(mut request: Request, next: Next) -> Response {
    // Extract extensions from request - reserved for future use
    let _ = request.extensions_mut();

    next.run(request).await
}

/// Extract and validate admin role from Authorization header.
/// Returns Ok(admin_id) if admin, Err(ApiError::Forbidden) otherwise.
#[allow(dead_code)]
pub fn require_admin(headers: &HeaderMap, jwt_secret: &str) -> Result<String, ApiError> {
    let (user_id, role) = crate::api::auth::extract_user_id_and_role_from_token_with_fallback(
        headers, jwt_secret, None,
    )
    .map_err(|_| ApiError::Unauthorized)?;

    if role != "admin" {
        tracing::warn!(role = %role, "Non-admin user attempted to access admin endpoint");
        return Err(ApiError::Forbidden);
    }

    Ok(user_id)
}
