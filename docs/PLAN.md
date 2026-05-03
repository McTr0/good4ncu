# Good4NCU Plan

> Active planning source of truth. Last updated: 2026-05-03.

## Current Status

- CI and security baselines are in place: Rust/Flutter checks run in GitHub Actions, refresh replay is blocked, and impersonation tokens are denylisted.
- Chat media is URL-first, with upload metrics and compatibility fallback still present.
- Core table `updated_at` rollout and WebSocket observability landed.
- UUID shadow columns, sync triggers, and divergence checks landed for `users`, `inventory`, and `orders`.
- Startup now fails fast on both `documents.embedding` dimension mismatch and UUID shadow drift.
- Planning docs were consolidated here; this file replaces parallel roadmap and execution notes.
- `src/api/user_chat.rs` reduced to 41-line thin wiring; handlers decomposed into `connection.rs`, `message.rs`, `context.rs`, `events.rs`.
- AI chat handler extracted from `api/mod.rs` into `api/chat.rs`; `api/mod.rs` now owns routing and middleware only.
- `admin_page.dart` and `user_chat_page.dart` decomposed into notifier-backed state and extracted tab widgets.
- CI gains Rust dependency caching (`Swatinem/rust-cache`) and job dependency chains to skip downstream jobs on lint failure.

## Now

### 1. Harden Transaction Boundaries

- Introduce transaction-aware repository and service paths for multi-table order and chat flows.
- Remove remaining ad hoc cross-table SQL from service code where it prevents rollback guarantees.
- Add forced-failure integration tests that prove atomic behavior.

## Next

### 4. Native UUID Migration for Core Tables

- Shadow-column backfill, trigger-based dual-write, and divergence checks are in place for `users`, `inventory`, and `orders`.
- Next cutover step is application read/write adoption of canonical UUID columns before any metadata swap.
- Keep rollback and cleanup steps explicit. Detailed track lives in `SPEC_PLANS.md`.

### 5. Media Storage Cleanup

- Retire Base64 fallback as the preferred chat-media failure path.
- Move failed uploads toward client-side retry or deferred send instead of large DB-backed payloads.
- Keep URL-first upload paths observable during the transition.

### 6. Runtime and Capacity Safety

- Startup guards now cover `vector_dim` mismatch and UUID shadow drift.
- Revisit rate-limit identity handling where only IP-based enforcement still exists.

## Later

- Avatar moderation placeholder UX in mobile.
- Listing and recommendation thumbnail polish where placeholder rendering remains.
- Web token storage hardening.
- Payment and settlement integration after current modularity and data work stabilizes.

## Recently Completed

- Flutter analyze and test CI gates.
- Chat decomposition: `src/api/user_chat.rs` reduced to 41-line thin wiring; logic in `connection.rs`, `message.rs`, `context.rs`, `events.rs`.
- AI chat handler extracted from `api/mod.rs` into `api/chat.rs`; mod.rs now owns routing and middleware only.
- Mobile page decomposition: `admin_page.dart` and `user_chat_page.dart` split into notifier-backed state and extracted tab widgets.
- CI: Rust dependency caching (`Swatinem/rust-cache`) and job dependency chains added.
- Token denylist and refresh replay protection.
- Chat media URL compatibility migration and upload path metrics.
- Initial `updated_at` rollout for core tables.
- UUID shadow drift startup guard plus integration coverage.
- WebSocket dropped and pruned metrics plus warning logs.
- Admin impersonation UI and admin order status actions.

## Exit Criteria For This Cycle

- Critical transactional flows have rollback coverage with forced-failure integration tests.
- UUID migration reaches application read/write adoption with zero divergence in soak checks.
- Base64 media fallback retired; client-side retry or deferred send replaces it.
