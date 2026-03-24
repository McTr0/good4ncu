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

        Arc::new(Self {
            gemini_api_key: gemini_api_key.unwrap_or_default(),
            minimax_api_key,
            minimax_api_base_url,
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set in environment"),
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
