# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration (Session 2)

Council verdict: **CONDITIONAL** | 2026-04-09 | 9 findings (9 verified, 0 dismissed) | 1v1 no-questioner | 2 rounds

Prior session: 8 findings, all 8 fixed (commits 5933501, 7f92f80, eb95851).

## Fixes Required (pre-merge)

### 1. sqlx missing chrono feature — timestamp/UUID columns silently null (CRITICAL)
- **File**: `Cargo.toml:17`
- **Related**: `src/db/postgres.rs:400-404`, `src/api/mod.rs:390-396`
- **Type**: bug
- **Severity**: critical
- **Verification**: verified — `humanize_type` at `postgres.rs:415` maps timestamp types to display names, confirming the app expects them, but sqlx features lack `chrono` so `try_get::<String>` fails for those OIDs
- **Fix**: Add `chrono` feature to sqlx in `Cargo.toml`. Add explicit match arms for `"timestamp without time zone"`, `"timestamp with time zone"`, `"date"`, `"time without time zone"`, `"time with time zone"`, `"uuid"` in both `pg_value_to_json` (`postgres.rs`) and `pg_value_to_csv_string` (`api/mod.rs`).
- **Citations**: `Cargo.toml:17`, `src/db/postgres.rs:400-404`, `src/db/postgres.rs:408-426`, `src/api/mod.rs:390-396`

### 2. Toolbar displays raw table name instead of display name (SHOULD-FIX)
- **File**: `frontend/src/App.svelte:146`
- **Related**: `frontend/src/components/Toolbar.svelte:16`
- **Type**: bug
- **Severity**: should-fix
- **Verification**: verified — `selectedTable` stores raw DB identifier; TableList correctly uses `display_name`; Toolbar does not
- **Fix**: Change `App.svelte:146` to: `tableName={displayConfig?.tables[selectedTable]?.display_name ?? selectedTable}`
- **Citations**: `App.svelte:62-64`, `App.svelte:146`, `Toolbar.svelte:16`, `TableList.svelte:21`

## Tracked (post-merge, not blocking)

### 3. Duplicate localStorage key constant
- **File**: `frontend/src/App.svelte:16`, `frontend/src/components/Sidebar.svelte:5`
- **Severity**: should-fix
- **Fix**: Extract to shared `lib/constants.ts`

### 4. Stale comment referencing getInitialCollapsed()
- **File**: `frontend/src/components/Sidebar.svelte:21`
- **Severity**: nit
- **Fix**: Update comment to match actual inline initialization at `App.svelte:17-19`

### 5. Zero frontend test coverage
- **File**: `frontend/`
- **Severity**: tracked obligation
- **Fix**: Set up Vitest with at least `assertShape` unit tests before Epic 3

### 6. Justfile dev recipe doesn't detect backend failure
- **File**: `Justfile:9`
- **Severity**: nit
- **Fix**: Add health check or wait after backgrounding `cargo run`

### 7. Mock total_rows inconsistent with row_count_estimate
- **File**: `frontend/src/lib/mock.ts:437` vs `mock.ts:8-18`
- **Severity**: nit
- **Fix**: Align mock generation count with declared estimates

### 8. DataGrid column width hardcoded at 150px
- **File**: `frontend/src/components/DataGrid.svelte:16`
- **Severity**: nit
- **Fix**: Use data-type-aware column sizing in DataGrid epic

### 9. No fetch timeout in api.ts
- **File**: `frontend/src/lib/api.ts:31`
- **Severity**: nit
- **Fix**: Add AbortController with reasonable timeout

## Test Command
```bash
cd frontend && npm run build && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
