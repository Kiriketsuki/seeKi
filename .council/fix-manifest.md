# Fix Manifest — PR #29: epic: Data Grid & Table Navigation (Session 2)

Council verdict: **FOR** | 2026-04-09 | 4 findings (4 verified, 1 dismissed) | 1v1 no-questioner | 2 rounds

Prior session: Session 1 (3 findings, all fixed, 3 dismissed).

## No Fixes Required (pre-merge)

All pre-merge issues from sessions 1+2 have been resolved.

### Session 1 Fixes (applied in prior commit)

1. **Date off-by-one for non-UTC users** — `data-grid.ts`: separate DATE_ONLY_TYPES, append T00:00:00, use dateFormatter
2. **Timestamp formatter omits year** — `data-grid.ts`: added year: 'numeric' to Intl.DateTimeFormat
3. **ToolStrip sort indicator missing ARIA role** — `ToolStrip.svelte`: added role="status"

### Session 2 Fixes (applied in this commit)

#### 1. Safari Invalid Date for `timestamp without time zone`
- **File**: `frontend/src/lib/data-grid.ts:137`
- **Severity**: high
- **Fix**: Replace space with T before `new Date()` — `new Date(raw.replace(' ', 'T'))`
- **Conceded by**: ADVOCATE

#### 2. CSV export ignores active filters/sort
- **File**: `frontend/src/App.svelte:207-219`
- **Severity**: medium
- **Fix**: Pass `buildRowsParams()` as query params to the export URL
- **Conceded by**: ADVOCATE

### Dismissed: sortStateToConfig returns undefined
- **Claim**: RevoGrid sort indicators not cleared when `sorting` is `undefined`
- **Reason**: ADVOCATE rebutted — sort indicators driven by per-column `order` prop (`DataGrid.svelte:198-199`), not `sorting` object prop

## Tracked (post-merge, not blocking)

### 1. is_valid_identifier rejects hyphenated identifiers
- **File**: `src/db/postgres.rs:169-171`
- **Severity**: medium
- **Fix**: Add hyphen to the identifier allowlist (safe inside double-quotes)

## Test Command
```bash
cd frontend && npm run check && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
