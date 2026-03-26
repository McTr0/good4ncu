use crate::agents::router::IntentRouter;
use crate::api::metrics::MetricsService;
use crate::llm::{LlmProvider, MarketplaceAgent};
use crate::services::chat::ChatService;
use crate::services::notification::NotificationService;
use crate::services::BusinessEvent;
use axum::{
    extract::State,
    middleware,
    response::Response,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use futures::StreamExt;
use sqlx::Row;
pub mod admin;
pub mod auth;
pub mod conversations;
pub mod error;
pub mod listings;
pub mod metrics;
pub mod negotiate;
pub mod notifications;
pub mod orders;
pub mod recommendations;
pub mod stats;
pub mod upload;
pub mod user;
pub mod user_chat;
pub mod watchlist;
pub mod ws;
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

use crate::middleware::rate_limit::{is_whitelisted, RateLimitStateHandle};
use regex::Regex;

/// Security headers applied to all responses.
async fn security_headers_middleware(
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    response
}

/// Rate-limit middleware that checks rate limits before passing requests to handlers.
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    use axum::response::IntoResponse;

    let path = request.uri().path().to_string();

    if is_whitelisted(&path) {
        return next.run(request).await;
    }

    // Extract peer address from TCP socket — cannot be spoofed by clients.
    let peer_addr = request
        .extensions()
        .get::<SocketAddr>()
        .copied()
        .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());

    if !state
        .rate_limit
        .check_rate_limit(&peer_addr.to_string())
        .await
    {
        state.metrics.record_rate_limit_rejected();
        return ApiError::RateLimitExceeded.into_response();
    }

    next.run(request).await
}

/// Normalize dynamic path segments to prevent Prometheus label cardinality explosion.
/// Replaces UUIDs, MongoDB ObjectIds, and numeric IDs with `{id}`.
fn normalize_path(path: &str) -> String {
    let uuid_regex =
        Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
            .unwrap();
    let mongo_id_regex = Regex::new(r"[0-9a-fA-F]{24}").unwrap();
    let numeric_regex = Regex::new(r"\d+").unwrap();

    let step1 = uuid_regex.replace_all(path, "{id}");
    let step2 = mongo_id_regex.replace_all(&step1, "{id}");
    let step3 = numeric_regex.replace_all(&step2, "{id}");
    step3.to_string()
}

/// HTTP metrics middleware that records request count and latency per endpoint.
pub async fn http_metrics_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let start = std::time::Instant::now();
    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status().as_u16();

    state
        .metrics
        .record_http(&method, &normalize_path(&path), status, duration);

    response
}

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
    pub notification: NotificationService,
    #[allow(dead_code)]
    pub ws_connections: Arc<ws::WsConnections>,
    pub router: IntentRouter,
    /// Alibaba Cloud OSS configuration for STS direct-upload.
    pub oss_endpoint: String,
    pub oss_bucket: String,
    pub oss_role_arn: Option<String>,
    pub oss_access_key_id: Option<String>,
    pub oss_access_key_secret: Option<String>,
    pub metrics: std::sync::Arc<MetricsService>,
}

pub fn create_router(state: AppState, cors_origins: &[String]) -> Router {
    let cors = if cors_origins.is_empty() {
        // Default restrictive CORS — no origins allowed by default
        CorsLayer::new().allow_methods(Any).allow_headers(Any)
    } else if cors_origins.iter().any(|s| s == "*") {
        // Wildcard: allow all origins
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
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
        .route("/api/metrics", get(get_metrics))
        .route("/api/stats", get(stats::get_stats))
        .route("/api/admin/stats", get(admin::get_admin_stats))
        .route("/api/admin/users", get(admin::get_admin_users))
        .route("/api/admin/listings", get(admin::get_admin_listings))
        .route("/api/admin/orders", get(admin::get_admin_orders))
        .route(
            "/api/recommendations/feed",
            get(recommendations::get_recommendation_feed),
        )
        .route(
            "/api/recommendations/similar",
            get(recommendations::get_similar_listings),
        )
        .route("/api/categories", get(listings::get_categories))
        .route("/api/chat", post(handle_chat))
        .route("/api/chat/stream", get(handle_chat_stream))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/change-password", post(auth::change_password))
        .route("/api/auth/refresh", post(auth::refresh_token))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/listings", get(listings::get_listings))
        .route("/api/listings/recognize", post(listings::recognize_item))
        .route("/api/listings/{id}", get(listings::get_listing))
        .route("/api/listings/{id}", put(listings::update_listing))
        .route("/api/listings", post(listings::create_listing))
        .route("/api/listings/{id}", delete(listings::delete_listing))
        .route("/api/listings/{id}/relist", post(listings::relist_listing))
        .route("/api/user/profile", get(user::get_profile))
        .route("/api/user/listings", get(user::get_user_listings))
        .route("/api/users/search", get(user::search_users))
        .route("/api/users/{id}", get(user::get_user_profile))
        .route("/api/orders", get(orders::get_orders))
        .route("/api/orders", post(orders::create_order))
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
            get(watchlist::check_watchlist),
        )
        .route(
            "/api/watchlist/{listing_id}",
            post(watchlist::add_to_watchlist),
        )
        .route(
            "/api/watchlist/{listing_id}",
            delete(watchlist::remove_from_watchlist),
        )
        .route("/api/notifications", get(notifications::get_notifications))
        .route(
            "/api/notifications/{id}/read",
            post(notifications::mark_notification_read),
        )
        .route(
            "/api/notifications/read-all",
            post(notifications::mark_all_notifications_read),
        )
        .route("/api/negotiations", get(negotiate::list_negotiations))
        .route(
            "/api/negotiations/{id}/respond",
            patch(negotiate::respond_negotiation),
        )
        .route(
            "/api/negotiations/{id}/accept",
            patch(negotiate::accept_counter_negotiation),
        )
        .route(
            "/api/negotiations/{id}/reject",
            patch(negotiate::reject_counter_negotiation),
        )
        .route(
            "/api/chat/connect/request",
            post(user_chat::connect_request),
        )
        .route("/api/chat/connect/accept", post(user_chat::connect_accept))
        .route("/api/chat/connect/reject", post(user_chat::connect_reject))
        .route("/api/chat/connections", get(user_chat::list_connections))
        .route(
            "/api/chat/conversations/{id}/messages",
            get(user_chat::get_connection_messages),
        )
        .route(
            "/api/chat/conversations/{id}/messages",
            post(user_chat::send_connection_message),
        )
        .route(
            "/api/chat/messages/{id}/read",
            post(user_chat::mark_message_read),
        )
        .route("/api/upload/token", get(upload::get_upload_token))
        .route("/api/ws", get(ws::ws_handler))
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            http_metrics_middleware,
        ))
        .with_state(state)
}

/// GET /api/metrics — Prometheus text format metrics (no auth required)
async fn get_metrics(State(state): State<AppState>) -> String {
    state.metrics.render()
}

async fn health_check(State(state): State<AppState>) -> Result<&'static str, ApiError> {
    // Verify database connectivity — critical for production deployments
    sqlx::query("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(%e, "Health check failed: database unreachable");
            ApiError::Internal(anyhow::anyhow!("Database unreachable: {}", e))
        })?;
    Ok("OK")
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    image: Option<String>,
    audio: Option<String>,
    conversation_id: Option<String>,
    /// Optional listing context — when provided, the buyer is inquiring about
    /// a specific listing. The conversation is anchored to the listing owner
    /// as receiver so they immediately see it in their conversation list.
    listing_id: Option<String>,
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

    let current_user_id = auth::extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Route: lightweight intent classification before doing any LLM work.
    // Blocked content and simple chat greetings short-circuit here — no token spent.
    let intent_result = state.router.classify(&payload.message);
    tracing::debug!(intent = ?intent_result.intent.as_str(), confidence = %intent_result.confidence, "Router classification");

    // Blocked: reject immediately, no LLM tokens consumed.
    if let Some(reply) = intent_result.direct_response(&payload.message) {
        let conversation_id = payload
            .conversation_id
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        return Ok(Json(ChatResponse {
            reply,
            conversation_id,
        }));
    }

    // Resolve listing context: if listing_id is provided, look up the owner to use as
    // the message receiver. This anchors buyer→seller conversations to a specific listing
    // so the seller immediately sees the conversation in their list (receiver = seller).
    let listing_id: String;
    let receiver: Option<String>;
    match payload.listing_id {
        Some(ref lid) if !lid.is_empty() => {
            let row = sqlx::query("SELECT owner_id, status FROM inventory WHERE id = $1")
                .bind(lid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

            match row {
                Some(r) => {
                    let owner_id: String = r.get("owner_id");
                    let status: String = r.get("status");
                    if status == "active" && owner_id != current_user_id {
                        // Pass listing_id for context; owner is stored as receiver so seller
                        // immediately sees this conversation in their list_conversations.
                        listing_id = lid.clone();
                        receiver = Some(owner_id);
                    } else {
                        // Inactive listing or buyer is the owner — fall back to global context
                        listing_id = "global".to_string();
                        receiver = None;
                    }
                }
                None => {
                    listing_id = "global".to_string();
                    receiver = None;
                }
            }
        }
        _ => {
            listing_id = "global".to_string();
            receiver = None;
        }
    };

    let conversation_id = payload
        .conversation_id
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let chat_svc = ChatService::new(state.db.clone());

    // Log user message BEFORE LLM call — prevents data loss if LLM times out or request is aborted.
    // When listing_id is provided, receiver is set to the listing owner so the seller
    // immediately sees the buyer's inquiry in their conversation list.
    let log_user = chat_svc.log_message(
        &conversation_id,
        &listing_id,
        &current_user_id,
        receiver.as_deref(),
        false,
        &payload.message,
        payload.image.as_deref(),
        payload.audio.as_deref(),
    );

    let log_result = log_user.await;
    if let Err(e) = log_result {
        tracing::warn!(%e, "Failed to log user message — continuing anyway");
    }

    state.metrics.record_chat_message();

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
            state.metrics.record_llm_error();
            ApiError::Internal(anyhow::anyhow!(e))
        })?;

    state.metrics.record_llm_call();

    // Log agent reply — fire and forget, errors are non-fatal.
    // Agent messages have no receiver (they're broadcast-style from the AI assistant).
    let log_agent = chat_svc.log_message(
        &conversation_id,
        &listing_id,
        "assistant",
        None,
        true,
        &reply,
        None,
        None,
    );

    if let Err(e) = log_agent.await {
        tracing::warn!(%e, "Failed to log agent reply");
    }

    // Send event with backpressure: wait up to 5 seconds, then log and continue.
    // Using send instead of try_send prevents silent event loss under load.
    let chat_event = BusinessEvent::ChatMessage {
        conversation_id: conversation_id.clone(),
        listing_id: listing_id.to_string(),
        sender: current_user_id,
        content: payload.message,
        image_data: payload.image,
        audio_data: payload.audio,
    };
    // Send event with backpressure: block until the event is received or the channel
    // is closed. This is preferred over try_send/timeout because ChatMessage must
    // be persisted — dropping it silently would cause the user to see their message
    // disappear with no reply and no error feedback.
    if let Err(e) = state.event_tx.send(chat_event).await {
        tracing::error!(%e, "Event bus closed, ChatMessage not delivered");
        return Err(ApiError::Internal(anyhow::anyhow!(
            "服务暂时不可用，请稍后重试: {}",
            e
        )));
    }

    Ok(Json(ChatResponse {
        reply,
        conversation_id,
    }))
}

/// Query params for SSE chat streaming.
/// Token is required for auth; message is required to start a new turn.
#[derive(Deserialize)]
struct ChatStreamQuery {
    token: String,
    message: String,
    /// Optional listing context — same as handle_chat.
    listing_id: Option<String>,
    /// Optional conversation_id — if provided, continues existing conversation.
    conversation_id: Option<String>,
}

/// GET /api/chat/stream — SSE streaming chat endpoint.
///
/// Clients send: GET /api/chat/stream?token=<jwt>&message=hello&listing_id=xxx
/// Server streams: text/event-stream with data: {"token": "..."} chunks.
///
/// Each chunk is a JSON object with a "token" field containing the text fragment.
/// The stream closes when the LLM finishes responding (no more tool calls).
async fn handle_chat_stream(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<ChatStreamQuery>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    if params.message.len() > 2000 {
        return Err(ApiError::BadRequest(
            "Text message exceeds maximum length of 2000 characters.".to_string(),
        ));
    }

    let current_user_id = auth::extract_user_id_from_token_str(&params.token, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Route: lightweight intent classification before doing any LLM work.
    let intent_result = state.router.classify(&params.message);
    tracing::debug!(intent = ?intent_result.intent.as_str(), confidence = %intent_result.confidence, "SSE Router classification");

    // Blocked: reject immediately, no LLM tokens consumed.
    if let Some(reply) = intent_result.direct_response(&params.message) {
        let conversation_id = params
            .conversation_id
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let payload = serde_json::json!({ "token": reply, "conversation_id": conversation_id });
        let body = axum::body::Body::from(format!(
            "data: {}\n\n",
            serde_json::to_string(&payload).unwrap()
        ));
        return Ok(Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("X-Conversation-Id", &conversation_id)
            .body(body)
            .unwrap());
    }

    // Same listing resolution as handle_chat.
    let listing_id: String;
    let _receiver: Option<String>;
    match params.listing_id {
        Some(ref lid) if !lid.is_empty() => {
            let row = sqlx::query("SELECT owner_id, status FROM inventory WHERE id = $1")
                .bind(lid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

            match row {
                Some(r) => {
                    let owner_id: String = r.get("owner_id");
                    let status: String = r.get("status");
                    if status == "active" && owner_id != current_user_id {
                        listing_id = lid.clone();
                        _receiver = Some(owner_id);
                    } else {
                        listing_id = "global".to_string();
                        _receiver = None;
                    }
                }
                None => {
                    listing_id = "global".to_string();
                    _receiver = None;
                }
            }
        }
        _ => {
            listing_id = "global".to_string();
            _receiver = None;
        }
    };

    let conversation_id = params
        .conversation_id
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let chat_svc = ChatService::new(state.db.clone());

    // Log user message before streaming.
    let log_user = chat_svc.log_message(
        &conversation_id,
        &listing_id,
        &current_user_id,
        _receiver.as_deref(),
        false,
        &params.message,
        None,
        None,
    );
    if let Err(e) = log_user.await {
        tracing::warn!(%e, "Failed to log user message for SSE stream");
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

    let stream = agent.stream_chat(params.message.clone(), chat_history);

    // Send ChatMessage event.
    let chat_event = BusinessEvent::ChatMessage {
        conversation_id: conversation_id.clone(),
        listing_id: listing_id.to_string(),
        sender: current_user_id,
        content: params.message,
        image_data: None,
        audio_data: None,
    };
    let _ = state.event_tx.try_send(chat_event);

    // Build SSE stream: each token becomes data: {"token": "..."}\n\n
    let conv_id = conversation_id.clone();
    let sse_stream = stream.map(move |result| {
        let line = match result {
            Ok(token) => {
                let payload = serde_json::json!({ "token": token, "conversation_id": conv_id });
                format!("data: {}\n\n", serde_json::to_string(&payload).unwrap())
            }
            Err(e) => {
                let payload = serde_json::json!({ "error": e.to_string() });
                format!("data: {}\n\n", serde_json::to_string(&payload).unwrap())
            }
        };
        Ok::<_, std::convert::Infallible>(line.into_bytes())
    });

    let body = axum::body::Body::from_stream(sse_stream);

    Ok(Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("X-Conversation-Id", &conversation_id)
        .body(body)
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_with_message() {
        let json = r#"{"message": "Hello!", "conversation_id": "conv-1"}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "Hello!");
        assert_eq!(req.conversation_id, Some("conv-1".to_string()));
        assert_eq!(req.image, None);
        assert_eq!(req.audio, None);
        assert_eq!(req.listing_id, None);
    }

    #[test]
    fn test_chat_request_with_media() {
        let json = r#"{"message": "Check this", "image": "base64data", "audio": "base64audio"}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "Check this");
        assert_eq!(req.image, Some("base64data".to_string()));
        assert_eq!(req.audio, Some("base64audio".to_string()));
        assert_eq!(req.listing_id, None);
    }

    #[test]
    fn test_chat_request_with_listing_context() {
        let json = r#"{"message": "Is this available?", "listing_id": "listing-123"}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "Is this available?");
        assert_eq!(req.listing_id, Some("listing-123".to_string()));
    }

    #[test]
    fn test_chat_request_without_conversation_id() {
        let json = r#"{"message": "Hello!"}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "Hello!");
        assert_eq!(req.conversation_id, None);
    }

    #[test]
    fn test_chat_request_empty_conversation_id() {
        let json = r#"{"message": "Hi", "conversation_id": ""}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "Hi");
        assert_eq!(req.conversation_id, Some("".to_string()));
    }

    #[test]
    fn test_chat_response_serialization() {
        let resp = ChatResponse {
            reply: "Hello back!".to_string(),
            conversation_id: "conv-123".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Hello back!"));
        assert!(json.contains("conv-123"));
    }

    #[test]
    fn test_peer_addr_clone() {
        use std::net::SocketAddr;
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let peer = PeerAddr(addr);
        let _cloned = peer.clone();
    }

    #[test]
    fn test_peer_addr_debug() {
        use std::net::SocketAddr;
        let addr: SocketAddr = "192.168.1.1:3000".parse().unwrap();
        let peer = PeerAddr(addr);
        let debug_str = format!("{:?}", peer);
        assert!(debug_str.contains("PeerAddr"));
        assert!(debug_str.contains("192.168.1.1"));
    }
}
