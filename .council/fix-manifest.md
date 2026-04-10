# Fix Manifest — PR #29: epic: Data Grid & Table Navigation (Session 7)

Council verdict: **FOR** | 2026-04-10 | 16 findings (16 verified, 16 FIXED) | 1v1 no-questioner | 2 rounds

Prior sessions: Session 1 (3 fixed, 3 dismissed), Session 2 (2 fixed, 1 tracked, 1 dismissed), Session 3 (1 fixed, 1 tracked), Session 4 (1 fixed, 1 tracked), Session 5 (2 fixed, 1 tracked), Session 6 (1 fixed, 2 tracked), Session 7 (1 new finding + 6 previously tracked — all 7 fixed).

## All Findings Resolved

All 16 findings across 7 sessions have been fixed. No tracked or open items remain.

## Session 7 Fixes

### 1. bigint > 2^53 precision loss in JSON transport
- **File**: `src/db/postgres.rs:405-408`
- **Fix**: Serialize bigint as string when `unsigned_abs() > 2^53`; values within safe JS range remain JSON numbers.

### 2. Dead currentPage arg in exportCsv
- **File**: `frontend/src/App.svelte:209`
- **Fix**: Changed `buildRowsParams(currentPage)` to `buildRowsParams(1)` — page is unused for CSV export.

### 3. Missing aria-sort on sorted column headers (WCAG 1.3.1)
- **File**: `frontend/src/components/DataGrid.svelte:66`
- **Fix**: Added `aria-sort="ascending"`/`"descending"` to header wrapper div via conditional spread.

### 4. real (float4) grid vs CSV serialization inconsistency
- **File**: `src/db/postgres.rs:409-412`
- **Fix**: Parse f32 via `to_string().parse::<f64>()` to avoid widening artifacts (e.g., 3.14 not 3.140000104904175).

### 5. Dead 'decimal' entry in NUMERIC_TEXT_TYPES
- **File**: `frontend/src/lib/data-grid.ts:20`
- **Fix**: Removed — PostgreSQL normalizes DECIMAL to 'numeric'.

### 6. Content-Disposition missing backslash escape
- **File**: `src/api/mod.rs:233`
- **Fix**: Added `.replace('\\', "")` to filename sanitizer chain.

### 7. Search ILIKE path skips identifier validation on schema-derived column names
- **File**: `src/db/postgres.rs:211`
- **Fix**: Added `&& is_valid_identifier(&c.name)` to the `text_cols` filter chain.

## Prior Session Fixes (sessions 1-6)

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

### Session 6
1. **ToolStrip export aria-label misinforms users** — `ToolStrip.svelte:83-85`: changed to "More export options coming soon"

## Dismissed (sessions 1-6)
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
