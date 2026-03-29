# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Agentic information sharing platform for Chinese university campuses. Built with Rust + Axum + PostgreSQL (relational + vector via pgvector) + Flutter mobile. AI-powered via Google Gemini or MiniMax LLM providers, using the Rig framework.

**Disclaimer:** 本产品仅做信息发布，无担保和资金中介，也不收手续费。

## Build and Run Commands

```bash
cargo build && cargo run          # Starts HTTP server on :3000 + interactive CLI
cargo check                       # Type-check without building
cargo clippy -- -D warnings       # Lint
cargo fmt && cargo fmt -- --check # Format and check
cargo test                        # Run all tests (lib + integration)
cargo test --lib                  # Run unit tests only
```

Requires `.env` with `GEMINI_API_KEY` (or `MINIMAX_API_KEY`), `JWT_SECRET`, and `DATABASE_URL`. See `AGENTS.md` for Flutter mobile development.

## Environment Variables

**Secrets (required) — must come from env vars, never from config file:**

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `GEMINI_API_KEY` | Yes* | Google Gemini API key (*required unless using minimax with gemini for embeddings) |
| `JWT_SECRET` | Yes | Secret for JWT token signing (min 32 chars) |
| `OSS_ACCESS_KEY_ID` | No | Alibaba OSS access key |
| `OSS_ACCESS_KEY_SECRET` | No | Alibaba OSS secret key |

**Non-secret config — env var overrides TOML file, TOML file overrides hardcoded default:**

| Variable | TOML Path | Default | Description |
|----------|-----------|---------|-------------|
| `LLM_PROVIDER` | `llm.provider` | `gemini` | `gemini` or `minimax` |
| `VECTOR_DIM` | `llm.vector_dim` | `768` | Embedding vector dimensions |
| `CORS_ORIGINS` | `cors.origins` | allow all | Comma-separated allowed origins |
| `RATE_LIMIT_MAX_REQUESTS` | `rate_limit.max_requests` | `100` | Max requests per window |
| `RATE_LIMIT_WINDOW_SECS` | `rate_limit.window_secs` | `60` | Rate limit window (seconds) |
| `OSS_ENDPOINT` | `oss.endpoint` | `https://oss-cn-beijing.aliyuncs.com` | OSS endpoint |
| `OSS_BUCKET` | `oss.bucket` | `good4ncu` | OSS bucket name |
| `REDIS_URL` | `rate_limit.redis_url` | — | Redis URL for distributed rate limiting |
| `BLOCKED_KEYWORDS` | `moderation.blocked_keywords` | — | Comma-separated blocked keywords |

**Config file search order:**
1. `$CONFIG_FILE` env var (if set)
2. `./good4ncu.toml` (if exists)
3. `./config/good4ncu.toml` (if exists)

See `config.toml.example` for the full TOML schema.

## Architecture

### AppState Structure
`AppState` (Clone) is passed to all Axum handlers with three sub-structs:
```rust
pub struct AppState {
    pub secrets: ApiSecrets,       // Static config (JWT secret, API keys, OSS config)
    pub infra: ApiInfrastructure,  // Runtime infra (DB, event bus, rate limiter, WS, metrics)
    pub agents: ApiAgents,         // LLM provider + intent router
    // Repository layer (concrete types)
    pub listing_repo: repositories::PostgresListingRepository,
    pub user_repo: repositories::PostgresUserRepository,
    pub chat_repo: repositories::PostgresChatRepository,
    pub auth_repo: repositories::PostgresAuthRepository,
    pub order_repo: repositories::PostgresOrderRepository,
}

pub struct ApiInfrastructure {
    pub db: PgPool,
    pub event_tx: mpsc::Sender<BusinessEvent>,
    pub rate_limit: RateLimitStateHandle,
    pub notification: NotificationService,
    pub ws_connections: Arc<ws::WsConnections>,
    pub metrics: Arc<MetricsService>,
    pub order_service: order::OrderService,
    pub admin_service: crate::services::admin::AdminService,
    pub moderation: services::moderation::ModerationService,
}
```

### Database
The codebase uses a **single PostgreSQL connection** (`sqlx::PgPool`) for both relational and vector data:
- Relational tables: `users`, `inventory`, `chat_messages`, `chat_connections`, `orders`, `documents`, `watchlist`, `notifications`, `hitl_requests`, `refresh_tokens`
- Vector storage: `pgvector` extension (created via `CREATE EXTENSION IF NOT EXISTS vector`)
- **Money conventions**: `suggested_price_cny` stored as `i32` (cents), converted to yuan via `cents_to_yuan()` helper in `src/utils.rs`

### Runtime Modes
`main.rs` runs two async tasks:
1. **Axum HTTP server** on `127.0.0.1:3000`
2. **CLI** (`cli::run_cli`) — spawned as background task, exits immediately in non-TTY

Shutdown via Ctrl+C aborts both tasks via stored `JoinHandle`s.

### Event-Driven Services
`services::ServiceManager` owns all services and runs a background event loop:
- Receives `BusinessEvent` variants via `tokio::sync::mpsc::Receiver`
- Spawns `tokio::spawn` tasks per event to call appropriate service methods
- Three event variants: `DealReached` (triggers order creation), `OrderPaid` (logs only), `ChatMessage` (logs to chat)
- Uses bounded channel (capacity 2048) for backpressure

### LLM Provider System
`llm::LlmProvider` is a unified trait abstracting over Gemini and MiniMax:
- `create_marketplace_agent()` — creates RAG-enabled marketplace agent
- `create_negotiate_agent()` — creates negotiation support agent
- `llm/gemini.rs` and `llm/minimax.rs` are concrete implementations
- Agent preamble is Chinese (Simplified) — see `llm/mod.rs::PREAMBLE` and `llm/mod.rs::NEGOTIATION_PREAMBLE`

### API Error Types
`ApiError` enum in `src/api/error.rs` maps to HTTP status codes:
| Variant | Status | Notes |
|---------|--------|-------|
| `NotFound` | 404 | Resource does not exist |
| `BadRequest(String)` | 400 | Invalid request parameters |
| `Unauthorized` | 401 | Not logged in |
| `AuthFailed(String)` | 401 | Invalid credentials |
| `Forbidden` | 403 | Insufficient permissions |
| `Conflict(String)` | 409 | Resource conflict (e.g., duplicate, already sold) |
| `RateLimitExceeded` | 429 | Too many requests |
| `Internal(anyhow::Error)` | 500 | Unexpected server error |

### WebSocket Events
WS events pushed to clients (see `src/api/ws.rs` and `src/api/user_chat.rs`):
- `connection_request` - new connection request received
- `connection_established` - connection accepted and established
- `new_message` - new direct message
- `message_read` - a message was marked as read
- `typing` - typing indicator

### Directory Layout
```
src/
├── main.rs              # Entry: init DB, LLM provider, ServiceManager, server, CLI
├── config.rs            # AppConfig::load() — env + TOML config loading
├── config/file.rs       # TOML file-based configuration provider
├── db.rs                # init_db() creates PgPool + pgvector extension + schema
├── cli.rs               # Interactive CLI loop
├── utils.rs             # Money helpers: yuan_to_cents(), cents_to_yuan()
├── api/                 # Axum router + handlers
│   ├── mod.rs           # AppState, create_router, /api/chat handler
│   ├── error.rs         # ApiError enum with HTTP status mappings
│   ├── auth.rs          # JWT register/login/refresh
│   ├── listings.rs      # Listing CRUD + AI item recognition
│   ├── orders.rs        # Order management + state transitions
│   ├── user.rs          # Profile, user listings, user search
│   ├── user_chat.rs     # User-to-user chat (connection handshake, messages)
│   ├── ws.rs            # WebSocket handler + broadcast
│   ├── conversations.rs # Conversation listing + pagination
│   ├── negotiate.rs     # Negotiation endpoints (HITL workflow)
│   ├── notifications.rs # Notification listing/marking
│   ├── watchlist.rs     # Watchlist add/remove
│   ├── recommendations.rs # Feed + similar listings (pgvector)
│   ├── upload.rs        # OSS upload token generation
│   ├── admin.rs         # Admin: stats, ban, takedown, audit logs
│   ├── metrics.rs       # Prometheus /metrics endpoint
│   └── stats.rs         # Public site statistics
├── middleware/
│   ├── rate_limit/      # Token bucket rate limiter
│   │   ├── mod.rs       # RateLimiterFactory, RateLimitStateHandle
│   │   ├── local.rs     # In-memory rate limiter (default)
│   │   ├── redis_backend.rs # Redis backend (optional)
│   │   └── traits.rs    # RateLimiter trait
│   └── admin.rs          # Admin authentication middleware
├── agents/
│   ├── mod.rs           # Module declarations
│   ├── router.rs        # IntentRouter for lightweight intent classification
│   ├── tools.rs         # Rig Tool implementations (CreateListing, UpdateListing, etc.)
│   ├── models.rs        # Domain models: ListingDetails, Document
│   └── negotiate.rs     # Negotiation agent: HitlRequest, HumanApprovalTool, etc.
├── llm/                 # LLM provider implementations
│   ├── mod.rs           # LlmProvider trait, PREAMBLE + NEGOTIATION_PREAMBLE constants
│   ├── gemini.rs        # GeminiProvider (Gemini chat + Gemini embeddings)
│   └── minimax.rs       # MiniMaxProvider (MiniMax chat + Gemini embeddings)
├── repositories/        # Data access layer (trait + concrete implementations)
│   ├── mod.rs           # Exports
│   ├── traits.rs        # Repository trait definitions
│   ├── auth_repo.rs     # PostgresAuthRepository
│   ├── chat_repo.rs     # PostgresChatRepository
│   ├── listing_repo.rs  # PostgresListingRepository
│   ├── order_repo.rs    # PostgresOrderRepository
│   └── user_repo.rs     # PostgresUserRepository
└── services/            # Business logic + event loop + workers
    ├── mod.rs           # ServiceManager, BusinessEvent, run_event_loop
    ├── product.rs       # ProductService (DISABLED — DealReached event disabled)
    ├── order.rs         # OrderService: create, status transitions, access verify
    ├── chat.rs          # ChatService: connection lifecycle, message history
    ├── settlement.rs    # SettlementService (DISABLED)
    ├── notification.rs  # NotificationService
    ├── admin.rs         # AdminService: audit logging, user banning, takedown
    ├── hitl_expire.rs   # HITL negotiation expiration worker (48h timeout)
    ├── order_worker.rs  # Order lifecycle worker (30min payment timeout, 7d auto-confirm)
    ├── moderation.rs    # ModerationService: text/image content moderation
    └── moderation_worker.rs  # Background image moderation worker (IMAN API)
```

## Key Patterns

- **Tool naming**: `{Action}{Entity}Tool` (`CreateListingTool`) and `{Action}{Entity}Args`
- **Service naming**: `{Domain}Service` pattern
- **Repository pattern**: Data access abstracted behind traits in `src/repositories/`
- **Errors**: `anyhow::Result` for app-level, `thiserror::Error` for domain types
- **Async**: All DB via sqlx (async), `Arc<dyn LlmProvider>` for provider abstraction
- **Event bus**: Fire-and-forget sends via bounded channel (capacity 2048) for backpressure
- **Money**: All prices stored as `i32` cents internally, converted to `f64` yuan via `cents_to_yuan()` for display
- **Auth**: JWT in Authorization header; agents receive `current_user_id: Option<String>`
- **Rate limiting**: Token bucket per IP via `RateLimitStateHandle` in `AppState.infra`
- **Database integrity**: FK constraints enforced in Postgres; `ON DELETE CASCADE` on all FK columns
- **Config**: All env vars loaded and validated once at startup via `config::AppConfig::load()`
- **Intent routing**: `IntentRouter` for lightweight classification before LLM calls

## Code Style

All code must pass `cargo fmt` and `cargo clippy -- -D warnings`. Chinese (Simplified) for all user-facing strings and agent prompts. See `AGENTS.md` for full style guide including import ordering, naming conventions, and async patterns.

## Code Review Requirements

Before committing:
- [ ] `cargo fmt` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] No hardcoded secrets (API keys, passwords, tokens)
- [ ] All user inputs validated
- [ ] SQL injection prevention (parameterized queries)
- [ ] Error messages don't leak sensitive data

## Agent Team

This project uses a **multi-agent orchestration** approach where specialized agents handle different aspects of development:

| Agent | Purpose | When to Invoke |
|-------|---------|---------------|
| `architect` | System design, architecture decisions | Complex features, refactoring |
| `planner` | Implementation planning, task breakdown | Feature implementation, multi-file changes |
| `rust-reviewer` | Rust ownership, lifetimes, concurrency safety | Any Rust code change |
| `flutter-reviewer` | Flutter widget best practices, state management | Any Flutter/Dart code change |
| `security-reviewer` | Security vulnerability detection | Auth, payments, user input handling |
| `code-reviewer` | General code quality, consistency | All code changes |
| `build-error-resolver` | Compilation errors, type errors, lint failures | Build fails |
| `tdd-guide` | Test-driven development, test coverage | Bug fixes, new features |
| `e2e-runner` | End-to-end testing, critical user flows | Integration testing |
| `refactor-cleaner` | Dead code cleanup, consolidation | Maintenance tasks |
| `doc-updater` | Documentation updates | Code changes requiring docs |

See [DEVELOPER.md](DEVELOPER.md) for the full multi-agent workflow, team responsibilities, and development guidelines.
