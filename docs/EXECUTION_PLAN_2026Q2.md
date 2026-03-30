# Good4NCU Execution Plan (2026 Q2)

Status: Active
Last Updated: 2026-03-30
Scope: Rust Axum backend + Flutter mobile + PostgreSQL/pgvector

## Context

This plan converts the architecture audit into a low-risk, PR-sized execution sequence.

Already completed baseline work in repo:
- Bidirectional uniqueness for chat connections.
- Chat media URL compatibility migration.
- Initial updated_at trigger rollout for core tables.

## Guiding Principles

- No big-bang refactors.
- Keep API contracts stable while splitting large modules.
- Add test and CI safety nets before deep changes.
- Prefer additive database changes (shadow columns, dual-write windows, reversible cutovers).

## Two-Week Plan

### Phase A (Day 1): Safety Net First
Goals:
- Enforce backend + mobile quality gates in CI.
- Lock merge baseline before major refactors.

Deliverables:
- CI includes: cargo fmt, cargo clippy -D warnings, cargo test.
- CI includes: flutter analyze, flutter test.

Acceptance Criteria:
- All checks are green on default branch for one full day.

### Phase B (Days 1-3): Chat API Decomposition Without Behavior Change
Goals:
- Reduce complexity of user_chat API module.
- Preserve route signatures and JSON contracts.

Deliverables:
- Extract chat models, connection handlers, message handlers into submodules.
- Keep router wiring stable.

Acceptance Criteria:
- Existing chat tests pass unchanged.
- Public routes and payloads unchanged.
- user_chat entry module shrinks materially.

### Phase C (Days 3-5): Transaction Boundaries + Mobile DI Wedge
Goals:
- Harden cross-table consistency on critical flows.
- Reduce direct service instantiation in key mobile pages.

Deliverables:
- First transaction helper/adapter applied to order and chat critical paths.
- Admin/UserChat/Router migrate from direct new Service() calls to injected dependencies.

Acceptance Criteria:
- Rollback tests confirm atomic behavior on forced failures.
- flutter analyze and targeted widget tests remain green.

### Phase D (Days 6-8): UUID Migration Work Units 1-3
Goals:
- Prepare UUID transition safely without metadata swap.

Deliverables:
- Shadow UUID columns.
- Backfill scripts and concurrent indexes.
- NOT VALID FK + VALIDATE constraints.

Acceptance Criteria:
- Shadow UUID columns have zero null values in staged scope.
- FK validation succeeds on staging.

### Phase E (Days 8-9): Dual-Write Window
Goals:
- Start old/new ID dual-write while reads remain stable.

Deliverables:
- Repository/service write paths dual-write IDs.
- Divergence checks and metrics.

Acceptance Criteria:
- Divergence remains zero during soak window.

### Phase F (Day 10): Stabilization and Go/No-Go
Goals:
- Verify rollback and runtime safety.

Deliverables:
- Rollback rehearsal docs and execution notes.
- Startup guard for vector dimension mismatch.

Acceptance Criteria:
- Rollback rehearsal passes.
- No P1 regressions in smoke tests.

## Week 1 PR Queue (PR-Sized Batches)

1. ci: add mobile analyze/test gates.
2. refactor(chat): extract DTO/event types from user_chat module.
3. refactor(chat): extract connection handlers.
4. refactor(chat): extract message handlers.
5. refactor(tx): transaction adapter for order/chat critical operations.
6. refactor(mobile-di): inject dependencies for Admin/UserChat/Router entrypoints.
7. feat(db): UUID shadow-column migration package (no swap).

## Top Risks and Mitigations

1. UUID dual-write divergence.
Mitigation: parity queries + dual-write metrics + block swap until divergence is zero.

2. Migration lock contention.
Mitigation: bounded batch updates, concurrent indexes, validate constraints off-peak.

3. Chat refactor behavior drift.
Mitigation: contract tests before/after extraction and route-level parity checks.

4. Transaction regressions.
Mitigation: forced-failure rollback tests in integration suite.

5. Vector dimension mismatch.
Mitigation: startup-time schema/config dimension check, fail fast.

## Tomorrow Morning Start (First 3 Tasks)

1. Merge CI gate expansion.
2. Open PR for user_chat DTO/event extraction only (behavior-neutral).
3. Add route contract assertions around extracted handlers before moving business logic.

## Definition of Done for This Cycle

- Week 1 PR queue merged with green CI.
- Chat module no longer monolithic at current scale.
- Critical transaction boundaries covered by rollback tests.
- UUID migration reaches validated shadow-column stage on staging.
- Mobile key flows no longer instantiate core services directly in target entry pages.
