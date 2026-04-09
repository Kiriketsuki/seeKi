# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration (Session 3)

Council verdict: **FOR** | 2026-04-09 | 4 findings (4 verified, 1 dismissed) | 1v1 no-questioner | 2 rounds

Prior sessions: Session 1 (8 findings, all fixed), Session 2 (9 findings, 2 pre-merge fixed, 7 tracked).

## No Fixes Required (pre-merge)

All pre-merge issues from prior sessions have been resolved. Session 3 found no new pre-merge blockers.

### Dismissed: setup.rs CWD write
- **File**: `src/api/setup.rs:155`
- **Reason**: CRITIC claimed new code in this PR. Arbiter verified via `git log main -- src/api/setup.rs` that `save_config` was merged in Epic 1 (commit c276345, PR #27). CWD write is symmetric with `config.rs:202-207` loader. Not in scope.

## Tracked (post-merge, not blocking)

### 1. Toolbar shows previous table name during loading
- **File**: `frontend/src/App.svelte:62`
- **Severity**: low
- **Fix**: Move `selectedTable = tableName` before the `await`, or accept as deliberate UX trade-off

### 2. Justfile dev recipe orphans cargo on Ctrl+C
- **File**: `Justfile:9-17`
- **Severity**: low
- **Fix**: Add `trap "kill $CARGO_PID 2>/dev/null" EXIT INT TERM` after backgrounding cargo

### 3. pick<T>() unsound on empty arrays
- **File**: `frontend/src/lib/mock.ts:326`
- **Severity**: nit
- **Fix**: Add empty-array guard or change return type to `T | undefined`

## Test Command
```bash
cd frontend && npm run build && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
