# Good4NCU Spec Plans

> Subsystem-specific implementation and verification tracks. Last updated: 2026-04-02.

## UUID Migration Track

### Goal

Migrate core primary and foreign keys from `TEXT` to native `UUID` with rollback safety and minimal lock time.

Current stage: shadow columns, sync triggers, shadow FKs, and `uuid_shadow_divergence` observability are landed. Application reads still use legacy text IDs.

### Scope

- Root tables: `users`, `inventory`, `orders`
- Preserve application compatibility during the transition
- Avoid big-bang swaps

### Dependency Order

1. `users`
2. `inventory`
3. `orders`

### Work Units

1. Shadow columns and generators
   Done. `new_id` and `new_*_id` UUID columns, defaults, and sync triggers exist without breaking reads or writes.
2. Backfill and indexes
   Done for the current schema baseline. Existing rows are backfilled and indexed during migration.
3. FK transition
   Done. Shadow foreign keys were added and validated after backfill.
4. Dual-write window
   Partially done. Database triggers keep shadow UUIDs aligned while application writes still target canonical text IDs.
5. Atomic swap
   Rename columns and constraints in a tightly controlled cutover.
6. Cleanup
   Drop legacy columns only after a stability window and rollback expiry.

### Safeguards

- Use bounded backfill batches.
- Measure null shadow IDs and FK validation failures continuously.
- Block the swap until dual-write divergence is zero.
- Keep reversible swap and rollback scripts.

### Acceptance

- Canonical PK and FK columns are native UUID.
- No null values remain in canonical IDs.
- Constraints validate cleanly.
- Read and write traffic passes without dual-write drift.

## Chat System Verification Track

### Environment

- Backend running on `localhost:3000`
- Valid `DATABASE_URL`, `JWT_SECRET`, and LLM key
- `jq` available for manual API checks

### API Flow Scenarios

1. Register two users and capture tokens.
2. User A requests a chat connection with User B.
3. User B accepts and both users see `connected`.
4. User A attempts to accept and receives `403`.
5. User B rejects a fresh pending connection and both users see `rejected`.
6. A connected user sends a message and both users can fetch it.
7. Invalid connection IDs return `404`.
8. Invalid or expired JWTs return `401`.

### WebSocket Scenarios

1. Valid token connects successfully.
2. Invalid token is rejected.
3. Accepting a request emits the expected connection-established event.
4. Reconnect behavior remains correct when a user leaves and re-enters chat.

### UI Scenarios

1. Pending connections show the correct accept or reject affordances for the receiver.
2. Connected conversations show the message composer.
3. Pending outbound connections show the waiting state.
4. Rapid repeat actions do not create duplicate state transitions.
5. Two-browser and disconnected-socket cases converge back to the correct server state.

### Manual Smoke Commands

```bash
cargo run

curl -s -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"usera","password":"Test123!","phone":"13800000001"}' | jq .
```

Use this track to preserve behavior while chat handlers continue to move out of the monolithic entry module.
