mod agents;
mod services;

mod api;
mod cli;
mod config;
mod db;
mod llm;
mod middleware;
mod utils;

use std::sync::Arc;

use crate::llm::LlmProvider;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    // Check for CLI commands first
    let cli_args: Vec<String> = std::env::args().collect();
    if cli::run_cli(&cli_args).await? {
        return Ok(());
    }

    // Initialize structured JSON logging for production observability
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("good4ncu=info".parse().unwrap())
                .add_directive("hyper=warn".parse().unwrap())
                .add_directive("tower=warn".parse().unwrap()),
        )
        .with_target(true)
        .with_thread_ids(true)
        .json()
        .init();

    // Load unified configuration at startup — fail fast if env vars are missing
    let config = config::AppConfig::load();
    tracing::info!(provider = %config.llm_provider, vector_dim = config.vector_dim, "Initializing LLM provider");

    // Metrics service — shared across all request handlers
    let metrics = Arc::new(api::metrics::MetricsService::new());
    tracing::info!("Metrics service initialized");

    // Single PgPool for relational + vector data (pgvector lives in the same Postgres instance)
    let db_pool = db::init_db(&config.database_url).await?;

    // Build the LLM provider based on configuration
    let llm_provider: Arc<dyn LlmProvider> = match config.llm_provider.as_str() {
        "minimax" => {
            let api_key = config
                .minimax_api_key
                .as_ref()
                .expect("MINIMAX_API_KEY must be set when LLM_PROVIDER=minimax");
            let base_url = config.minimax_api_base_url.as_deref();
            Arc::new(crate::llm::minimax::MiniMaxProvider::new(
                api_key,
                base_url,
                &config.gemini_api_key,
                config.vector_dim,
            )?)
        }
        _ => {
            // Default to Gemini
            let api_key = &config.gemini_api_key;
            if api_key.is_empty() {
                panic!("GEMINI_API_KEY must be set when LLM_PROVIDER=gemini");
            }
            Arc::new(crate::llm::gemini::GeminiProvider::new(
                api_key,
                config.vector_dim,
            )?)
        }
    };

    let (services, event_rx) = services::ServiceManager::new(db_pool.clone());
    let event_tx = services.event_tx.clone();

    let event_loop_handle = tokio::spawn(async move {
        services.run_event_loop(event_rx).await;
    });

    // WebSocket global state — shared across all connections.
    let ws_state = api::ws::new_ws_state();

    // Shared broadcast callback for WS push — passed to both NotificationService and hitl_expire.
    let broadcast: crate::services::notification::NotificationBroadcast =
        Arc::new(|user_id: String, payload: String| {
            api::ws::broadcast_to_user(&user_id, &payload);
        });

    // Build the notification service with WebSocket broadcast wired in.
    let notification = crate::services::notification::NotificationService::new(db_pool.clone())
        .with_broadcast(Arc::clone(&broadcast));

    let router = crate::agents::router::IntentRouter::new(config.blocked_keywords.clone());

    // HITL expiration worker: scans every 10 min for pending requests > 48h old.
    let hitl_expire_handle = tokio::spawn(services::hitl_expire::run(
        db_pool.clone(),
        Arc::clone(&broadcast),
    ));

    let app_state = api::AppState {
        db: db_pool.clone(),
        llm_provider: Arc::clone(&llm_provider),
        event_tx: event_tx.clone(),
        rate_limit: {
            let factory = middleware::rate_limit::RateLimiterFactory::new(
                config.rate_limit_max_requests,
                config.rate_limit_window_secs,
            );
            middleware::rate_limit::RateLimitStateHandle::new(factory.build_local())
        },
        jwt_secret: config.jwt_secret.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        notification,
        ws_connections: ws_state,
        router,
        oss_endpoint: config.oss_endpoint.clone(),
        oss_bucket: config.oss_bucket.clone(),
        oss_role_arn: config.oss_role_arn.clone(),
        oss_access_key_id: config.oss_access_key_id.clone(),
        oss_access_key_secret: config.oss_access_key_secret.clone(),
        metrics: Arc::clone(&metrics),
    };

    let app = api::create_router(app_state, &config.cors_origins);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Web Server started at http://127.0.0.1:3000");

    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(%e, "Server error");
        }
    });

    // Wait for Ctrl+C to shut down gracefully
    tokio::signal::ctrl_c().await?;
    tracing::info!("Ctrl+C received, shutting down.");

    server_handle.abort();
    event_loop_handle.abort();
    hitl_expire_handle.abort();

    // Gracefully close the DB pool so Postgres can cleanly收回所有连接
    // and flush any pending transaction results in the buffer.
    db_pool.close().await;

    tracing::info!("Shutdown complete.");
    Ok(())
}
