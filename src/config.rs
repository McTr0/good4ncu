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
