use crate::llm::{LlmProvider, MarketplaceAgent};
use crate::services::chat::ChatService;
use crate::services::BusinessEvent;
use axum::{
    extract::State,
    routing::{delete, get, post, put},
    Json, Router,
};
pub mod auth;
pub mod conversations;
pub mod error;
pub mod listings;
pub mod orders;
pub mod user;
pub mod watchlist;
use error::ApiError;
use rig::completion::Message;
use rig::message::{AssistantContent, Text, UserContent};
use rig::OneOrMany;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use uuid::Uuid;

use crate::middleware::rate_limit::RateLimitStateHandle;

/// Extractor that provides the direct TCP peer address of the connected client.
/// Unlike ConnectInfo, this works with the plain axum::serve without special server configuration.
/// The peer address cannot be spoofed by clients since it comes from the TCP stack.
#[derive(Clone, Debug)]
pub struct PeerAddr(pub SocketAddr);

impl<S> axum::extract::FromRequestParts<S> for PeerAddr
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // axum::serve automatically adds extensions::PeerAddr when using the MakeService
        let addr = parts
            .extensions
            .get::<axum::extract::connect_info::ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0)
            .or_else(|| parts.extensions.get::<std::net::SocketAddr>().copied())
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());
        Ok(PeerAddr(addr))
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub llm_provider: Arc<dyn LlmProvider>,
    pub event_tx: mpsc::Sender<BusinessEvent>,
    pub rate_limit: RateLimitStateHandle,
    pub jwt_secret: String,
    pub gemini_api_key: String,
}

pub fn create_router(state: AppState, cors_origins: &[String]) -> Router {
    let cors = if cors_origins.is_empty() {
        // Default restrictive CORS — no origins allowed by default
        CorsLayer::new().allow_methods(Any).allow_headers(Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = cors_origins
            .iter()
            .filter_map(|s| s.parse::<axum::http::HeaderValue>().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/chat", post(handle_chat))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/change-password", post(auth::change_password))
        .route("/api/listings", get(listings::get_listings))
        .route("/api/listings/recognize", post(listings::recognize_item))
        .route("/api/listings/{id}", get(listings::get_listing))
        .route("/api/listings/{id}", put(listings::update_listing))
        .route("/api/listings", post(listings::create_listing))
        .route("/api/listings/{id}", delete(listings::delete_listing))
        .route("/api/user/profile", get(user::get_profile))
        .route("/api/user/listings", get(user::get_user_listings))
        .route("/api/users/search", get(user::search_users))
        .route("/api/orders", get(orders::get_orders))
        .route("/api/orders/{id}", get(orders::get_order))
        .route("/api/orders/{id}/cancel", post(orders::cancel_order))
        .route("/api/orders/{id}/confirm", post(orders::confirm_order))
        .route("/api/orders/{id}/pay", post(orders::pay_order))
        .route("/api/orders/{id}/ship", post(orders::ship_order))
        .route("/api/conversations", get(conversations::list_conversations))
        .route(
            "/api/conversations/{id}/messages",
            get(conversations::get_conversation_messages),
        )
        .route("/api/watchlist", get(watchlist::get_watchlist))
        .route(
            "/api/watchlist/{listing_id}",
            post(watchlist::add_to_watchlist),
        )
        .route(
            "/api/watchlist/{listing_id}",
            delete(watchlist::remove_from_watchlist),
        )
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    image: Option<String>,
    audio: Option<String>,
    conversation_id: Option<String>,
}

#[derive(Serialize)]
struct ChatResponse {
    reply: String,
    conversation_id: String,
}

use axum::http::HeaderMap;

async fn handle_chat(
    State(state): State<AppState>,
    PeerAddr(addr): PeerAddr,
    headers: HeaderMap,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    // Reject oversized payloads before they can exhaust API tokens or memory.
    // 10 MB network limit is enforced by RequestBodyLimitLayer.
    // Text messages beyond 2000 chars are almost certainly abuse.
    if payload.message.len() > 2000 {
        return Err(ApiError::BadRequest(
            "Text message exceeds maximum length of 2000 characters.".to_string(),
        ));
    }

    // Use the direct TCP peer address as rate limit key — it cannot be spoofed.
    // X-Forwarded-For is read for logging only, never as a rate-limit token.
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim());

    if let Some(proxy_ip) = client_ip {
        tracing::debug!(client_ip = %proxy_ip, peer = %addr, "Chat request");
    }

    if !state.rate_limit.check_rate_limit(&addr.to_string()) {
        return Err(ApiError::RateLimitExceeded);
    }

    let current_user_id = auth::extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let conversation_id = payload
        .conversation_id
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let listing_id = "global";

    let chat_svc = ChatService::new(state.db.clone());

    // Log user message BEFORE LLM call — prevents data loss if LLM times out or request is aborted.
    let log_user = chat_svc.log_message(
        &conversation_id,
        listing_id,
        &current_user_id,
        false,
        &payload.message,
        payload.image.as_deref(),
        payload.audio.as_deref(),
    );

    let log_result = log_user.await;
    if let Err(e) = log_result {
        tracing::warn!(%e, "Failed to log user message — continuing anyway");
    }

    let history_entries = chat_svc
        .get_conversation_history(&conversation_id)
        .await
        .unwrap_or_default();

    let chat_history: Vec<Message> = history_entries
        .iter()
        .map(|entry| {
            if entry.is_agent {
                Message::Assistant {
                    id: None,
                    content: OneOrMany::one(AssistantContent::Text(Text {
                        text: entry.content.clone(),
                    })),
                }
            } else {
                Message::User {
                    content: OneOrMany::one(UserContent::Text(Text {
                        text: entry.content.clone(),
                    })),
                }
            }
        })
        .collect();

    let agent: Box<dyn MarketplaceAgent> = state
        .llm_provider
        .create_marketplace_agent(
            &state.db,
            state.event_tx.clone(),
            Some(current_user_id.clone()),
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let reply = agent
        .prompt_with_history(payload.message.clone(), chat_history)
        .await
        .map_err(|e| {
            tracing::error!(err = %e, "LLM prompt failed");
            ApiError::Internal(anyhow::anyhow!(e))
        })?;

    // Log agent reply — fire and forget, errors are non-fatal.
    let log_agent = chat_svc.log_message(
        &conversation_id,
        listing_id,
        "assistant",
        true,
        &reply,
        None,
        None,
    );

    if let Err(e) = log_agent.await {
        tracing::warn!(%e, "Failed to log agent reply");
    }

    if state
        .event_tx
        .try_send(BusinessEvent::ChatMessage {
            conversation_id: conversation_id.clone(),
            listing_id: listing_id.to_string(),
            sender: current_user_id,
            content: payload.message,
            image_data: payload.image,
            audio_data: payload.audio,
        })
        .is_err()
    {
        tracing::warn!("Event bus full, dropping ChatMessage");
    }

    Ok(Json(ChatResponse {
        reply,
        conversation_id,
    }))
}
