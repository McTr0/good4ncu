//! Core trait for distributed rate limiters.

use crate::middleware::rate_limit::RateLimitResult;
use async_trait::async_trait;

/// A rate limiter that can be used across a distributed system.
/// Implementations can be in-memory (single node) or Redis-backed (multi-node).
#[allow(dead_code)]
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Check if a request from the given IP is allowed under the rate limit.
    /// Returns `Ok(true)` if allowed, `Ok(false)` if rate limited.
    async fn check_rate_limit(&self, ip: &str) -> RateLimitResult<bool>;

    /// Reset the rate limit for a given IP (admin use only).
    async fn reset(&self, ip: &str) -> RateLimitResult<()>;
}
