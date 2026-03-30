# DB Migration Plan: TEXT IDs → UUID (Zero/Low Downtime)

> Status: Ready for implementation
> Updated: 2026-03-29
> Scope: `users`, `inventory`, `orders` and their FK graph

## Goal

Migrate core primary/foreign keys from `TEXT` to native `UUID` with rollback safety and minimal lock time.

## Strategy (Staged)

1. **Add shadow UUID columns** (`new_id`, `new_*_id`) without breaking old reads/writes.
2. **Backfill in batches** and create concurrent indexes.
3. **Add FK as NOT VALID**, then `VALIDATE CONSTRAINT`.
4. **Dual-write window** in application (old+new columns).
5. **Atomic metadata swap** (rename columns, switch PK/FK/indexes).
6. **Rollback window** keep old columns temporarily.
7. **Cleanup** old columns after stability window.

## Table Dependency Order

1. `users` (root)
2. `inventory` (depends on users)
3. `orders` (depends on users + inventory)

## Migration Work Units

### WU-1: Shadow columns + generators

- Add `new_id UUID` for each PK table.
- Add `new_user_id`, `new_inventory_id` where needed for FK tables.
- Use `gen_random_uuid()` for defaults during transition.

### WU-2: Backfill and indexes

- Batch update until no null in shadow columns.
- Create indexes with `CREATE INDEX CONCURRENTLY`.

### WU-3: FK transition

- Add FK on shadow columns using `NOT VALID`.
- Validate in off-peak window (`VALIDATE CONSTRAINT`).

### WU-4: App dual-write cutover

- Writes populate old+new IDs for one deployment window.
- Reads still from old columns until validation done.

### WU-5: Atomic swap

- In transaction: rename old/new columns and switch constraints/index aliases.
- Keep old columns as rollback fallback for one release cycle.

### WU-6: Cleanup

- Drop legacy columns and triggers after stability window.

## Operational Safeguards

- Run backfills in bounded batches.
- Run index creation concurrently.
- Add dashboard checks for FK violations and null shadow IDs.
- Keep reversible migration scripts for swap/rollback.

## Acceptance Criteria

- All PK/FK columns are native UUID.
- No table contains null in canonical ID/FK columns.
- All constraints validated.
- Production read/write pass with no dual-write divergence.

## Next Execution Tasks

1. Create SQL migration set `0013_*` to `0016_*` for shadow columns/backfill/index/fk.
2. Add temporary repository/service dual-write adapters.
3. Add migration verification script in CI.
