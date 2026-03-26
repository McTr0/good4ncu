//! Application metrics for observability.
//!
//! Exposes key business and infrastructure metrics in Prometheus text format
//! via the [`MetricsService`] struct. All metrics are process-local atomic
//! counters/gauges — no external dependency required.
//!
//! Key metrics:
//! - HTTP request counts and latencies per endpoint
//! - Order lifecycle events (created, paid, shipped, completed, cancelled)
//! - Chat message counts
//! - Rate limit rejections
//! - LLM call counts and errors

use prometheus::{Counter, CounterVec, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder};
use std::time::Duration;

/// Centralized metrics registry for the application.
/// All metric collectors are registered here and exposed via `/api/metrics`.
pub struct MetricsService {
    registry: Registry,
    // HTTP — pending HTTP metrics middleware
    #[allow(dead_code)]
    pub http_requests_total: CounterVec,
    #[allow(dead_code)]
    pub http_request_duration_seconds: HistogramVec,
    // Business
    pub orders_created_total: Counter,
    pub orders_paid_total: Counter,
    pub orders_shipped_total: Counter,
    pub orders_completed_total: Counter,
    pub orders_cancelled_total: Counter,
    pub chat_messages_total: Counter,
    // Infrastructure
    pub rate_limit_rejected_total: Counter,
    pub llm_calls_total: Counter,
    pub llm_errors_total: Counter,
}

impl MetricsService {
    /// Create a new MetricsService and register all metric collectors.
    pub fn new() -> Self {
        let registry = Registry::new();

        let http_requests_total = CounterVec::new(
            Opts::new(
                "http_requests_total",
                "Total HTTP requests by method, path, status",
            ),
            &["method", "path", "status"],
        )
        .expect("metric definition is valid");
        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request latency distribution",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5,
            ]),
            &["method", "path"],
        )
        .expect("metric definition is valid");

        let orders_created_total = Counter::new("orders_created_total", "Total orders created")
            .expect("metric definition is valid");
        let orders_paid_total = Counter::new("orders_paid_total", "Total orders marked as paid")
            .expect("metric definition is valid");
        let orders_shipped_total = Counter::new("orders_shipped_total", "Total orders shipped")
            .expect("metric definition is valid");
        let orders_completed_total =
            Counter::new("orders_completed_total", "Total orders completed")
                .expect("metric definition is valid");
        let orders_cancelled_total =
            Counter::new("orders_cancelled_total", "Total orders cancelled")
                .expect("metric definition is valid");
        let chat_messages_total =
            Counter::new("chat_messages_total", "Total chat messages processed")
                .expect("metric definition is valid");
        let rate_limit_rejected_total = Counter::new(
            "rate_limit_rejected_total",
            "Total requests rejected by rate limiter",
        )
        .expect("metric definition is valid");
        let llm_calls_total = Counter::new("llm_calls_total", "Total LLM API calls made")
            .expect("metric definition is valid");
        let llm_errors_total = Counter::new("llm_errors_total", "Total LLM API errors")
            .expect("metric definition is valid");

        // Register all metrics
        registry
            .register(Box::new(http_requests_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(http_request_duration_seconds.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(orders_created_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(orders_paid_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(orders_shipped_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(orders_completed_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(orders_cancelled_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(chat_messages_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(rate_limit_rejected_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(llm_calls_total.clone()))
            .expect("metric is unique");
        registry
            .register(Box::new(llm_errors_total.clone()))
            .expect("metric is unique");

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            orders_created_total,
            orders_paid_total,
            orders_shipped_total,
            orders_completed_total,
            orders_cancelled_total,
            chat_messages_total,
            rate_limit_rejected_total,
            llm_calls_total,
            llm_errors_total,
        }
    }

    /// Record an HTTP request (pending HTTP metrics middleware).
    #[allow(dead_code)]
    pub fn record_http(&self, method: &str, path: &str, status: u16, duration: Duration) {
        let status_str = status.to_string();
        self.http_requests_total
            .with_label_values(&[method, path, &status_str])
            .inc();
        self.http_request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration.as_secs_f64());
    }

    /// Record an order created event.
    pub fn record_order_created(&self) {
        self.orders_created_total.inc();
    }

    /// Record an order paid event.
    pub fn record_order_paid(&self) {
        self.orders_paid_total.inc();
    }

    /// Record an order shipped event.
    pub fn record_order_shipped(&self) {
        self.orders_shipped_total.inc();
    }

    /// Record an order completed event.
    pub fn record_order_completed(&self) {
        self.orders_completed_total.inc();
    }

    /// Record an order cancelled event.
    pub fn record_order_cancelled(&self) {
        self.orders_cancelled_total.inc();
    }

    /// Record a chat message processed.
    pub fn record_chat_message(&self) {
        self.chat_messages_total.inc();
    }

    /// Record a rate limit rejection.
    pub fn record_rate_limit_rejected(&self) {
        self.rate_limit_rejected_total.inc();
    }

    /// Record an LLM API call.
    pub fn record_llm_call(&self) {
        self.llm_calls_total.inc();
    }

    /// Record an LLM API error.
    pub fn record_llm_error(&self) {
        self.llm_errors_total.inc();
    }

    /// Render all metrics in Prometheus text format.
    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder
            .encode_to_string(&metric_families)
            .expect("encoding is infallible")
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new()
    }
}
