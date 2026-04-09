# Fix Manifest — PR #29: epic: Data Grid & Table Navigation (Session 6)

Council verdict: **FOR** | 2026-04-09 | 15 findings (15 verified) | 1v1 no-questioner | 2 rounds

Prior sessions: Session 1 (3 fixed, 3 dismissed), Session 2 (2 fixed, 1 tracked, 1 dismissed), Session 3 (1 fixed, 1 tracked), Session 4 (1 fixed, 1 tracked), Session 5 (2 fixed, 1 tracked).

## Session 6 Fixes (applied in this commit)

### 1. ToolStrip export aria-label misinforms users
- **File**: `frontend/src/components/ToolStrip.svelte:83-85`
- **Severity**: medium
- **Fix**: Changed `aria-label="Export tools coming later"` to `"More export options coming soon"` — disambiguates from the existing working CSV export in the Toolbar.
- **Conceded by**: ADVOCATE (full concession — factually incorrect accessible text)

## Prior Session Fixes

### Session 1
1. **Date off-by-one for non-UTC users** — `data-grid.ts`: separate DATE_ONLY_TYPES, append T00:00:00
2. **Timestamp formatter omits year** — `data-grid.ts`: added year: 'numeric' to Intl.DateTimeFormat
3. **ToolStrip sort indicator missing ARIA role** — `ToolStrip.svelte`: added role="status"

### Session 2
1. **Safari Invalid Date for `timestamp without time zone`** — `data-grid.ts:137`: replace space with T
2. **CSV export ignores active filters/sort** — `App.svelte:207-219`: pass sort/filter params to export URL

### Session 3
1. **numeric/decimal precision loss via Number() cast** — `data-grid.ts:14-15`: removed numeric/decimal from NUMBER_TYPES

### Session 4
1. **real (float4) columns silently null out via OID mismatch** — `postgres.rs:386-389`: split real/double precision arms

### Session 5
1. **Boolean filter broken: Yes/No display vs true/false SQL cast** — `postgres.rs:221-248`: use = TRUE/FALSE instead of ::text ILIKE
2. **numeric/decimal columns display as left-aligned plain text** — `data-grid.ts:16-21, 152-157`: added NUMERIC_TEXT_TYPES set

## Tracked (post-merge, not blocking)

### 1. bigint values > 2^53 lose precision in JSON transport
- **File**: `src/db/postgres.rs:382-386`
- **Severity**: low (2^53 threshold rarely hit in practice)
- **Fix**: Serialize bigint as string for values > Number.MAX_SAFE_INTEGER

### 2. Dead currentPage arg in exportCsv
- **File**: `frontend/src/App.svelte:209`
- **Severity**: nit
- **Fix**: Remove unused page param from `buildRowsParams` call in `exportCsv`

### 3. Missing aria-sort on sorted column headers (WCAG 1.3.1)
- **File**: `frontend/src/components/DataGrid.svelte:66-100`
- **Severity**: medium
- **Fix**: Add `aria-sort="ascending"` / `"descending"` to header div in renderHeader

### 4. real (float4) grid vs CSV serialization inconsistency
- **File**: `src/db/postgres.rs:388`, `src/api/mod.rs:380`
- **Severity**: low
- **Fix**: Use `f32.to_string()` then parse for JSON instead of `f32 as f64` widening

### 5. Dead 'decimal' entry in NUMERIC_TEXT_TYPES
- **File**: `frontend/src/lib/data-grid.ts:20`
- **Severity**: nit
- **Fix**: Remove the entry — PostgreSQL normalizes DECIMAL to 'numeric'

### 6. Content-Disposition missing backslash escape
- **File**: `src/api/mod.rs:235`
- **Severity**: nit
- **Fix**: Add `.replace('\\', "")` to filename sanitizer chain

## Dismissed (sessions 1-4)
- `on:beforesorting` Svelte 5 syntax — correct for Svelte 4 dispatcher consumed by Svelte 5
- Boolean coercion misses 1/yes/on — backend sends JSON booleans only
- ILIKE performance on 500K rows — pre-existing design choice
- sortStateToConfig returns undefined — sort indicators driven by per-column `order` prop

## Test Command
```bash
cd frontend && npm run check && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
