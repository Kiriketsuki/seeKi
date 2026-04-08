# Fix Manifest — PR #27: epic: Backend API & Config

Council verdict: CONDITIONAL | 2026-04-08 | 6 findings (6 verified)

## Fixes Required

### 1. smallint/integer type decode bug — silent data loss
- **File**: `src/db/postgres.rs`
- **Lines**: 305-312 (pg_value_to_json), also `src/api/mod.rs:319-327` (pg_value_to_csv_string)
- **Type**: bug
- **Severity**: critical
- **Verification**: verified by Critic 2
- **Fix**: Use `try_get::<i32>` for integer, `try_get::<i16>` for smallint, `try_get::<i64>` for bigint. Also fix `numeric` → use `sqlx::types::BigDecimal` or `String` instead of `f64`.
- **Citations**: `src/db/postgres.rs:306-310`, `src/api/mod.rs:320-322`

### 2. Unbounded page_size — DoS vector
- **File**: `src/api/mod.rs`
- **Line**: 139-151
- **Type**: hardening
- **Severity**: high
- **Verification**: verified by Critic 1, Critic 2, Questioner
- **Fix**: Clamp `page_size` to a maximum (e.g. 1000) after deserialization. Also guard `page_size=0`.
- **Citations**: `src/api/mod.rs:139`, `src/db/postgres.rs:221`

### 3. TOML injection in save_config
- **File**: `src/api/setup.rs`
- **Lines**: 113-128
- **Type**: security
- **Severity**: high
- **Verification**: verified by Critic 1, Critic 2, Questioner
- **Fix**: Build a typed struct and use `toml::to_string_pretty` instead of `format!` interpolation.
- **Citations**: `src/api/setup.rs:113-128`

### 4. Error messages leak DB internals
- **File**: `src/api/mod.rs`
- **Lines**: 364-375
- **Type**: security
- **Severity**: high
- **Verification**: verified by Critic 1
- **Fix**: Return generic "Internal server error" for 500s. Log the real error via `tracing::error!`. Also sanitize `test_connection` errors in `src/api/setup.rs:49-53`.
- **Citations**: `src/api/mod.rs:371-374`, `src/api/setup.rs:49-53`

### 5. sort_column not schema-validated
- **File**: `src/db/postgres.rs`
- **Lines**: 171-180
- **Type**: bug
- **Severity**: medium
- **Verification**: verified by Critic 1, Critic 2, Questioner
- **Fix**: Validate `sort_column` against `valid_column_names` set (same pattern as filter validation at lines 122-133).
- **Citations**: `src/db/postgres.rs:171-180`

### 6. Config parse error silently enters setup mode
- **File**: `src/main.rs`
- **Lines**: 26-29
- **Type**: bug
- **Severity**: medium
- **Verification**: verified by Critic 2
- **Fix**: Distinguish file-not-found from parse error. Only enter setup mode when no config file exists. Print the parse error and exit on malformed config.
- **Citations**: `src/main.rs:26-29`, `src/config.rs:172-196`

## Test Command
```
cargo test && cargo clippy
```

## Raw Data
council-result: .council/fix-manifest.md
