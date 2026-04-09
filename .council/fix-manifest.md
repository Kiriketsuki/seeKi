# Fix Manifest — PR #29: epic: Data Grid & Table Navigation (Session 3)

Council verdict: **CONDITIONAL→FOR** | 2026-04-09 | 7 findings (7 verified) | 1v1 no-questioner | 2 rounds

Prior sessions: Session 1 (3 findings, all fixed, 3 dismissed), Session 2 (2 fixed, 1 tracked, 1 dismissed).

## Session 3 Fix (applied in this commit)

### 1. numeric/decimal precision loss via Number() cast
- **File**: `frontend/src/lib/data-grid.ts:14-15`
- **Severity**: high
- **Fix**: Removed `'numeric'` and `'decimal'` from `NUMBER_TYPES`. Backend already sends full-precision string (`postgres.rs:391`); values now fall through to the text formatter at `data-grid.ts:158-161`.
- **Conceded by**: ADVOCATE (partially — agreed bug is real, contested blocking status)

## Prior Session Fixes

### Session 1
1. **Date off-by-one for non-UTC users** — `data-grid.ts`: separate DATE_ONLY_TYPES, append T00:00:00
2. **Timestamp formatter omits year** — `data-grid.ts`: added year: 'numeric' to Intl.DateTimeFormat
3. **ToolStrip sort indicator missing ARIA role** — `ToolStrip.svelte`: added role="status"

### Session 2
1. **Safari Invalid Date for `timestamp without time zone`** — `data-grid.ts:137`: replace space with T
2. **CSV export ignores active filters/sort** — `App.svelte:207-219`: pass sort/filter params to export URL

## Tracked (post-merge, not blocking)

### 1. bigint values > 2^53 lose precision in JSON transport
- **File**: `src/db/postgres.rs:382-386`
- **Severity**: low (2^53 threshold rarely hit in practice)
- **Fix**: Serialize bigint as string for values > Number.MAX_SAFE_INTEGER

### 2. Dead currentPage arg in exportCsv
- **File**: `frontend/src/App.svelte:209`
- **Severity**: nit
- **Fix**: Remove unused page param from `buildRowsParams` call in `exportCsv`

## Dismissed (sessions 1-2)
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
