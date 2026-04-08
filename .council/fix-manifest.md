# Fix Manifest -- epic/1-backend-api-config merge into main

Council verdict: CONDITIONAL | 2026-04-08 | 2 findings (2 verified)

Third adversarial council (pre-merge review). Prior councils found 8 issues total -- all fixed.
This council reviews the fully-hardened state before merging into main.

## Fixes Required

### 1. u32 overflow in OFFSET computation
- **File**: `src/db/postgres.rs`
- **Line**: 287
- **Type**: bug
- **Severity**: high
- **Verification**: verified by Critic 2, conceded by Advocate
- **Fix**: Cast `page` and `page_size` to `u64` before multiplication to prevent silent wrapping in release mode. e.g. `let offset = (params.page.saturating_sub(1) as u64) * (params.page_size as u64);` and interpolate as u64 into SQL.
- **Citations**: `src/db/postgres.rs:287`, `src/db/mod.rs:48-49`

### 2. CSV error sentinel is not valid CSV
- **File**: `src/api/mod.rs`
- **Lines**: 271-276, 322-327
- **Type**: bug
- **Severity**: medium
- **Verification**: verified by Critic 1 + Critic 2, conceded by Advocate
- **Fix**: Either (a) remove the raw error text sentinels entirely and just close the stream (log the error server-side, which already happens), or (b) emit the error as a properly quoted CSV row. Option (a) is simpler and safer -- a truncated CSV is less harmful than a corrupted one.
- **Citations**: `src/api/mod.rs:271-276`, `src/api/mod.rs:322-327`

## Informational Notes (not blocking)
- CORS prefix match could be tightened to exact match (main.rs:47) -- low severity for localhost tool
- No integration tests for DB layer (pre-existing gap, not introduced by this branch)
- save_config accepts non-localhost hosts (low risk, setup mode is one-time)
- Identifier validation rejects `$` in table/column names (compatibility edge case)
- display_config / get_columns not cached (acceptable for v0.1, cache for v0.2)
- Divergent pg_value_to_json / pg_value_to_csv_string boolean rendering (intentional: true/false vs Yes/No)

## Test Command
```
cargo test
```
