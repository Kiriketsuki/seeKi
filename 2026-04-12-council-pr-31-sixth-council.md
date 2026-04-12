## Adversarial Council — PR #31 First-Run Setup Wizard (Sixth Council)

> Convened: 2026-04-12 | Advocates: 1 | Critics: 1 | Rounds: 3/4 | Motion type: CODE

### Motion

PR #31 (epic: First-Run Setup Wizard), after applying all five prior council fix sessions — including the fifth council's CONDITIONAL fix adding a 60-second `AbortController` timeout to `setupTestConnection` and `setupSaveConfig` in `frontend/src/lib/api.ts` — is correct, complete, and ready to merge into main.

### Advocate Positions

**ADVOCATE-1**: The fifth council's sole condition (AbortController timeout) is fully met. `SETUP_TIMEOUT_MS = 60_000` is correctly applied to both `setupTestConnection` (api.ts:139-156) and `setupSaveConfig` (api.ts:165-182) with proper timer cleanup and user-readable abort messages. All prior blocking findings remain resolved: mode guards return 409, atomic TOML write, secrets cleanup on failure, serde defaults, SSH password binding. The two new findings (C-New-1, C-New-2) are real but narrow — the triggering scenario requires SSH with both tunnel handshakes exceeding 30s — and do not meet the blocking bar of correctness/data integrity/security. Acknowledged that both fixes are trivial and deferred to ARBITER on whether they clear the CONDITIONAL threshold.

### Critic Positions

**CRITIC-1**: The fifth council's timeout condition is mechanically met, but the fix introduces a regression on slow-SSH save paths. `save_config` makes two sequential SSH operations (`postgres::test_connection` at setup.rs:262 and `DatabasePool::connect` at setup.rs:343); if combined time exceeds 60s, the frontend aborts while the server completes and swaps to Normal mode. On retry, the user hits the 409 mode guard and sees raw JSON (C-New-2) with no guided recovery. Pre-fix behavior on this path was success with a long wait. Both defects are introduced by this PR, both have trivial fixes (~10-15 lines total), and deferring to a follow-up costs more than fixing in-PR. SeeKi's non-technical target audience cannot navigate the stuck state.

### Questioner Findings

No questioner in this 1v1 council format.

### Key Conflicts

- **C-New-1 severity**: ADVOCATE argued the slow-SSH race is narrow (corporate bastions typically 2-5s handshake) and below the blocking bar. CRITIC argued the pre-fix behavior was success on the same path, making this a regression — not a trade-off — and that SSH deployments are the hard case that motivated SSH support in the first place. **Resolved**: ADVOCATE conceded the regression framing and deferred to ARBITER on the CONDITIONAL threshold.

- **C-New-2 provenance**: ADVOCATE initially characterized C-New-2 as "pre-existing tech debt." ARBITER challenged this: `setupTestConnection` and `setupSaveConfig` do not exist on `main` — they are new in PR #31. ADVOCATE withdrew the "pre-existing" defense. **Resolved**: Both sides agree C-New-2 is a defect introduced by this PR.

- **Fix cost vs. deferral**: ADVOCATE argued these are follow-up items. CRITIC argued that real defects with trivial fixes should be fixed before merge, not deferred to compete in the backlog. **Resolved**: ADVOCATE acknowledged they "cannot honestly claim otherwise" that fix cost argues for fixing before merge.

### Concessions

- **ADVOCATE-1** conceded to **CRITIC-1**: C-New-1 mechanics are real; the trade-off framing was incorrect (it is a regression, not a trade-off); C-New-2 is introduced by this PR (withdrew "pre-existing" defense); fixes are small and correct.
- **CRITIC-1** conceded to **ADVOCATE-1**: "No recovery path" was too strong — page refresh recovers. Corrected to "no guided recovery path."

### Regression Lineage

The fifth council's C1 condition (AbortController timeout on `setupTestConnection` and `setupSaveConfig`) is met. Commit `368771b` applied `SETUP_TIMEOUT_MS = 60_000` to both functions with correct timer cleanup and user-readable abort messages. All prior blocking findings from councils 1-5 remain resolved and verified in source.

### Arbiter Recommendation

**CONDITIONAL**

Both C-New-1 and C-New-2 are substantiated defects in new code introduced by this PR. The debate established the following facts without dispute: (1) the 60-second timeout creates a regression on slow-SSH save paths where the pre-fix behavior was success, (2) the raw-JSON error handling is inconsistent with the existing `apiFetch` pattern in the same file, and (3) both fixes are trivial and well-specified. The ADVOCATE's probability argument — that the triggering scenario is narrow — is reasonable but does not overcome the combination of trivial fix cost, non-technical target audience, and the irony of the timeout creating a failure on the exact path it was designed to protect. When both sides agree the defects are real and the fixes are small, deferring to a follow-up is harder to justify than fixing in-PR.

### Conditions

1. **C-New-2 (raw JSON error surface)**: In `setupTestConnection` (api.ts:157-161) and `setupSaveConfig` (api.ts:183-187), replace the raw `throw new Error(`...failed: ${text}`)` with JSON-parsing error extraction matching the `apiFetch` pattern at api.ts:54-59.

2. **C-New-1 (slow-SSH save timeout recovery)**: In the `AbortError` catch path of `setupSaveConfig` (api.ts:175-179), before surfacing the timeout error, poll `/api/status` once. If `mode === 'normal'`, call `window.location.reload()` — reusing the same logic already present in `SetupStep4Confirm.svelte:73-75`. This ensures that if the server completed while the frontend timed out, the user transitions to the app instead of seeing a misleading error.

### Suggested Fixes

All verified findings are fixed in the current PR — no "follow-on" tier exists.

#### Fixes (all in-PR)

**Fix 1 — C-New-2: Parse JSON errors in setup fetch functions**

CITE: `frontend/src/lib/api.ts` L:157-161, L:183-187

Replace the raw-text error throw in both `setupTestConnection` and `setupSaveConfig` with the same JSON-parsing block used by `apiFetch` (L:54-59):

```typescript
if (!res.ok) {
  const text = await res.text().catch(() => '');
  let message = `Setup [operation] failed (${res.status})`;
  try {
    const body = JSON.parse(text);
    if (body?.error) message = body.error;
  } catch {
    if (text) message += `: ${text}`;
  }
  throw new Error(message);
}
```

**Fix 2 — C-New-1: Poll /api/status on save timeout before surfacing error**

CITE: `frontend/src/lib/api.ts` L:175-179

In the `AbortError` catch of `setupSaveConfig`, before throwing the timeout error, check whether the server already completed:

```typescript
if (e instanceof DOMException && e.name === 'AbortError') {
  // Server may have completed while we timed out — check before erroring
  try {
    const status = await fetch('/api/status').then(r => r.json());
    if (status?.mode === 'normal') {
      window.location.reload();
      return undefined as unknown as SetupSaveResponse; // unreachable after reload
    }
  } catch { /* status check failed — fall through to timeout error */ }
  throw new Error('Config save timed out — SSH tunnel negotiation may be slow. Try again.');
}
```

Note: The `window.location.reload()` approach is already the established recovery pattern in `SetupStep4Confirm.svelte:75`. An alternative implementation could return a sentinel value and let the component handle the reload, but the effect is identical.

#### PR Description Amendments

Note in the PR description that setup API functions use a 60-second timeout (`SETUP_TIMEOUT_MS`) to accommodate SSH tunnel negotiation, longer than the 30-second default for normal API calls. (This was already suggested by the fifth council.)

#### Critical Discoveries (informational)

None identified during this debate.
