# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration (Round 2)

Council verdict: CONDITIONAL | 2026-04-08 | 7 findings (7 verified) | 1v1 no-questioner

## Fixes Required (merge conditions)

### 1. selectTable error destroys working UI (BLOCKER)
- **File**: `frontend/src/App.svelte`
- **Line**: 40-52
- **Type**: bug
- **Severity**: high (blocker)
- **Verification**: verified
- **Fix**: Scope selectTable errors to a table-specific error variable (e.g., `tableError`) instead of the global `error` that flips the entire UI to error state. Preserve previously-loaded table data on failure. Only set global `error` for unrecoverable failures (onMount).
- **Citations**: `App.svelte:42` (sets selectedTable before fetch), `App.svelte:49-50` (sets global error on table load failure), `App.svelte:58` (error branch hides all data)

## Follow-Up Issues (do not block merge)

### 2. Empty table list in error state sidebar
- **File**: `frontend/src/App.svelte:66-81`
- **Severity**: low

### 3. Duplicate table-list nav markup
- **File**: `frontend/src/App.svelte:66-81,101-116`
- **Severity**: low

### 4. Mock generates only 50 rows
- **File**: `frontend/src/lib/mock.ts:437`
- **Severity**: low

### 5. fetchStatus serial in critical path
- **File**: `frontend/src/App.svelte:21`
- **Severity**: low

### 6. No loading state indicator
- **File**: `frontend/src/App.svelte`
- **Severity**: medium

### 7. Sidebar localStorage flash
- **File**: `frontend/src/components/Sidebar.svelte:21-29`
- **Severity**: low

## Test Command
```bash
cd frontend && npm run build && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
