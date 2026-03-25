use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("资源不存在")]
    NotFound,

    #[error("请求错误: {0}")]
    BadRequest(String),

    #[error("未授权")]
    Unauthorized,

    #[error("无权限访问")]
    Forbidden,

    #[error("冲突: {0}")]
    Conflict(String),

    #[error("请求过于频繁，请稍后再试")]
    RateLimitExceeded,

    #[error("服务器内部错误")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "资源不存在".to_string()),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, format!("请求错误: {}", m)),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "请先登录后再操作".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "您没有权限执行此操作".to_string()),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, format!("冲突: {}", m)),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "请求过于频繁，请稍后再试".to_string(),
            ),
            ApiError::Internal(ref e) => {
                // Log the full error for server-side traceability before hiding it from the client.
                tracing::error!(err = %e, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器内部错误，请稍后再试".to_string(),
                )
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_api_error_not_found_status() {
        let error = ApiError::NotFound;
        assert_eq!(error.to_string(), "资源不存在");
    }

    #[test]
    fn test_api_error_bad_request_status() {
        let error = ApiError::BadRequest("输入无效".to_string());
        assert_eq!(error.to_string(), "请求错误: 输入无效");
    }

    #[test]
    fn test_api_error_unauthorized_status() {
        let error = ApiError::Unauthorized;
        assert_eq!(error.to_string(), "未授权");
    }

    #[test]
    fn test_api_error_forbidden_status() {
        let error = ApiError::Forbidden;
        assert_eq!(error.to_string(), "无权限访问");
    }

    #[test]
    fn test_api_error_conflict_status() {
        let error = ApiError::Conflict("用户名已被使用".to_string());
        assert_eq!(error.to_string(), "冲突: 用户名已被使用");
    }

    #[test]
    fn test_api_error_rate_limit_status() {
        let error = ApiError::RateLimitExceeded;
        assert_eq!(error.to_string(), "请求过于频繁，请稍后再试");
    }

    #[test]
    fn test_api_error_into_response_not_found() {
        let error = ApiError::NotFound;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_api_error_into_response_bad_request() {
        let error = ApiError::BadRequest("test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_api_error_into_response_unauthorized() {
        let error = ApiError::Unauthorized;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_api_error_into_response_forbidden() {
        let error = ApiError::Forbidden;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_api_error_into_response_rate_limit() {
        let error = ApiError::RateLimitExceeded;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_api_error_into_response_internal() {
        let error = ApiError::Internal(anyhow::anyhow!("secret error"));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_api_error_conflict_into_response() {
        let error = ApiError::Conflict("resource exists".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
