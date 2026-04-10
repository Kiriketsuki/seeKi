# Fix Manifest — PR #29: epic: Data Grid & Table Navigation (Session 8)

Council verdict: **FOR** | 2026-04-10 | Session 8: 4 findings (1 dismissed, 3 tracked, 1 noted) | 1v1 no-questioner | 2 rounds

Prior sessions: Sessions 1-7 produced 16 findings, all FIXED. Session 8 is a fresh review of the final code state.

## No Pre-Merge Fixes Required

Session 8 found no pre-merge blockers. All findings are post-merge follow-ups.

## Session 8 Findings

### Dismissed: `get_columns_bulk` PK subquery missing `table_schema`
- **File**: `src/db/postgres.rs:134-140`
- **Reason**: ARBITER verified both `get_columns` and `get_columns_bulk` exist on `main` since PR #27 (Epic 1) with identical missing `table_schema` pattern. Pre-existing, not introduced by this PR. CRITIC withdrew.

### Tracked 1: `formatCellValue` has zero unit tests (medium)
- **File**: `frontend/src/lib/data-grid.ts:100-174`
- **Type**: improvement
- **Fix**: Add `data-grid.test.ts` covering null, boolean, date (T00:00:00 suffix), datetime (space→T), numeric passthrough, NaN/Infinity guard.

### Tracked 2: Content-Disposition non-ASCII filename (low)
- **File**: `src/api/mod.rs:229-238`
- **Type**: bug (cosmetic)
- **Fix**: Add RFC 6266 `filename*=UTF-8''<percent-encoded>` for non-ASCII display names, or strip non-ASCII in sanitizer.

### Tracked 3: Missing `type="button"` (nit)
- **Files**: `frontend/src/components/TableList.svelte:43`, `frontend/src/components/StatusBar.svelte:33,41`
- **Fix**: Add `type="button"` to all three `<button>` elements.

### Noted: ToolStrip `role="status"` live region noisiness
- **File**: `frontend/src/components/ToolStrip.svelte:57`
- **Type**: accessibility refinement
- **Fix**: Consider replacing `role="status"` with `aria-label` on a non-live container to avoid announcing every sort change.

## Cumulative History (sessions 1-7)

All 16 findings from sessions 1-7 are FIXED:
- Session 1: Date off-by-one, timestamp year omission, ToolStrip ARIA role
- Session 2: Safari Invalid Date, CSV export ignores filters/sort
- Session 3: numeric/decimal precision loss via Number() cast
- Session 4: real (float4) OID mismatch, missing aria-sort on headers
- Session 5: Boolean filter mapping, numeric display classification
- Session 6: ToolStrip export aria-label misinformation, dead 'decimal' entry, Content-Disposition backslash
- Session 7: Search ILIKE identifier validation, bigint > 2^53 serialization, dead currentPage arg, + 4 tracked fixes

## Dismissed Across All Sessions
- `on:beforesorting` Svelte 5 syntax — correct for Svelte 4 dispatcher
- Boolean coercion misses 1/yes/on — backend sends JSON booleans only
- ILIKE performance on 500K rows — pre-existing design choice
- sortStateToConfig returns undefined — sort indicators driven by per-column `order` prop
- get_columns_bulk schema filter — pre-existing on main since Epic 1

## Test Command
```bash
cd frontend && npm run check && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
