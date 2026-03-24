# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Agentic secondhand marketplace for Chinese university campuses. Built with Rust + Axum + SQLite (relational + vector) + Flutter mobile. AI-powered via Google Gemini and Rig framework.

## Build and Run Commands

```bash
cargo build && cargo run          # Starts HTTP server on :3000 + interactive CLI
cargo check                       # Type-check without building
cargo clippy -- -D warnings       # Lint
cargo fmt && cargo fmt -- --check # Format and check
cargo test [test_name]            # Run tests
```

Requires `.env` with `GEMINI_API_KEY` and `JWT_SECRET`. See `AGENTS.md` for detailed commands including Flutter mobile development.

## Architecture

### Dual SQLite Connections
The codebase uses two separate SQLite connections for different purposes:
- **`sqlx::SqlitePool`** (async) — relational queries, API endpoints, services
- **`tokio_rusqlite::Connection`** (sync via `Arc`) — `sqlite-vec` vector store for RAG

This is visible in `db::init_db()` and `ToolContext` which holds both.

### Runtime Modes
`main.rs` runs two async tasks via `tokio::select!`:
1. **CLI** (`cli::run_cli`) — interactive menu via `inquire`
2. **Axum HTTP server** — REST API on `:3000`

Shutdown is triggered by CLI exit or Ctrl+C, aborting both tasks via stored `JoinHandle`s.

### Event-Driven Services
`services::ServiceManager` owns all services and runs a background event loop:
- Receives `BusinessEvent` variants via `mpsc::UnboundedReceiver`
- Spawns `tokio::spawn` tasks per event to call appropriate service methods
- Events: `DealReached`, `OrderPaid`, `ChatMessage`

Key types:
- `BusinessEvent` enum — all event variants
- `ServiceManager` — holds services + `event_tx` sender (use `with_vector()` constructor to enable vector cleanup on sold items)
- Each service (`ProductService`, `OrderService`, etc.) owns a `SqlitePool`
- `ProductService` also holds optional `vector_conn` for cleaning up sold item embeddings from sqlite-vec

### Agent System (Rig + Gemini)
`agents::marketplace::create_marketplace_agent` builds the marketplace agent:
1. Creates `SqliteVectorStore` with `EMBEDDING_001` (768-dim)
2. Attaches RAG via `.dynamic_context(3, index)`
3. Registers all tools (CRUD, search, purchase)
4. Chinese preamble defining agent behavior

`ToolContext` is cloned and shared across all tools — contains db pool, vector conn, gemini client, event tx, current user ID.

### API Structure
`AppState` (Clone) is passed to all Axum handlers:
```rust
pub struct AppState {
    pub db: SqlitePool,
    pub vector_db: Arc<Connection>,  // sync, wrapped in Arc
    pub gemini: gemini::Client,
    pub event_tx: UnboundedSender<BusinessEvent>,
    pub rate_limit: RateLimitStateHandle,  // token bucket rate limiter
    pub jwt_secret: String,  // loaded once at startup via AppConfig
}
```

Routes: `/api/health`, `/api/chat` (auth required, rate-limited 20/min), `/api/auth/*`, `/api/user/*`

### Directory Layout
```
src/
├── main.rs              # Entry: init DB, Gemini, ServiceManager, server, CLI
├── config.rs            # Unified config: AppConfig::load() validates all env vars at startup
├── db.rs                # init_db() returns (SqlitePool, Connection), FK constraints enabled
├── cli.rs               # Interactive CLI loop
├── utils.rs             # Money helpers: yuan_to_cents(), cents_to_yuan()
├── api/                 # Axum router + handlers (auth, user, chat)
│   ├── mod.rs           # AppState, create_router, /api/chat handler (auth required)
│   ├── auth.rs          # JWT register/login
│   ├── error.rs         # ApiError enum (Unauthorized, RateLimitExceeded, etc.)
│   └── user.rs          # Profile, listings endpoints
├── middleware/           # Axum middleware
│   ├── mod.rs           # Module declarations
│   └── rate_limit.rs    # Token bucket rate limiter (20 req/min per IP)
├── agents/              # Rig agents and tools
│   ├── marketplace.rs   # create_marketplace_agent, run_marketplace_agent
│   ├── tools.rs         # All Tool impls (CreateListingTool, etc.)
│   ├── models.rs        # ListingDetails, Document (with rig::Embed)
│   └── negotiate.rs     # Auto-negotiation agent
└── services/            # Business logic + event loop
    ├── mod.rs           # ServiceManager, BusinessEvent, run_event_loop
    ├── product.rs        # ProductService
    ├── order.rs          # OrderService
    ├── chat.rs           # ChatService
    └── settlement.rs     # SettlementService (stub)
```

## Key Patterns

- **Tool naming**: `{Action}{Entity}Tool` (`CreateListingTool`) and `{Action}{Entity}Args`
- **Service naming**: `{Domain}Service` pattern
- **Errors**: `anyhow::Result` for app-level, `thiserror::Error` for domain types
- **Async**: All DB via sqlx (async), sqlite-vec operations via sync `Connection` wrapped in `Arc`
- **Event bus**: Fire-and-forget sends (`let _ = tx.send(...)`) in tools, proper receive in event loop
- **Money**: All prices stored as `i64` cents internally, converted to `f64` yuan only for display
- **Auth**: Agent tools require `current_user_id` to be `Some(...)` — they reject anonymous access; `/api/chat` enforces authentication (returns 401 if missing/invalid)
- **HITL (Human-in-the-Loop)**: Uses async channels (`HitlChannel`) — CLI handler runs in separate task, web context uses `new_disabled()` to reject gracefully
- **Rate limiting**: Token bucket per IP via `RateLimitStateHandle` in `AppState`, applied to `/api/chat`
- **Database integrity**: FK constraints enabled (`PRAGMA foreign_keys = ON`); all FK columns use `ON DELETE CASCADE` — user deletion cascades to their orders, inventory, and chat messages
- **Config**: All env vars loaded and validated once at startup via `config::AppConfig::load()`; `jwt_secret` passed through `AppState` to avoid per-module env reads

## Code Style

All code must pass `cargo fmt` and `cargo clippy -- -D warnings`. Chinese (Simplified) for all user-facing strings and agent prompts. See `AGENTS.md` for full style guide.
