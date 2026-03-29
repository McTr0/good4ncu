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
- Requires `.env` file with `GEMINI_API_KEY` and `DATABASE_URL` set (loaded via `dotenvy`)
- PostgreSQL database (relational + pgvector for vector similarity search) — connection via `DATABASE_URL`
- Dependencies from crates.io: `rig-core` 0.33.0, `rig-postgres` 0.2.2
- Uses `rustls` TLS backend for reqwest (not native-tls) — required for proxy compatibility
- Flutter SDK required for mobile app development

## Architecture

```
src/                         # Rust backend
├── main.rs                  # Entry point: DB init, LLM provider, event bus, Axum server, CLI
├── db.rs                    # PostgreSQL + pgvector init (sqlx pool, CREATE EXTENSION)
├── cli.rs                   # Interactive CLI menu (inquire)
├── config.rs                # AppConfig loading and validation
├── utils.rs                 # Money helpers: yuan_to_cents(), cents_to_yuan()
├── api/
│   ├── mod.rs               # AppState (with infra/secrets/agents sub-structs), create_router
│   ├── error.rs             # ApiError enum with HTTP status mappings
│   ├── auth.rs              # JWT authentication
│   ├── listings.rs          # Listing CRUD + item recognition
│   ├── user.rs              # User profiles
│   ├── user_chat.rs         # User-to-user chat with connection handshake
│   ├── ws.rs                # WebSocket handler + broadcast
│   ├── conversations.rs     # Conversation listing
│   ├── orders.rs            # Order management
│   ├── negotiate.rs         # Negotiation endpoints
│   ├── notifications.rs     # Notification endpoints
│   ├── watchlist.rs         # Watchlist endpoints
│   ├── recommendations.rs    # Recommendation feed
│   ├── upload.rs            # OSS upload token generation
│   ├── admin.rs             # Admin-only endpoints
│   ├── metrics.rs           # Prometheus metrics
│   └── stats.rs             # Site statistics
├── agents/
│   ├── mod.rs               # Module declarations
│   ├── router.rs            # IntentRouter for lightweight intent classification
│   ├── tools.rs             # Rig Tool implementations
│   └── models.rs            # Domain models: ListingDetails, Document (with Embed)
├── llm/
│   ├── mod.rs               # LlmProvider trait + PREAMBLE constants
│   ├── gemini.rs            # GeminiProvider (Gemini + pgvector)
│   └── minimax.rs           # MiniMaxProvider (MiniMax chat + Gemini embeddings)
├── repositories/            # Data access layer
│   ├── mod.rs               # Exports
│   ├── traits.rs            # Repository traits
│   ├── auth_repo.rs         # PostgresAuthRepository
│   ├── chat_repo.rs         # PostgresChatRepository
│   ├── listing_repo.rs      # PostgresListingRepository
│   └── user_repo.rs         # PostgresUserRepository
├── services/
│   ├── mod.rs               # ServiceManager, BusinessEvent enum, event loop
│   ├── product.rs           # ProductService
│   ├── order.rs             # OrderService
│   ├── chat.rs              # ChatService
│   ├── notification.rs     # NotificationService
│   ├── settlement.rs       # SettlementService
│   └── hitl_expire.rs       # HITL expiration worker
└── middleware/
    ├── mod.rs               # Middleware exports
    ├── rate_limit/
    │   ├── mod.rs           # RateLimiterFactory, RateLimitStateHandle
    │   ├── local.rs        # In-memory rate limiter
    │   └── redis_backend.rs # Redis rate limiter (optional)
    └── admin.rs             # Admin authentication middleware

mobile/                      # Flutter mobile app (only lib/ and config tracked in git)
├── lib/
│   ├── main.dart            # App entry point
│   ├── pages/chat_page.dart # Chat UI page
│   ├── services/api_service.dart # HTTP client to backend API
│   └── models/models.dart   # Dart data models
├── pubspec.yaml             # Flutter dependencies
└── analysis_options.yaml    # Dart lint rules
```

Key patterns:
- Event-driven architecture via `tokio::sync::mpsc` bounded channel (2048 capacity)
- `ServiceManager` runs a background event loop processing `BusinessEvent` variants
- Repository layer: `src/repositories/` provides trait-based data access
- `IntentRouter` classifies intent before LLM calls (blocks content, greets, etc.)
- Agents are built using Rig's `AgentBuilder` with `.tool()` and `.dynamic_context()` for RAG
- AppState uses sub-structs: `secrets` (config), `infra` (runtime), `agents` (LLM + router)

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

## Notes

- Agent preambles and user-facing strings are in Chinese (Simplified)
- Flutter platform directories are gitignored — regenerate with `cd mobile && flutter create .`
- Database: PostgreSQL with pgvector for relational + vector storage
- Rate limiting: token bucket per IP, whitelisted paths include health/metrics/chat read endpoints
