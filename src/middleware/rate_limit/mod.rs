//! Distributed rate limiter abstraction.
//!
//! Provides a [`RateLimiter`] trait that can be implemented by:
//! - [`LocalRateLimiter`](local::LocalRateLimiter): moka-based, single-node
//! - [`RedisRateLimiter`](redis::RedisRateLimiter): redis-based, multi-node (behind `redis` feature)
//!
//! Use [`RateLimiterFactory`] to construct the appropriate implementation
//! based on application configuration.

pub mod local;
#[cfg(feature = "redis")]
pub mod redis_backend;
pub mod traits;

// Re-export for backward compatibility with code that imported from `middleware::rate_limit`
pub use local::{is_whitelisted, RateLimitStateHandle};

/// Factory for creating [`RateLimiter`] instances based on configuration.
#[allow(dead_code)]
#[derive(Clone)]
pub struct RateLimiterFactory {
    max_requests: u64,
    window_secs: u64,
}

#[allow(dead_code)]
impl RateLimiterFactory {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
        }
    }

    /// Build a local (moka-based) rate limiter. Always available.
    pub fn build_local(&self) -> local::LocalRateLimiter {
        local::LocalRateLimiter::new(self.max_requests, self.window_secs)
    }

    /// Build a Redis-backed distributed rate limiter.
    /// Returns an error if the `redis` feature is not enabled or if
    /// the Redis URL is not set.
    #[cfg(feature = "redis")]
    pub async fn build_redis(
        &self,
        redis_url: &str,
    ) -> Result<redis_backend::RedisRateLimiter, redis::RedisError> {
        redis_backend::RedisRateLimiter::new(redis_url, self.max_requests, self.window_secs).await
    }

    /// Build a Redis-backed distributed rate limiter (non-async factory version).
    /// Panics if Redis connection fails.
    #[cfg(not(feature = "redis"))]
    pub fn build_redis_unsupported(&self, _redis_url: &str) -> ! {
        panic!("Redis rate limiter requires the `redis` feature to be enabled")
    }
}

/// Result type for rate limiter operations that can fail.
#[allow(dead_code)]
pub type RateLimitResult<T> = Result<T, RateLimitError>;

/// Errors that can occur during rate limiting operations.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[cfg(feature = "redis")]
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
