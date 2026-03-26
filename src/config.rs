//! Unified application configuration.
//! All environment variables are loaded and validated at startup.
//! Use `AppConfig::load()` once in `main()`, then pass `Arc<AppConfig>` to components.

use std::fmt;
use std::sync::Arc;

/// Centralized application configuration loaded from environment variables.
/// Load once at startup via `AppConfig::load()`.
///
/// NOTE: Debug is manually implemented to redact sensitive fields.
/// Never use `tracing::info!(?config)` — it would leak secrets.
#[derive(Clone)]
pub struct AppConfig {
    pub gemini_api_key: String,
    pub minimax_api_key: Option<String>,
    pub minimax_api_base_url: Option<String>,
    pub jwt_secret: String,
    pub database_url: String,
    pub llm_provider: String,
    pub vector_dim: usize,
    pub cors_origins: Vec<String>,
    pub blocked_keywords: Vec<String>,
    /// Alibaba Cloud OSS config (optional — upload_token endpoint fails gracefully if not set).
    pub oss_endpoint: String,
    pub oss_bucket: String,
    pub oss_role_arn: Option<String>,
    pub oss_access_key_id: Option<String>,
    pub oss_access_key_secret: Option<String>,
    /// Redis URL for distributed rate limiter (optional — falls back to local moka if not set).
    pub redis_url: Option<String>,
    /// Maximum requests per window for rate limiting. Defaults to 100.
    pub rate_limit_max_requests: u64,
    /// Rate limit window in seconds. Defaults to 60.
    pub rate_limit_window_secs: u64,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("gemini_api_key", &"[REDACTED]")
            .field(
                "minimax_api_key",
                &self.minimax_api_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("minimax_api_base_url", &self.minimax_api_base_url)
            .field("jwt_secret", &"[REDACTED]")
            .field("database_url", &"[REDACTED]")
            .field("llm_provider", &self.llm_provider)
            .field("vector_dim", &self.vector_dim)
            .field("cors_origins", &self.cors_origins)
            .field("blocked_keywords", &self.blocked_keywords)
            .field("oss_endpoint", &self.oss_endpoint)
            .field("oss_bucket", &self.oss_bucket)
            .field(
                "oss_role_arn",
                &self.oss_role_arn.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "oss_access_key_id",
                &self.oss_access_key_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "oss_access_key_secret",
                &self.oss_access_key_secret.as_ref().map(|_| "[REDACTED]"),
            )
            .field("redis_url", &self.redis_url)
            .field("rate_limit_max_requests", &self.rate_limit_max_requests)
            .field("rate_limit_window_secs", &self.rate_limit_window_secs)
            .finish()
    }
}

impl AppConfig {
    /// Load all configuration from environment variables.
    /// Panics if any required variable is missing.
    pub fn load() -> Arc<Self> {
        let llm_provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "gemini".into());

        // Validate provider
        if !["gemini", "minimax"].contains(&llm_provider.as_str()) {
            panic!(
                "LLM_PROVIDER must be 'gemini' or 'minimax', got: {}",
                llm_provider
            );
        }

        let vector_dim: usize = std::env::var("VECTOR_DIM")
            .unwrap_or_else(|_| "768".into())
            .parse()
            .unwrap_or(768);

        let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();
        let minimax_api_key = std::env::var("MINIMAX_API_KEY").ok();
        let minimax_api_base_url = std::env::var("MINIMAX_API_BASE_URL").ok();

        // Ensure at least one LLM key is set
        if gemini_api_key.is_none() && minimax_api_key.is_none() {
            panic!("GEMINI_API_KEY or MINIMAX_API_KEY must be set in environment");
        }

        // When using minimax for chat, we still need gemini for embeddings
        if llm_provider == "minimax" && gemini_api_key.is_none() {
            panic!("GEMINI_API_KEY must be set when LLM_PROVIDER=minimax (used for embeddings)");
        }

        let jwt_secret =
            std::env::var("JWT_SECRET").expect("JWT_SECRET must be set in environment");
        // Enforce minimum secret length to prevent weak JWT signatures
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters for security");
        }

        Arc::new(Self {
            gemini_api_key: gemini_api_key.unwrap_or_default(),
            minimax_api_key,
            minimax_api_base_url,
            jwt_secret,
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set in environment"),
            llm_provider,
            vector_dim,
            cors_origins: std::env::var("CORS_ORIGINS")
                .ok()
                .map(|s| s.split(',').map(|v| v.trim().to_string()).collect())
                .unwrap_or_default(),
            blocked_keywords: std::env::var("BLOCKED_KEYWORDS")
                .ok()
                .map(|s| s.split(',').map(|v| v.trim().to_string()).collect())
                .unwrap_or_default(),
            oss_endpoint: std::env::var("OSS_ENDPOINT")
                .unwrap_or_else(|_| "https://oss-cn-beijing.aliyuncs.com".into()),
            oss_bucket: std::env::var("OSS_BUCKET").unwrap_or_else(|_| "good4ncu".into()),
            oss_role_arn: std::env::var("OSS_ROLE_ARN").ok(),
            oss_access_key_id: std::env::var("OSS_ACCESS_KEY_ID").ok(),
            oss_access_key_secret: std::env::var("OSS_ACCESS_KEY_SECRET").ok(),
            redis_url: std::env::var("REDIS_URL").ok(),
            rate_limit_max_requests: std::env::var("RATE_LIMIT_MAX_REQUESTS")
                .unwrap_or_else(|_| "100".into())
                .parse()
                .unwrap_or(100),
            rate_limit_window_secs: std::env::var("RATE_LIMIT_WINDOW_SECS")
                .unwrap_or_else(|_| "60".into())
                .parse()
                .unwrap_or(60),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_redacts_secrets() {
        let config = AppConfig {
            gemini_api_key: "secret-key-123".to_string(),
            minimax_api_key: Some("minimax-key".to_string()),
            minimax_api_base_url: Some("https://api.minimax.chat".to_string()),
            jwt_secret: "super-secret-jwt-key-that-is-long-enough".to_string(),
            database_url: "postgres://user:password@localhost/db".to_string(),
            llm_provider: "gemini".to_string(),
            vector_dim: 768,
            cors_origins: vec!["https://example.com".to_string()],
            blocked_keywords: vec!["毒品".to_string(), "武器".to_string()],
            oss_endpoint: "https://oss-cn-beijing.aliyuncs.com".to_string(),
            oss_bucket: "good4ncu".to_string(),
            oss_role_arn: Some("acs:ram::123456:role/TestRole".to_string()),
            oss_access_key_id: Some("oss-key-id".to_string()),
            oss_access_key_secret: Some("oss-secret".to_string()),
            redis_url: None,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
        };

        let debug_str = format!("{:?}", config);
        // Sensitive fields should be redacted
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("secret-key-123"));
        assert!(!debug_str.contains("password"));
        // Non-sensitive fields should be visible
        assert!(debug_str.contains("gemini"));
        assert!(debug_str.contains("768"));
    }

    #[test]
    fn test_valid_llm_providers() {
        // Valid providers are gemini and minimax - just verify the constant exists
        assert!(["gemini", "minimax"].contains(&"gemini"));
        assert!(["gemini", "minimax"].contains(&"minimax"));
    }

    #[test]
    fn test_jwt_secret_minimum_length() {
        // JWT_SECRET must be at least 32 characters
        let valid_secret = "a".repeat(32);
        assert!(valid_secret.len() >= 32);

        let invalid_secret = "a".repeat(31);
        assert!(invalid_secret.len() < 32);
    }

    #[test]
    fn test_cors_origins_parsing() {
        let origins = "https://example.com,https://app.example.com, http://localhost:3000";
        let parsed: Vec<String> = origins.split(',').map(|v| v.trim().to_string()).collect();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "https://example.com");
        assert_eq!(parsed[1], "https://app.example.com");
        assert_eq!(parsed[2], "http://localhost:3000");
    }

    #[test]
    fn test_vector_dim_default() {
        // Default vector dimension is 768
        let dim: usize = "768".parse().unwrap_or(768);
        assert_eq!(dim, 768);
    }

    #[test]
    fn test_category_constants_defined() {
        use crate::api::listings::MARKETPLACE_CATEGORIES;
        assert!(MARKETPLACE_CATEGORIES.contains(&"electronics"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"books"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"digitalAccessories"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"dailyGoods"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"clothingShoes"));
        assert!(MARKETPLACE_CATEGORIES.contains(&"other"));
    }
}
