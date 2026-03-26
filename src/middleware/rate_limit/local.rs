//! Local (in-memory) rate limiter using moka.
//!
//! Suitable for single-node deployments. Each instance maintains its own
//! token bucket state. Not safe for multi-node deployments.

use moka::sync::Cache;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::middleware::rate_limit::traits::RateLimiter;
use crate::middleware::rate_limit::RateLimitResult;
use async_trait::async_trait;

const WHITELISTED_PATHS: &[&str] = &[
    "/api/health",
    "/api/stats",
    "/api/categories",
    "/api/chat/connections",
    "/api/chat/conversations",
    "/api/chat/messages",
];
const DEFAULT_MAX_REQUESTS: u64 = 100;
const DEFAULT_WINDOW_SECS: u64 = 60;

/// Token bucket rate limiter using moka cache.
/// Each IP gets a token bucket that refills over time.
#[derive(Clone)]
pub struct LocalRateLimiter {
    buckets: Cache<u64, (Instant, u64)>,
    max_tokens: u64,
    refill_duration: Duration,
}

impl LocalRateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            buckets: Cache::builder()
                .max_capacity(100_000)
                .time_to_live(Duration::from_secs(window_secs * 2))
                .build(),
            max_tokens: max_requests,
            refill_duration: Duration::from_secs(window_secs),
        }
    }

    fn hash_ip(&self, ip: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::net::SocketAddr;

        let ip_only = ip
            .parse::<SocketAddr>()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|_| ip.to_string());
        let mut hasher = DefaultHasher::new();
        ip_only.hash(&mut hasher);
        hasher.finish()
    }

    fn check(&self, ip: &str) -> bool {
        let ip_hash = self.hash_ip(ip);
        let now = Instant::now();

        if let Some((last_reset, tokens)) = self.buckets.get(&ip_hash) {
            // moka::sync::Cache::get() returns Option<V> (owned copy), not Option<&V>
            // so last_reset and tokens are owned values here
            let elapsed = now.duration_since(last_reset);
            if elapsed < self.refill_duration {
                if tokens > 0 {
                    self.buckets.insert(ip_hash, (now, tokens - 1));
                    return true;
                }
                return false;
            }
        }

        self.buckets.insert(ip_hash, (now, self.max_tokens - 1));
        true
    }

    #[allow(dead_code)]
    fn reset_ip(&self, ip: &str) {
        let ip_hash = self.hash_ip(ip);
        self.buckets.remove(&ip_hash);
    }
}

#[async_trait]
impl RateLimiter for LocalRateLimiter {
    async fn check_rate_limit(&self, ip: &str) -> RateLimitResult<bool> {
        // moka operations are synchronous and fast; no need to spawn_blocking
        Ok(self.check(ip))
    }

    async fn reset(&self, ip: &str) -> RateLimitResult<()> {
        self.reset_ip(ip);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allows_requests_under_limit() {
        let limiter = LocalRateLimiter::new(5, 60);
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("192.168.1.1").await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_blocks_requests_over_limit() {
        let limiter = LocalRateLimiter::new(3, 60);
        assert!(limiter.check_rate_limit("10.0.0.1").await.unwrap());
        assert!(limiter.check_rate_limit("10.0.0.1").await.unwrap());
        assert!(limiter.check_rate_limit("10.0.0.1").await.unwrap());
        assert!(!limiter.check_rate_limit("10.0.0.1").await.unwrap());
    }

    #[tokio::test]
    async fn test_per_ip_isolation() {
        let limiter = LocalRateLimiter::new(2, 60);
        assert!(limiter.check_rate_limit("1.1.1.1").await.unwrap());
        assert!(limiter.check_rate_limit("1.1.1.1").await.unwrap());
        assert!(!limiter.check_rate_limit("1.1.1.1").await.unwrap());
        assert!(limiter.check_rate_limit("2.2.2.2").await.unwrap());
        assert!(limiter.check_rate_limit("2.2.2.2").await.unwrap());
        assert!(!limiter.check_rate_limit("2.2.2.2").await.unwrap());
    }

    #[tokio::test]
    async fn test_reset_clears_bucket() {
        let limiter = LocalRateLimiter::new(2, 60);
        assert!(limiter.check_rate_limit("10.0.0.1").await.unwrap());
        assert!(limiter.check_rate_limit("10.0.0.1").await.unwrap());
        assert!(!limiter.check_rate_limit("10.0.0.1").await.unwrap());
        limiter.reset("10.0.0.1").await.unwrap();
        assert!(limiter.check_rate_limit("10.0.0.1").await.unwrap());
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible wrappers (used by AppState)
// ---------------------------------------------------------------------------

/// Handle to a [`LocalRateLimiter`] that implements [`Clone`] and can be
/// shared across request handlers. Exposes a synchronous `check_rate_limit`
/// API for use in Axum extractors.
#[derive(Clone)]
pub struct RateLimitStateHandle(Arc<LocalRateLimiter>);

impl RateLimitStateHandle {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self(Arc::new(LocalRateLimiter::new(max_requests, window_secs)))
    }

    pub fn check_rate_limit(&self, ip: &str) -> bool {
        // LocalRateLimiter::check_rate_limit is sync (moka is thread-safe)
        // Use blocking check since we don't want async overhead in the hot path
        self.0.check(ip)
    }
}

/// Create the default rate limit state (100 requests per 60 seconds per IP).
pub fn make_rate_limit_state() -> RateLimitStateHandle {
    RateLimitStateHandle::new(DEFAULT_MAX_REQUESTS, DEFAULT_WINDOW_SECS)
}

/// Returns `true` if the given path is whitelisted from rate limiting.
pub fn is_whitelisted(path: &str) -> bool {
    WHITELISTED_PATHS.contains(&path)
}
