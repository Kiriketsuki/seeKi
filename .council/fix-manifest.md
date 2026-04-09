# Fix Manifest -- PR #28: epic: Frontend Scaffold & Integration

Council verdict: CONDITIONAL | 2026-04-08 | 5 findings (5 verified) | 1v1 no-questioner

## Fixes Required (pre-merge)

### 1. selectTable race condition
- **File**: `frontend/src/App.svelte`
- **Line**: 50
- **Type**: bug
- **Severity**: should-fix
- **Verification**: verified, conceded by advocate
- **Fix**: Add a monotonic request counter (e.g. `let requestId = 0`). At entry, increment and capture `const myId = ++requestId`. After the await, guard: `if (myId !== requestId) return`. This ensures only the latest call writes state.
- **Citations**: `frontend/src/App.svelte:50-67`

### 2. CSV error row corrupts output
- **File**: `src/api/mod.rs`
- **Line**: 322
- **Type**: bug
- **Severity**: should-fix
- **Verification**: verified, conceded by advocate
- **Fix**: Remove lines 322-328 (the `# ERROR:` comment). The `tracing::warn` at line 321 already logs the event. Truncated output is safer than corrupt output for downstream parsers.
- **Citations**: `src/api/mod.rs:320-328`

### 3. csv::Writer per-row allocation in export loop
- **File**: `src/api/mod.rs`
- **Line**: 291
- **Type**: improvement
- **Severity**: should-fix
- **Verification**: verified, conceded by advocate
- **Fix**: Create one `csv::Writer` before the loop. Call `flush()` at batch boundaries (every 100 rows), take the underlying buffer, send the chunk.
- **Citations**: `src/api/mod.rs:283-313`

## Follow-ups (post-merge OK)

### 4. goToPage optimistic page update (nit)
- **File**: `frontend/src/App.svelte`
- **Line**: 73
- **Fix**: Move `currentPage = page` to after `queryResult = result` inside the try block.

### 5. Content-Disposition filename unsanitized (nit)
- **File**: `src/api/mod.rs`
- **Line**: 338
- **Fix**: Add `.replace('"', "").replace(';', "")` to filename before header injection.

## Test Command
```bash
cd frontend && npm run build && cd .. && cargo test
```

## Raw Data
council-result.json: .council/council-result.json
