# good4ncu

Agentic secondhand marketplace for campus communities.

## Overview

AI-powered buy/sell platform built with Rust, [Rig](https://rig.rs/) framework, and Google Gemini LLM. Features intelligent listing creation, semantic search via RAG, and automated buyer-seller negotiation through LLM agents.

Designed for Chinese university campuses — all agent prompts and user-facing strings are in Simplified Chinese.

## Architecture

- **Backend**: Rust + Axum (HTTP server on `:3000`) + SQLite (relational via `sqlx`, vector via `sqlite-vec`)
- **Mobile**: Flutter / Dart
- **AI**: Google Gemini (`gemini-3-flash-preview`) with `EMBEDDING_001` (768-dim) for vector search
- **Agent Framework**: [Rig](https://github.com/0xPlaygrounds/rig) — tools, extractors, RAG via `dynamic_context`

```
src/
├── main.rs              # Entry point: DB, Gemini client, event bus, Axum server, CLI
├── db.rs                # SQLite init (sqlx pool + rusqlite/sqlite-vec)
├── cli.rs               # Interactive CLI menu
├── api/mod.rs           # REST API (health, chat with multimodal support)
├── agents/              # LLM agent definitions
│   ├── models.rs        # Domain models (ListingDetails, Document with Embed)
│   ├── tools.rs         # Rig Tool impls (CRUD listings, search, purchase)
│   ├── marketplace.rs   # Marketplace agent builder with RAG + tools
│   └── negotiate.rs     # Auto-negotiation with human-in-the-loop
└── services/            # Business logic layer
    ├── mod.rs           # ServiceManager, BusinessEvent enum, event loop
    ├── product.rs       # ProductService
    ├── order.rs         # OrderService
    ├── chat.rs          # ChatService
    └── settlement.rs    # SettlementService (stub)

mobile/                  # Flutter mobile app
```

## Getting Started

### Prerequisites

- Rust (edition 2021)
- Flutter SDK (for mobile app)
- Google Gemini API key

### Setup

```bash
# Create .env file
echo "GEMINI_API_KEY=your_key_here" > .env

# Build and run (starts web server on :3000 + interactive CLI)
cargo run
```

### Mobile App

```bash
cd mobile
flutter pub get
flutter run
```

## Status

Prototype stage — no auth, no HTTP error codes, no test suite, permissive CORS.
