# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration (Round 4 — Final)

Council verdict: **FOR** | 2026-04-09 | 8 findings (8 verified, 1 dismissed) | 1v1 no-questioner | 4 rounds total

## All findings fixed

### 1. selectTable race condition — FIXED (commit 5933501)
- **File**: `frontend/src/App.svelte:50-67`
- **Fix**: Monotonic `selectRequestId` counter guards all state writes.

### 2. CSV error row corrupts output — FIXED (commit 5933501)
- **File**: `src/api/mod.rs:322-328`
- **Fix**: Error comment row removed. `tracing::warn` logs the event.

### 3. csv::Writer per-row allocation — FIXED (commit 5933501)
- **File**: `src/api/mod.rs:283-313`
- **Fix**: Single `csv::Writer` hoisted outside loop, flush at batch boundaries.

### 4. CSV export silent truncation — FIXED (commit 7f92f80)
- **File**: `src/api/mod.rs:333-341`
- **Fix**: `Err(UnexpectedEof)` sent to `tx` on `stream_error`.

### 5. goToPage optimistic currentPage — FIXED (commit 7f92f80)
- **File**: `frontend/src/App.svelte:82`
- **Fix**: `currentPage = page` moved to after successful fetch.

### 6. Content-Disposition filename unsanitized — FIXED (commit 7f92f80)
- **File**: `src/api/mod.rs:232-236`
- **Fix**: Strip `"`, `;`, `\r`, `\n` from filename.

### 7. goToPage/selectTable cross-type race — FIXED (round 4)
- **File**: `frontend/src/App.svelte:161`, `frontend/src/components/StatusBar.svelte`
- **Issue**: Pagination buttons remained clickable during table switch (StatusBar outside loading overlay). goToPage could silently cancel in-flight selectTable.
- **Fix**: Pass `loading={tableLoading}` to StatusBar; pagination buttons disabled when `loading=true`.

### 8. "1 of 0" pagination on empty tables — FIXED (round 4)
- **File**: `frontend/src/App.svelte:166`
- **Issue**: `Math.ceil(0/N) = 0` but `page=1`, showing "1 of 0".
- **Fix**: `Math.max(1, Math.ceil(...))` ensures totalPages >= 1.

## Dismissed

### into_inner().unwrap_or_default() silent batch drop
- **File**: `src/api/mod.rs:307`
- **Reason**: Unsubstantiated. `csv::Writer` wraps `Vec<u8>`; `flush()` on Vec is infallible; `into_inner()` after successful flush cannot fail. Dead-code defense only.

## Follow-ups (not blocking)
- Frontend test infrastructure (zero frontend tests — scaffold establishes patterns, tests are a natural follow-up)

## Test Command
```bash
cd frontend && npm run build && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
