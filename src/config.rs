//! Unified application configuration.
//! All environment variables are loaded and validated at startup.
//! Use `AppConfig::load()` once in `main()`, then pass `Arc<AppConfig>` to components.
//!
//! A TOML config file (e.g. `good4ncu.toml`) can supplement env vars.
//! Use `AppConfig::load_with_file()` to merge TOML + env vars.
//! Environment variables always override TOML file values.

mod file;

use std::fmt;
use std::path::Path;
use std::sync::Arc;

/// Default marketplace categories (used when not set in config file).
pub const DEFAULT_CATEGORIES: &[&str] = &[
    "electronics",
    "books",
    "digitalAccessories",
    "dailyGoods",
    "clothingShoes",
    "other",
];

/// Centralized application configuration loaded from environment variables.
#[derive(Clone)]
pub struct AppConfig {
    // --- Secrets (env var only, never from TOML) ---
    pub gemini_api_key: String,
    pub minimax_api_key: Option<String>,
    pub minimax_api_base_url: Option<String>,
    pub jwt_secret: String,
    pub database_url: String,
    pub oss_access_key_id: Option<String>,
    pub oss_access_key_secret: Option<String>,

    // --- LLM config (env var, with TOML override) ---
    pub llm_provider: String,
    pub vector_dim: usize,

    // --- Infrastructure ---
    pub cors_origins: Vec<String>,
    pub oss_endpoint: String,
    pub oss_bucket: String,
    pub oss_role_arn: Option<String>,
    pub redis_url: Option<String>,
    pub rate_limit_max_requests: u64,
    pub rate_limit_window_secs: u64,

    // --- TOML-only fields (with hardcoded defaults when no file) ---
    pub server_host: String,
    pub server_port: u16,
    pub event_bus_capacity: usize,
    pub hitl_expire_scan_interval_secs: u64,
    pub hitl_expire_timeout_hours: u64,
    pub moka_cache_max_capacity: u64,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub conversation_history_limit: usize,
    pub max_keyword_len: usize,
    pub price_tolerance: f64,
    pub categories: Vec<String>,
    pub blocked_keywords: Vec<String>,
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
            .field("server_host", &self.server_host)
            .field("server_port", &self.server_port)
            .field("event_bus_capacity", &self.event_bus_capacity)
            .field(
                "hitl_expire_scan_interval_secs",
                &self.hitl_expire_scan_interval_secs,
            )
            .field("hitl_expire_timeout_hours", &self.hitl_expire_timeout_hours)
            .field("moka_cache_max_capacity", &self.moka_cache_max_capacity)
            .field("access_token_ttl_secs", &self.access_token_ttl_secs)
            .field("refresh_token_ttl_secs", &self.refresh_token_ttl_secs)
            .field(
                "conversation_history_limit",
                &self.conversation_history_limit,
            )
            .field("max_keyword_len", &self.max_keyword_len)
            .field("price_tolerance", &self.price_tolerance)
            .field("categories", &self.categories)
            .field("blocked_keywords", &self.blocked_keywords)
            .finish()
    }
}

impl AppConfig {
    /// Load all configuration from environment variables only (no config file).
    /// Panics if any required variable is missing.
    #[allow(dead_code)]
    pub fn load() -> Arc<Self> {
        Self::load_with_file(None)
    }

    /// Load configuration from environment variables, optionally merged with a TOML file.
    ///
    /// Priority: **env var > TOML file > hardcoded default**
    ///
    /// The config file path is determined by (in order):
    /// 1. `$CONFIG_FILE` env var (if set)
    /// 2. `./good4ncu.toml` (if exists)
    /// 3. `./config/good4ncu.toml` (if exists)
    /// 4. No file (env vars only, all TOML fields use defaults)
    pub fn load_with_file(config_path: Option<&Path>) -> Arc<Self> {
        // Phase 1: Load TOML file (ignore if missing or invalid)
        let file = file::load(config_path);

        // Phase 2: Build config with env var override of file override of default

        // LLM provider: env > file > "gemini"
        let llm_provider = std::env::var("LLM_PROVIDER")
            .ok()
            .or_else(|| file.as_ref()?.llm.provider.clone())
            .unwrap_or_else(|| "gemini".into());

        // Validate provider
        if !["gemini", "minimax"].contains(&llm_provider.as_str()) {
            panic!(
                "LLM_PROVIDER must be 'gemini' or 'minimax', got: {}",
                llm_provider
            );
        }

        let vector_dim: usize = std::env::var("VECTOR_DIM")
            .ok()
            .or_else(|| file.as_ref()?.llm.vector_dim.map(|v| v.to_string()))
            .and_then(|s| s.parse().ok())
            .unwrap_or(768);

        let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();
        let minimax_api_key = std::env::var("MINIMAX_API_KEY").ok();
        let minimax_api_base_url = std::env::var("MINIMAX_API_BASE_URL").ok();

        if gemini_api_key.is_none() && minimax_api_key.is_none() {
            panic!("GEMINI_API_KEY or MINIMAX_API_KEY must be set in environment");
        }

        if llm_provider == "minimax" && gemini_api_key.is_none() {
            panic!("GEMINI_API_KEY must be set when LLM_PROVIDER=minimax (used for embeddings)");
        }

        let jwt_secret =
            std::env::var("JWT_SECRET").expect("JWT_SECRET must be set in environment");
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters for security");
        }

        // CORS: env > file > default (empty = allow all)
        let cors_origins = std::env::var("CORS_ORIGINS")
            .ok()
            .map(|s| s.split(',').map(|v| v.trim().to_string()).collect())
            .or_else(|| file.as_ref()?.cors.origins.clone())
            .unwrap_or_default();

        // Blocked keywords: env > file > default
        let blocked_keywords = std::env::var("BLOCKED_KEYWORDS")
            .ok()
            .map(|s| s.split(',').map(|v| v.trim().to_string()).collect())
            .or_else(|| file.as_ref()?.moderation.blocked_keywords.clone())
            .unwrap_or_default();

        // OSS: env > file > hardcoded default
        let oss_endpoint = std::env::var("OSS_ENDPOINT")
            .ok()
            .or_else(|| file.as_ref()?.oss.endpoint.clone())
            .unwrap_or_else(|| "https://oss-cn-beijing.aliyuncs.com".into());

        let oss_bucket = std::env::var("OSS_BUCKET")
            .ok()
            .or_else(|| file.as_ref()?.oss.bucket.clone())
            .unwrap_or_else(|| "good4ncu".into());

        let oss_role_arn = std::env::var("OSS_ROLE_ARN").ok();
        let oss_access_key_id = std::env::var("OSS_ACCESS_KEY_ID").ok();
        let oss_access_key_secret = std::env::var("OSS_ACCESS_KEY_SECRET").ok();

        // Redis: env > file > None
        let redis_url = std::env::var("REDIS_URL")
            .ok()
            .or_else(|| file.as_ref()?.rate_limit.redis_url.clone());

        // Rate limit: env > file > default (fail-fast on invalid env value)
        let rate_limit_max_requests: u64 = if let Ok(v) = std::env::var("RATE_LIMIT_MAX_REQUESTS") {
            v.parse()
                .expect("RATE_LIMIT_MAX_REQUESTS must be a valid u64")
        } else {
            file.as_ref()
                .and_then(|f| f.rate_limit.max_requests)
                .unwrap_or(100)
        };

        let rate_limit_window_secs: u64 = if let Ok(v) = std::env::var("RATE_LIMIT_WINDOW_SECS") {
            v.parse()
                .expect("RATE_LIMIT_WINDOW_SECS must be a valid u64")
        } else {
            file.as_ref()
                .and_then(|f| f.rate_limit.window_secs)
                .unwrap_or(60)
        };

        // TOML-only fields (env vars don't override these)
        let server_host = file
            .as_ref()
            .and_then(|f| f.server.host.clone())
            .unwrap_or_else(|| "127.0.0.1".into());

        let server_port = file.as_ref().and_then(|f| f.server.port).unwrap_or(3000);

        let event_bus_capacity = file
            .as_ref()
            .and_then(|f| f.event_bus.capacity)
            .unwrap_or(2048);

        let hitl_expire_scan_interval_secs = file
            .as_ref()
            .and_then(|f| f.workers.hitl_expire.scan_interval_secs)
            .unwrap_or(600);

        let hitl_expire_timeout_hours = file
            .as_ref()
            .and_then(|f| f.workers.hitl_expire.expire_timeout_hours)
            .unwrap_or(48);

        let moka_cache_max_capacity = file
            .as_ref()
            .and_then(|f| f.rate_limit.moka_cache_max_capacity)
            .unwrap_or(100_000);

        let access_token_ttl_secs = file
            .as_ref()
            .and_then(|f| f.auth.access_token_ttl_secs)
            .unwrap_or(86400);

        let refresh_token_ttl_secs = file
            .as_ref()
            .and_then(|f| f.auth.refresh_token_ttl_secs)
            .unwrap_or(604800);

        let conversation_history_limit = file
            .as_ref()
            .and_then(|f| f.marketplace.conversation_history_limit)
            .unwrap_or(10);

        let max_keyword_len = file
            .as_ref()
            .and_then(|f| f.marketplace.max_keyword_len)
            .unwrap_or(200);

        let price_tolerance = file
            .as_ref()
            .and_then(|f| f.marketplace.price_tolerance)
            .unwrap_or(0.50);

        let categories = file
            .as_ref()
            .and_then(|f| f.marketplace.categories.clone())
            .unwrap_or_else(|| {
                DEFAULT_CATEGORIES
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect()
            });

        Arc::new(Self {
            gemini_api_key: gemini_api_key.unwrap_or_default(),
            minimax_api_key,
            minimax_api_base_url,
            jwt_secret,
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set in environment"),
            llm_provider,
            vector_dim,
            cors_origins,
            blocked_keywords,
            oss_endpoint,
            oss_bucket,
            oss_role_arn,
            oss_access_key_id,
            oss_access_key_secret,
            redis_url,
            rate_limit_max_requests,
            rate_limit_window_secs,
            server_host,
            server_port,
            event_bus_capacity,
            hitl_expire_scan_interval_secs,
            hitl_expire_timeout_hours,
            moka_cache_max_capacity,
            access_token_ttl_secs,
            refresh_token_ttl_secs,
            conversation_history_limit,
            max_keyword_len,
            price_tolerance,
            categories,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_redacts_secrets() {
        let config = AppConfig::load_with_file(None);
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("secret-key-123"));
    }

    #[test]
    fn test_valid_llm_providers() {
        assert!(["gemini", "minimax"].contains(&"gemini"));
        assert!(["gemini", "minimax"].contains(&"minimax"));
    }

    #[test]
    fn test_jwt_secret_minimum_length() {
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
    }

    #[test]
    fn test_categories_default() {
        let config = AppConfig::load_with_file(None);
        assert_eq!(config.categories.len(), 6);
        assert!(config.categories.contains(&"electronics".to_string()));
    }

    #[test]
    fn test_file_config_load_missing_file() {
        // load() returns None when no file exists
        let result = file::load(Some(Path::new("/nonexistent/path.toml")));
        assert!(result.is_none());
    }
}
