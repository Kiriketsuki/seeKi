# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration (Round 2)

Council verdict: **FOR** | 2026-04-08 | 8 findings (8 verified) | 1v1 no-questioner

## Blockers (all fixed)

### 1. selectTable error destroys working UI — FIXED
- **File**: `frontend/src/App.svelte:40-52`
- **Fix applied**: Scoped to `tableError` variable; global `error` only set by `onMount`. Inline dismissible banner for table-level failures.
- **Commit**: 8a75543

### 2. Error state permanent after transient failure — FIXED
- **File**: `frontend/src/App.svelte:40-52`
- **Fix applied**: `tableError` cleared at start of `selectTable`. Global `error` never set by `selectTable`.
- **Commit**: 8a75543

## Reclassified (blocker → follow-up)

### 3. Pagination permanently disabled (HIGH follow-up)
- **File**: `frontend/src/components/StatusBar.svelte:25,29`
- **Arbiter ruling**: Scaffold PR scope — all interactive buttons are intentionally disabled placeholders. Backend pagination fully implemented. Wiring is Epic 3.

### 4. Export button wired to nothing
- **File**: `frontend/src/components/Toolbar.svelte:35`
- **Same scaffold pattern as pagination**

## Other Follow-ups

### 5. Duplicate table-list nav markup — `App.svelte:66-81,101-116`
### 6. Mock generates only 50 rows — `mock.ts:437`
### 7. No loading state indicator — `App.svelte`
### 8. Sidebar localStorage flash — `Sidebar.svelte:21-29`

## Test Command
```bash
cd frontend && npm run build && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
