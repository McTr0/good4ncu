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
    tracing_subscriber::fmt::init();

    // Load unified configuration at startup — fail fast if env vars are missing
    let config = config::AppConfig::load();
    tracing::info!(provider = %config.llm_provider, vector_dim = config.vector_dim, "Initializing LLM provider");

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
            Arc::new(crate::llm::minimax::MiniMaxProvider::new(api_key, base_url, &config.gemini_api_key, config.vector_dim)?)
        }
        _ => {
            // Default to Gemini
            let api_key = &config.gemini_api_key;
            if api_key.is_empty() {
                panic!("GEMINI_API_KEY must be set when LLM_PROVIDER=gemini");
            }
            Arc::new(crate::llm::gemini::GeminiProvider::new(api_key, config.vector_dim)?)
        }
    };

    let (services, event_rx) = services::ServiceManager::new(db_pool.clone());
    let event_tx = services.event_tx.clone();

    let event_loop_handle = tokio::spawn(async move {
        services.run_event_loop(event_rx).await;
    });

    let app_state = api::AppState {
        db: db_pool.clone(),
        llm_provider: Arc::clone(&llm_provider),
        event_tx: event_tx.clone(),
        rate_limit: middleware::rate_limit::make_rate_limit_state(),
        jwt_secret: config.jwt_secret.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
    };

    let app = api::create_router(app_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Web Server started at http://127.0.0.1:3000");

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Spawn the CLI as a background task — it will exit immediately in non-TTY environments.
    // The HTTP server continues running regardless.
    let _cli_handle = tokio::spawn(cli::run_cli(db_pool, llm_provider, event_tx));

    // Wait for Ctrl+C to shut down gracefully
    tokio::signal::ctrl_c().await?;
    tracing::info!("Ctrl+C received, shutting down.");

    server_handle.abort();
    event_loop_handle.abort();

    Ok(())
}
