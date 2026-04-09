# Fix Manifest — PR #29: epic: Data Grid & Table Navigation

Council verdict: **CONDITIONAL** | 2026-04-09 | 3 findings (3 verified)

## Fixes Required

### 1. Date off-by-one for non-UTC users
- **File**: `frontend/src/lib/data-grid.ts`
- **Line**: 111
- **Type**: bug
- **Severity**: high
- **Verification**: verified
- **Fix**: Detect date-only columns (`column.data_type === 'date'`) and either:
  (a) Append `'T00:00:00'` to force local-timezone parsing, or
  (b) Use a separate date-only `Intl.DateTimeFormat` (no hour/minute) for `date` columns.
  Date columns should NOT show time components in the display.
- **Citations**: `data-grid.ts:111`, `data-grid.ts:18-23`, `postgres.rs:408-411`

### 2. Timestamp formatter omits year
- **File**: `frontend/src/lib/data-grid.ts`
- **Line**: 25
- **Type**: bug
- **Severity**: medium
- **Verification**: verified
- **Fix**: Add `year: 'numeric'` to the `Intl.DateTimeFormat` options at lines 25-30.
- **Citations**: `data-grid.ts:25-30`

### 3. ToolStrip sort indicator missing ARIA role
- **File**: `frontend/src/components/ToolStrip.svelte`
- **Line**: 57
- **Type**: improvement
- **Severity**: medium
- **Verification**: verified
- **Fix**: Add `role="status"` to the sort indicator `<div>` element.
- **Citations**: `ToolStrip.svelte:57`

## Conditions
- All 3 fixes must be applied before merge

## Follow-ups (not blocking)
- Unit tests for `data-grid.ts` pure functions
- Tighten `sortStateToConfig` return type
- Boolean coercion for future SQLite
- Investigate `aria-sort` on RevoGrid column headers

## Test Command
```bash
cd frontend && npm run test && npm run check
```

## Raw Data
council-result.json: .council/council-result.json
