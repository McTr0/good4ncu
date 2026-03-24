use std::time::{Duration, Instant};

use moka::sync::Cache;
use std::sync::Arc;

/// Simple in-memory rate limiter using token bucket algorithm per IP.
/// Uses moka cache with TTL to automatically evict stale entries.
pub struct RateLimitState {
    /// Map from IP hash to (window_start, tokens_available)
    /// TTL-based eviction prevents unbounded memory growth from attack.
    buckets: Cache<u64, (Instant, u64)>,
    max_tokens: u64,
    refill_duration: Duration,
}

impl RateLimitState {
    fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            // Max 100k entries; TTL = 2x window so entries survive one full window.
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

        // Try to get existing bucket
        if let Some((last_reset, tokens)) = self.buckets.get(&ip_hash) {
            let elapsed = now.duration_since(last_reset);
            if elapsed < self.refill_duration {
                // Within window: consume a token if available
                if tokens > 0 {
                    // Insert decremented value (u64 is Copy)
                    self.buckets.insert(ip_hash, (now, tokens - 1));
                    return true;
                }
                return false;
            }
            // Window expired: moka TTL will eventually clean this entry.
            // Fall through to insert a fresh bucket below.
        }

        // No entry or window expired: insert fresh bucket
        self.buckets.insert(ip_hash, (now, self.max_tokens - 1));
        true
    }

    fn hash_ip(&self, ip: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        ip.hash(&mut hasher);
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

/// Create a rate limit state handle.
/// Limits to 20 requests per minute per IP address.
pub fn make_rate_limit_state() -> RateLimitStateHandle {
    RateLimitStateHandle::new(20, 60)
}
