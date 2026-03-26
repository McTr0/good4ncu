use std::time::{Duration, Instant};

use moka::sync::Cache;
use std::sync::Arc;

/// Simple in-memory rate limiter using token bucket algorithm per IP.
/// Uses moka cache with TTL to automatically evict stale entries.
pub struct RateLimitState {
    buckets: Cache<u64, (Instant, u64)>,
    max_tokens: u64,
    refill_duration: Duration,
}

const WHITELISTED_PATHS: &[&str] = &[
    "/api/health",
    "/api/stats",
    "/api/categories",
    "/api/chat/connections",
    "/api/chat/conversations",
    "/api/chat/messages",
];
const DEFAULT_MAX_REQUESTS: u64 = 100; // per window
const DEFAULT_WINDOW_SECS: u64 = 60;

impl RateLimitState {
    fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            buckets: Cache::builder()
                .max_capacity(100_000)
                .time_to_live(Duration::from_secs(window_secs * 2))
                .build(),
            max_tokens: max_requests,
            refill_duration: Duration::from_secs(window_secs),
        }
    }

    fn check_rate_limit(&self, ip: &str) -> bool {
        let ip_hash = self.hash_ip(ip);
        let now = Instant::now();

        if let Some((last_reset, tokens)) = self.buckets.get(&ip_hash) {
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
}

#[derive(Clone)]
pub struct RateLimitStateHandle(pub Arc<RateLimitState>);

impl RateLimitStateHandle {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self(Arc::new(RateLimitState::new(max_requests, window_secs)))
    }

    pub fn check_rate_limit(&self, ip: &str) -> bool {
        self.0.check_rate_limit(ip)
    }
}

pub fn make_rate_limit_state() -> RateLimitStateHandle {
    RateLimitStateHandle::new(DEFAULT_MAX_REQUESTS, DEFAULT_WINDOW_SECS)
}

pub(crate) fn is_whitelisted(path: &str) -> bool {
    WHITELISTED_PATHS.contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_requests_under_limit() {
        let limiter = RateLimitStateHandle::new(5, 60);
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("192.168.1.1"));
        }
    }

    #[test]
    fn test_rate_limit_blocks_requests_over_limit() {
        let limiter = RateLimitStateHandle::new(3, 60);
        assert!(limiter.check_rate_limit("10.0.0.1"));
        assert!(limiter.check_rate_limit("10.0.0.1"));
        assert!(limiter.check_rate_limit("10.0.0.1"));
        assert!(!limiter.check_rate_limit("10.0.0.1"));
    }

    #[test]
    fn test_rate_limit_per_ip_isolation() {
        let limiter = RateLimitStateHandle::new(2, 60);
        assert!(limiter.check_rate_limit("1.1.1.1"));
        assert!(limiter.check_rate_limit("1.1.1.1"));
        assert!(!limiter.check_rate_limit("1.1.1.1"));
        assert!(limiter.check_rate_limit("2.2.2.2"));
        assert!(limiter.check_rate_limit("2.2.2.2"));
        assert!(!limiter.check_rate_limit("2.2.2.2"));
    }

    #[test]
    fn test_rate_limit_ipv6() {
        let limiter = RateLimitStateHandle::new(2, 60);
        assert!(limiter.check_rate_limit("::1"));
        assert!(limiter.check_rate_limit("::1"));
        assert!(!limiter.check_rate_limit("::1"));
    }

    #[test]
    fn test_make_rate_limit_state_default() {
        let limiter = make_rate_limit_state();
        for _ in 0..20 {
            assert!(limiter.check_rate_limit("8.8.8.8"));
        }
        assert!(!limiter.check_rate_limit("8.8.8.8"));
    }
}
