# AGENTS.md — good4ncu

Agentic information sharing platform built with Rust, Rig framework, and Gemini LLM.
Campus-oriented information publishing platform with AI agents for listing, search, and RAG-based semantic retrieval.

**Disclaimer:** 本产品仅做信息发布，无担保和资金中介，也不收手续费。

Monorepo: Rust backend + Flutter mobile frontend.

## Build / Run / Test

```bash
# Build
cargo build

# Build (release)
cargo build --release

# Run (starts web server on :3000 + interactive CLI)
# Requires GEMINI_API_KEY in .env
cargo run

# Check (type-check without building)
cargo check

# Clippy (linting)
cargo clippy -- -D warnings

# Format
cargo fmt

# Format check (CI-friendly)
cargo fmt -- --check

# Run all tests
cargo test

# Run a single test by name
cargo test test_name_here

# Run tests in a specific module
cargo test module_name::

# Run tests with output
cargo test -- --nocapture

# Flutter mobile app
cd mobile && flutter pub get && flutter run
```

No CI pipeline exists yet. Unit tests are run locally with `cargo test`.

## Environment

- Rust edition 2021, Cargo lock version 4
- Configuration via `.env` file (secrets) + `good4ncu.toml` (non-secret config)
- **Config file search order:** `$CONFIG_FILE` env var → `./good4ncu.toml` → `./config/good4ncu.toml`
- PostgreSQL database (relational + pgvector for vector similarity search) — connection via `DATABASE_URL`
- Dependencies from crates.io: `rig-core` 0.33.0, `rig-postgres` 0.2.2
- Uses `rustls` TLS backend for reqwest (not native-tls) — required for proxy compatibility
- Flutter SDK required for mobile app development

### TOML Configuration

Non-secret settings can be set in `good4ncu.toml`:

```toml
[server]
host = "127.0.0.1"
port = 3000

[llm]
provider = "gemini"      # or "minimax"
vector_dim = 768

[rate_limit]
max_requests = 100
window_secs = 60
redis_url = ""           # Leave empty for local in-memory limiter

[event_bus]
capacity = 2048          # Bounded channel capacity for BusinessEvent

[workers.hitl_expire]
scan_interval_secs = 600  # 10 minutes
expire_timeout_hours = 48

[cors]
origins = ["*"]          # Comma-separated list

[marketplace]
conversation_history_limit = 10
price_tolerance = 0.50    # ±50% tolerance for negotiated prices

[auth]
access_token_ttl_secs = 86400      # 24 hours
refresh_token_ttl_secs = 604800    # 7 days

[moderation]
blocked_keywords = ""    # Comma-separated

[oss]
endpoint = "https://oss-cn-beijing.aliyuncs.com"
bucket = "good4ncu"
role_arn = ""             # STS role ARN for temporary credentials
```

## Architecture

```
src/                         # Rust backend
├── main.rs                  # Entry point: DB init, LLM provider, ServiceManager, Axum server, CLI
├── db.rs                    # PostgreSQL + pgvector init (sqlx pool, CREATE EXTENSION)
├── cli.rs                   # Interactive CLI menu (inquire)
├── config.rs                # AppConfig loading and validation (env + TOML)
├── config/file.rs           # TOML file-based configuration provider
├── utils.rs                 # Money helpers: yuan_to_cents(), cents_to_yuan()
├── api/
│   ├── mod.rs               # AppState (with infra/secrets/agents sub-structs), create_router
│   ├── error.rs             # ApiError enum with HTTP status mappings
│   ├── auth.rs              # JWT authentication (register, login, refresh)
│   ├── listings.rs          # Listing CRUD + item recognition + search
│   ├── orders.rs            # Order management + state machine transitions
│   ├── user.rs              # User profiles, listings, search
│   ├── user_chat.rs         # User-to-user chat with connection handshake
│   ├── ws.rs                # WebSocket handler + broadcast
│   ├── conversations.rs     # Conversation listing + pagination
│   ├── negotiate.rs         # Negotiation endpoints (HITL workflow)
│   ├── notifications.rs     # Notification listing/marking
│   ├── watchlist.rs         # Watchlist add/remove
│   ├── recommendations.rs   # Feed + similar listings (pgvector cosine similarity)
│   ├── upload.rs            # OSS upload token generation
│   ├── admin.rs             # Admin-only: stats, ban, takedown, audit logs
│   ├── metrics.rs           # Prometheus /metrics endpoint
│   └── stats.rs             # Public site statistics
├── agents/
│   ├── mod.rs               # Module declarations
│   ├── router.rs            # IntentRouter for lightweight intent classification
│   ├── tools.rs             # Rig Tool implementations
│   ├── models.rs            # Domain models: ListingDetails, Document (with Embed)
│   └── negotiate.rs         # Negotiation agent: HitlRequest, HumanApprovalTool, HitlChannel
├── llm/
│   ├── mod.rs               # LlmProvider trait + PREAMBLE + NEGOTIATION_PREAMBLE constants
│   ├── gemini.rs            # GeminiProvider (Gemini chat + Gemini embeddings)
│   └── minimax.rs           # MiniMaxProvider (MiniMax chat + Gemini embeddings)
├── repositories/            # Data access layer
│   ├── mod.rs               # Exports
│   ├── traits.rs            # Repository traits
│   ├── auth_repo.rs         # PostgresAuthRepository
│   ├── chat_repo.rs         # PostgresChatRepository
│   ├── listing_repo.rs      # PostgresListingRepository
│   ├── order_repo.rs        # PostgresOrderRepository
│   └── user_repo.rs         # PostgresUserRepository
├── services/
│   ├── mod.rs               # ServiceManager, BusinessEvent enum, run_event_loop
│   ├── product.rs           # ProductService (DISABLED)
│   ├── order.rs             # OrderService
│   ├── chat.rs              # ChatService
│   ├── notification.rs      # NotificationService
│   ├── settlement.rs         # SettlementService (DISABLED)
│   ├── admin.rs             # AdminService (audit logging, ban, takedown)
│   ├── hitl_expire.rs       # HITL expiration worker (48h timeout, 10min scan interval)
│   └── order_worker.rs      # Order lifecycle worker (30min payment timeout, 7d auto-confirm)
└── middleware/
    ├── mod.rs               # Middleware exports
    ├── rate_limit/
    │   ├── mod.rs           # RateLimiterFactory, RateLimitStateHandle
    │   ├── local.rs         # In-memory token bucket rate limiter
    │   ├── redis_backend.rs # Redis backend for distributed rate limiting
    │   └── traits.rs        # RateLimiter trait
    └── admin.rs             # Admin authentication middleware

mobile/                      # Flutter mobile app (only lib/ and config tracked in git)
├── lib/
│   ├── main.dart            # App entry point
│   ├── pages/               # All pages (home, login, chat, listing_detail, etc.)
│   ├── services/            # API clients by domain (api_service, auth_service, etc.)
│   ├── providers/            # Provider state management
│   └── models/models.dart   # Dart data models
├── pubspec.yaml             # Flutter dependencies
└── analysis_options.yaml    # Dart lint rules
```

Key patterns:
- Event-driven architecture via `tokio::sync::mpsc` bounded channel (2048 capacity)
- `ServiceManager` runs a background event loop processing `BusinessEvent` variants:
  - `DealReached` → creates order atomically (listing sold + order insert in tx)
  - `OrderPaid` → logs only (no-op)
  - `ChatMessage` → logs to chat
- Repository layer: `src/repositories/` provides trait-based data access
- `IntentRouter` classifies intent before LLM calls (blocks content, greets, etc.)
- Agents are built using Rig's `AgentBuilder` with `.tool()` and `.dynamic_context()` for RAG
- AppState uses sub-structs: `secrets` (config), `infra` (runtime), `agents` (LLM + router)
- HITL negotiation: `HumanApprovalTool` creates `HitlRequest` rows, worker scans for expired pending requests

## Git Policy

- Platform scaffolding (`mobile/android/`, `mobile/ios/`, `mobile/web/`, `mobile/linux/`, `mobile/macos/`, `mobile/windows/`, `mobile/test/`) is gitignored — regenerate with `flutter create`
- Generated files (`pubspec.lock`, `generated_plugin_registrant.*`) are gitignored
- `.env`, `.DS_Store` are gitignored — never commit secrets
- Commit style: conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, `style:`)

## Code Review Checklist

Before every commit:
- [ ] `cargo fmt` passes
- [ ] `cargo clippy -- -D warnings` passes (treat warnings as errors)
- [ ] No hardcoded secrets (API keys, passwords, tokens in source)
- [ ] All user inputs validated at system boundaries
- [ ] SQL injection prevention (parameterized queries only)
- [ ] Error messages don't leak internal paths or sensitive data
- [ ] New public APIs have doc comments (`///`)

## Code Style

### Imports
- Group imports: std → external crates → internal (`crate::` / `super::`)
- One `use` per line or grouped with braces from the same crate
- Alphabetical order within groups (enforced by `cargo fmt`)
- Prefer specific imports over glob imports (exception: `use super::tools::*` in marketplace.rs)

### Naming
- Types/structs: `PascalCase` — `ToolContext`, `BusinessEvent`, `CreateListingTool`
- Functions/methods: `snake_case` — `run_marketplace_agent`, `create_listing`
- Constants: `SCREAMING_SNAKE_CASE` (Rig tool names use `const NAME: &'static str`)
- Modules: `snake_case` file names
- Tool structs follow `{Action}{Entity}Tool` pattern — `CreateListingTool`, `SearchInventoryTool`
- Tool args follow `{Action}{Entity}Args` pattern — `CreateListingArgs`
- Services follow `{Domain}Service` pattern — `ChatService`

### Types
- Use `anyhow::Result` for application-level error propagation
- Use `thiserror::Error` for domain-specific error types (e.g., `ToolError`, `HumanInteractionError`)
- Derive `Clone` on service structs and tool structs (they hold `sqlx::PgPool` / `Arc` handles)
- Use `serde::{Serialize, Deserialize}` for all data transfer types
- Use `schemars::JsonSchema` for types used with Rig's extractor
- Use `sqlx::FromRow` for database row types (keep them private to the module)
- Use `rig::Embed` derive for vector store documents

### Error Handling
- Propagate errors with `?` operator — do not use `.unwrap()` in business logic
- `.unwrap()` / `.expect()` only for infallible operations or startup config (e.g., env vars, client builders)
- Tool errors: map with `.map_err(|e| ToolError(format!("context: {}", e)))?`
- Print user-facing errors with emoji prefix: `println!("❌ Agent error: {}\n", e)`
- Event bus sends use `let _ = tx.send(...)` — fire-and-forget, ignore send errors

### Formatting
- All code must pass `cargo fmt` and `cargo clippy -- -D warnings`
- 4-space indentation (Rust default)
- No trailing semicolons on final expressions returned from blocks
- Use `r#"..."#` raw strings for multi-line SQL
- Section separators in large files: `// ---------------------------------------------------------------------------`
- Doc comments (`///`) on public functions and key structs

### Async
- All DB operations are async (sqlx queries with `PgPool`)
- Use `tokio::spawn` for background tasks (event loop, web server)
- Use `mpsc::UnboundedSender` / `UnboundedReceiver` for event bus
- Shared state across async boundaries uses `Arc<T>` (e.g., `Arc<Connection>`)

### Database
- PostgreSQL with pgvector extension: relational data + vector similarity search in one DB
- UUIDs as TEXT primary keys (generated via `uuid::Uuid::new_v4().to_string()`)
- Soft deletes: `status` column with values `'active'`, `'deleted'`
- SQL uses `$1, $2` bind parameters (PostgreSQL style) — never interpolate user input directly into SQL strings
- Schema defined inline in `db::init_db()` with `CREATE TABLE IF NOT EXISTS` and `CREATE EXTENSION IF NOT EXISTS vector`

### API (Axum)
- State passed via `AppState` struct with `#[derive(Clone)]`
- CORS: permissive (`Any` origin/methods/headers) — prototype only
- Request/response types: private structs with `Deserialize` / `Serialize`
- Handlers return `Json<T>` directly
- Errors returned via `ApiError` enum → structured JSON responses with HTTP status codes

### Rig Framework
- Agent model: `gemini-3-flash-preview`
- Embedding model: `gemini::EMBEDDING_001` with 768 dimensions
- Preamble written in Chinese (target audience is Chinese campus users)
- Tools implement `rig::tool::Tool` trait with `definition()` returning JSON schema and `call()` for execution
- RAG via `.dynamic_context(n, index)` on agent builder — retrieves top-n similar documents

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `rig-core` | 0.33.0 | LLM agent framework |
| `rig-postgres` | 0.2.2 | Vector store integration (pgvector) |
| `axum` | 0.8 | HTTP server |
| `sqlx` | 0.8.6 | Async SQL (PostgreSQL) |
| `pgvector` | 0.4 | Vector similarity search extension |
| `serde` / `serde_json` | 1.0 | Serialization |
| `schemars` | 1.0.4 | JSON Schema generation for Rig extractors |
| `anyhow` / `thiserror` | 1.0 | Error handling |
| `tokio` | 1.34 | Async runtime |
| `reqwest` | 0.13 | HTTP client (rustls backend) |
| `inquire` | 0.7.5 | Interactive CLI prompts |
| `chrono` | 0.4 | Timestamps |
| `uuid` | 1.22.0 | ID generation |
| `dotenvy` | 0.15 | .env file loading |
| `jsonwebtoken` | 9.3 | JWT encoding/decoding |
| `argon2` | 0.5 | Password hashing |
| `tokio-tungstenite` | 0.28 | WebSocket server |
| `tower` / `tower-http` | 0.5 | Axum middleware (CORS, rate limit) |
| `prometheus` | 0.13 | Metrics collection |
| `moka` | 0.12 | Local rate limiter cache |
| `sha2` / `hmac` / `hex` | various | OSS signing |
| `base64` | 0.22 | Media data encoding |
| `lazy_static` | 1.5 | Static initialization |
| `async-stream` | 0.3 | Async iterators for SSE |
| `futures` / `futures-util` | 0.3 | Async combinators |

## Notes

- Agent preambles and user-facing strings are in Chinese (Simplified)
- Flutter platform directories are gitignored — regenerate with `cd mobile && flutter create .`
- Database: PostgreSQL with pgvector for relational + vector storage
- Rate limiting: token bucket per IP, whitelisted paths include health/metrics/chat read endpoints
