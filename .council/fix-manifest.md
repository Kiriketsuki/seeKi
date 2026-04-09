# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration (Round 3)

Council verdict: **FOR** | 2026-04-09 | 6 findings (6 verified) | 1v1 no-questioner | 3 rounds total

## Pre-merge fixes (all applied)

### 1. selectTable race condition — FIXED
- **File**: `frontend/src/App.svelte:50-67`
- **Fix applied**: Monotonic `selectRequestId` counter guards all state writes in `selectTable` and `goToPage`.
- **Commit**: 5933501

### 2. CSV error row corrupts output — FIXED
- **File**: `src/api/mod.rs:322-328`
- **Fix applied**: Error comment row removed. `tracing::warn` logs the event. Truncated output is safer than corrupt output.
- **Commit**: 5933501

### 3. csv::Writer per-row allocation — FIXED
- **File**: `src/api/mod.rs:283-313`
- **Fix applied**: Single `csv::Writer` hoisted outside loop, flush at batch boundaries.
- **Commit**: 5933501

## Follow-ups (post-merge)

### 4. CSV export silent truncation on mid-stream DB error (should-fix)
- **File**: `src/api/mod.rs:328-330`
- **Issue**: HTTP 200 committed before spawn; mid-stream DB error produces silently truncated file with no client-visible signal.
- **Fix**: Send `Err(...)` to `tx` on `stream_error` instead of silently dropping the sender.

### 5. goToPage optimistic currentPage not rolled back on error (should-fix)
- **File**: `frontend/src/App.svelte:78`
- **Issue**: `currentPage = page` set before fetch; on failure, StatusBar shows wrong page while grid shows stale data.
- **Fix**: Move `currentPage = page` to after `queryResult = result` inside the try block.

### 6. Content-Disposition filename unsanitized (nit)
- **File**: `src/api/mod.rs:341`
- **Issue**: Display name with `"` or CRLF produces malformed header. Operator-controlled, not external attack surface.
- **Fix**: Strip `"`, `;`, `\r`, `\n` from filename before header interpolation.

## Test Command
```bash
cd frontend && npm run build && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
