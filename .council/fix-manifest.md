# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration

Council verdict: CONDITIONAL | 2026-04-08 | 10 findings (9 verified, 1 dismissed) | TeamCreate council

## Fixes Required (merge conditions)

### 1. Mock total_rows misrepresents actual data
- **File**: `frontend/src/lib/mock.ts`
- **Lines**: 435, 437, 467-473
- **Type**: bug
- **Severity**: medium
- **Verification**: verified by Questioner — page 2 returns [] while StatusBar shows "51–100 of 427,229"
- **Fix**: Set `total_rows` in the returned QueryResult to the actual length of the filtered/generated rows array, not `row_count_estimate`
- **Citations**: `mock.ts:435` (totalRows = row_count_estimate), `mock.ts:437` (getRows hardcoded to 50), `mock.ts:464-465` (slice returns [] for page 2)

### 2. Justfile build recipe doesn't pin VITE_MOCK=false
- **File**: `Justfile`
- **Line**: 20
- **Type**: hardening
- **Severity**: medium
- **Verification**: verified — risk downgraded (requires explicit `export VITE_MOCK=true` in shell)
- **Fix**: Change `cd frontend && npm run build` to `cd frontend && VITE_MOCK=false npm run build`
- **Citations**: `Justfile:20`, `api.ts:16` (strict equality gate)

## Follow-Up Issues (track separately, do not block merge)

### 3. No table navigation UI (HIGH priority)
- **File**: `frontend/src/App.svelte`
- **Line**: 73
- **Type**: bug (missing feature)
- **Severity**: high
- **Fix**: Add a table list component as Sidebar children; wire onclick to selectTable
- **Conceded by**: both advocates

### 4. Setup mode disconnected (HIGH priority)
- **File**: `frontend/src/App.svelte`
- **Line**: 16
- **Type**: bug (missing feature)
- **Severity**: high
- **Fix**: Wire isSetup to backend API detection; render setup wizard when backend is in setup mode
- **Conceded by**: both advocates

### 5. CORS allows Any methods/headers
- **File**: `src/main.rs`
- **Line**: 58
- **Type**: hardening
- **Severity**: low
- **Fix**: Restrict to `allow_methods([GET])` and `allow_headers([CONTENT_TYPE])`

### 6. content.data.to_vec() clones embedded bytes
- **File**: `src/embed.rs`
- **Lines**: 26, 41
- **Type**: performance
- **Severity**: low
- **Fix**: Use `Cow`/`Bytes` conversion instead of `.to_vec()` allocation

### 7. API error handling discards response body
- **File**: `frontend/src/lib/api.ts`
- **Line**: 21
- **Type**: improvement
- **Severity**: low
- **Fix**: Read `res.text()` or `res.json()` for error body instead of relying on `statusText`

### 8. No runtime validation of API response shapes
- **File**: `frontend/src/lib/api.ts`
- **Line**: 23
- **Type**: improvement
- **Severity**: low
- **Fix**: Add field-presence checks or zod schema validation at the API boundary

### 9. Setup mode hardcodes bind address
- **File**: `src/main.rs`
- **Line**: 88
- **Type**: improvement
- **Severity**: low
- **Fix**: Read from env var or partial config file

## Dismissed Findings

### ~~CDN font dependency~~ (DISMISSED)
- **Reason**: Factually wrong — fonts are npm-bundled via `@fontsource-variable` imports in `main.ts:1-2`
- **Conceded by**: critic-2

## Test Command
```bash
cd frontend && npm run build && cargo check
```

## Raw Data
council-result.json: .council/council-result.json
