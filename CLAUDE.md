# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Agentic secondhand marketplace for Chinese university campuses. Built with Rust + Axum + PostgreSQL (relational + vector via pgvector) + Flutter mobile. AI-powered via Google Gemini or MiniMax LLM providers, using the Rig framework.

## Build and Run Commands

```bash
cargo build && cargo run          # Starts HTTP server on :3000 + interactive CLI
cargo check                       # Type-check without building
cargo clippy -- -D warnings       # Lint
cargo fmt && cargo fmt -- --check # Format and check
cargo test [test_name]            # Run tests (no test suite yet — prototype stage)
```

Requires `.env` with `GEMINI_API_KEY` (or `MINIMAX_API_KEY`), `JWT_SECRET`, and `DATABASE_URL`. See `AGENTS.md` for Flutter mobile development.

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `GEMINI_API_KEY` | Yes* | — | Google Gemini API key (*required unless using minimax with gemini for embeddings) |
| `JWT_SECRET` | Yes | — | Secret for JWT token signing |
| `LLM_PROVIDER` | No | `gemini` | `gemini` or `minimax` |
| `MINIMAX_API_KEY` | If LLM_PROVIDER=minimax | — | MiniMax API key |
| `MINIMAX_API_BASE_URL` | No | — | MiniMax API base URL |
| `VECTOR_DIM` | No | `768` | Embedding vector dimensions |

## Architecture

### Database
The codebase uses a **single PostgreSQL connection** (`sqlx::PgPool`) for both relational and vector data:
- Relational tables: `users`, `inventory`, `orders`, `chat_messages`
- Vector storage: `pgvector` extension (created via `CREATE EXTENSION IF NOT EXISTS vector`)

### Runtime Modes
`main.rs` runs two async tasks:
1. **Axum HTTP server** on `127.0.0.1:3000`
2. **CLI** (`cli::run_cli`) — spawned as background task, exits immediately in non-TTY

Shutdown via Ctrl+C aborts both tasks via stored `JoinHandle`s.

### Event-Driven Services
`services::ServiceManager` owns all services and runs a background event loop:
- Receives `BusinessEvent` variants via `tokio::sync::mpsc::Receiver`
- Spawns `tokio::spawn` tasks per event to call appropriate service methods
- Events: `DealReached`, `OrderPaid`, `ChatMessage`
- Uses bounded channel (capacity 2048) for backpressure

### LLM Provider System
`llm::LlmProvider` is a unified trait abstracting over Gemini and MiniMax:
- `create_marketplace_agent()` — creates RAG-enabled marketplace agent
- `create_negotiate_agent()` — creates negotiation agent
- `llm/gemini.rs` and `llm/minimax.rs` are concrete implementations
- Agent preamble is Chinese (Simplified) — see `llm/mod.rs::PREAMBLE`

### API Structure
`AppState` (Clone) passed to all Axum handlers:
```rust
pub struct AppState {
    pub db: PgPool,
    pub llm_provider: Arc<dyn LlmProvider>,
    pub event_tx: mpsc::Sender<BusinessEvent>,
    pub rate_limit: RateLimitStateHandle,
    pub jwt_secret: String,
    pub gemini_api_key: String,
}
```

Routes: `/api/health`, `/api/chat` (auth required, rate-limited), `/api/auth/*`, `/api/listings/*`, `/api/user/*`

### Directory Layout
```
src/
├── main.rs              # Entry: init DB, LLM provider, ServiceManager, server, CLI
├── config.rs            # AppConfig::load() — validates all env vars at startup
├── db.rs                # init_db() creates PgPool + pgvector extension + schema
├── cli.rs               # Interactive CLI loop
├── utils.rs             # Money helpers: yuan_to_cents(), cents_to_yuan()
├── api/                 # Axum router + handlers
│   ├── mod.rs           # AppState, create_router, /api/chat handler
│   ├── auth.rs          # JWT register/login
│   ├── error.rs         # ApiError enum
│   ├── listings.rs      # Listing CRUD + item recognition
│   └── user.rs          # Profile, user listings
├── middleware/
│   └── rate_limit.rs    # Token bucket rate limiter (20 req/min per IP)
├── agents/              # (agent definitions moved to llm/gemini.rs / llm/minimax.rs)
│   ├── marketplace.rs   # Placeholder — see llm/gemini.rs for actual agent building
│   ├── tools.rs         # Tool implementations (used by llm providers)
│   └── models.rs        # Domain models: ListingDetails, Document
├── llm/                 # LLM provider implementations
│   ├── mod.rs           # LlmProvider trait, PREAMBLE constants
│   ├── gemini.rs        # GeminiProvider (Gemini + pgvector)
│   └── minimax.rs       # MiniMaxProvider (MiniMax chat + Gemini embeddings)
└── services/            # Business logic + event loop
    ├── mod.rs           # ServiceManager, BusinessEvent, run_event_loop
    ├── product.rs       # ProductService
    ├── order.rs         # OrderService
    ├── chat.rs          # ChatService
    └── settlement.rs    # SettlementService (stub)
```

## Key Patterns

- **Tool naming**: `{Action}{Entity}Tool` (`CreateListingTool`) and `{Action}{Entity}Args`
- **Service naming**: `{Domain}Service` pattern
- **Errors**: `anyhow::Result` for app-level, `thiserror::Error` for domain types
- **Async**: All DB via sqlx (async), `Arc<dyn LlmProvider>` for provider abstraction
- **Event bus**: Fire-and-forget sends (`let _ = tx.try_send(...)`) in API handlers; bounded channel provides backpressure
- **Money**: All prices stored as `i64` cents internally, converted to `f64` yuan only for display
- **Auth**: `/api/chat` extracts user ID from JWT in Authorization header; agents receive `current_user_id: Option<String>`
- **Rate limiting**: Token bucket per IP via `RateLimitStateHandle` in `AppState`, applied to `/api/chat`
- **Database integrity**: FK constraints enforced in Postgres; `ON DELETE CASCADE` on all FK columns
- **Config**: All env vars loaded and validated once at startup via `config::AppConfig::load()`; `Arc<AppConfig>` passed around, not individual fields

## Code Style

All code must pass `cargo fmt` and `cargo clippy -- -D warnings`. Chinese (Simplified) for all user-facing strings and agent prompts. See `AGENTS.md` for full style guide including import ordering, naming conventions, and async patterns.
