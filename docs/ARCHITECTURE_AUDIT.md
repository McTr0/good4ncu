# Good4NCU Architecture Audit

> Consolidated architecture diagnosis. Last updated: 2026-04-02.

## Summary

The project is materially healthier than the early Q1 baseline: CI is active, admin and auth regressions are covered, token denylisting exists, and chat/media observability improved. The main remaining risk is not basic deployability; it is the cost of carrying oversized modules and transitional data-model choices.

## High-Priority Findings

### Oversized chat backend surface

- `src/api/user_chat.rs` is smaller than before but still large, and `src/api/user_chat/message.rs` is also substantial.
- Responsibility is still split across router glue, authorization, SQL, broadcast behavior, and compatibility logic.
- Risk: behavior drift during future changes and high review cost.

### Oversized Flutter pages

- `mobile/lib/pages/admin_page.dart` and `mobile/lib/pages/user_chat_page.dart` are both still over 1,000 lines.
- They mix layout, networking, and state transitions in the same files.
- Risk: poor testability, slow iteration, and fragile UI changes.

### Core IDs still use `TEXT`

- `users`, `inventory`, and `orders` still rely on string IDs rather than native `UUID`.
- Risk: slower joins, larger indexes, and a more complex long-term migration path.

### Transaction propagation remains uneven

- Some critical multi-table flows still depend on service-layer coordination instead of explicit transaction-aware repository boundaries.
- Risk: partial writes and hard-to-prove rollback behavior.

## Medium-Priority Findings

### `AppState` remains broad

- Infra, secrets, agents, and repositories are still carried together through a wide application state object.
- This is workable, but it keeps handler-level tests and narrower dependency boundaries harder than they need to be.

### Chat conversation typing is still mixed

- `chat_connections.id` is `UUID`, while `chat_messages.conversation_id` remains `TEXT` to support special values such as `__agent__` and `global`.
- Index and join-path mitigations exist, but the mismatch still complicates data integrity and query design.

### Base64 media fallback is only partially retired

- Chat is URL-first now, but compatibility fallback still allows large Base64 payloads into the database path.
- Risk: table growth, request-size pressure, and operational inconsistency.

### Runtime guards are incomplete

- `vector_dim` is configurable, but there is no clear startup-time guard that fails fast when schema expectations drift from configuration.

## Resolved Or Largely Mitigated

- Impersonation token revocation gap: mitigated by token denylisting.
- Refresh replay gap: mitigated.
- Admin route authorization regressions: covered by explicit tests.
- Chat connection split and duplicate connection handling: mitigated by routing cleanup and bidirectional uniqueness.
- WebSocket observability blind spots: mitigated by dropped and pruned metrics plus warning logs.

## Recommended Direction

- Use [`PLAN.md`](./PLAN.md) as the execution source of truth.
- Keep this document focused on diagnosis, not backlog duplication.
- Add new findings here only when they change risk understanding or architectural direction.
