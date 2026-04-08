# Fix Manifest -- PR #27: epic: Backend API & Config

Council verdict: CONDITIONAL | 2026-04-08 | 2 findings (2 verified)

Previous council (pre-hardening) had 6 findings -- all 6 were fixed in commit 6e26117.
This council reviews the post-hardening state.

## Fixes Required

### 1. save_config must verify connection before writing
- **File**: `src/api/setup.rs`
- **Lines**: 107-172
- **Type**: bug
- **Severity**: medium
- **Verification**: verified by Critic rebuttal, Questioner Q1+Q11
- **Fix**: Call `postgres::test_connection(&req.database.url)` before writing seeki.toml. If the connection fails, return `{ success: false, error: "..." }` without writing the file. This reuses the existing `test_connection` function already in `src/db/postgres.rs:11-28`.
- **Citations**: `src/api/setup.rs:107-172`, `src/main.rs:93`, `src/db/postgres.rs:11-28`

### 2. row_count_estimate exposes -1 for unanalyzed tables
- **File**: `src/db/postgres.rs`, `src/db/mod.rs`
- **Lines**: postgres.rs:50-53, mod.rs:24
- **Type**: bug
- **Severity**: medium
- **Verification**: verified by Questioner Q9, Critic rebuttal
- **Fix**: Change `TableInfo.row_count_estimate` from `i64` to `Option<i64>`. In `list_tables`, clamp negative values to `None`. Update `api/mod.rs` serialization to emit `null` instead of `-1`.
- **Citations**: `src/db/postgres.rs:50-53`, `src/db/mod.rs:24`, `src/api/mod.rs:107-109`

## Informational Notes (not blocking)
- SSRF via test_connection URL (mitigated by localhost bind)
- Divergent pg_value_to_json / pg_value_to_csv_string implementations
- ILIKE full-table scan at scale (acceptable for v0.1)
- display_config queries DB on every request (cache for v0.2)
- get_columns PK subquery missing table_schema filter
- CSV error sentinel not RFC 4180 compliant (acceptable for v0.1)

## Test Command
```
cargo test
```
