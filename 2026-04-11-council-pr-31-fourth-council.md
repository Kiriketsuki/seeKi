# Council Recommendation: PR #31 — First-Run Setup Wizard (Fourth Council)

**Date:** 2026-04-11
**Motion:** PR #31 (epic: First-Run Setup Wizard) — after applying all prior council blocking conditions (third council B1: hardcoded server address, B2: secrets cleanup on chmod failure), the implementation is correct, complete, and secure and is ready to merge into main.
**Motion Type:** CODE
**Branch:** `epic/5-first-run-setup-wizard`
**Prior Councils:** 3 (2026-04-11, 2026-04-11, 2026-04-11)
**Rounds Completed:** 2
**Panel:** 1 Advocate, 2 Critics, 1 Questioner, 1 Arbiter
**Verdict:** CONDITIONAL

---

## Summary

The fourth council reviewed PR #31 following remediation of blocking issues identified in the third council (B1: hardcoded server address, B2: secrets cleanup on chmod failure). Upon verification, B2 was correctly fixed — `src/api/setup.rs:317-318` now removes both `.seeki.secrets` and `seeki.toml` on chmod failure.

However, a critical regression was introduced in the B1 fix. The frontend component `SetupStep4Confirm.svelte` now omits the `server` field entirely from the save request body, but the backend `SaveConfigRequest` struct requires this field for deserialization. This contract mismatch will cause a guaranteed serde error ("missing field 'server'") when users attempt to save their configuration, rendering the wizard non-functional.

All three council agents (advocate and both critics) independently identified and confirmed this blocking issue. A secondary finding regarding SSH password input binding to `key_passphrase` was debated but downgraded to LOW severity since password-based SSH auth is explicitly not implemented and returns an error at runtime.

---

## Verdict Rationale

The motion cannot pass because a blocking regression exists. While the prior council's B2 fix (secrets cleanup) was correctly applied, the B1 fix introduced a new deserialization failure. The `server` field in `SaveConfigRequest` is required (no `Option<>`, no `#[serde(default)]`), but the frontend omits it entirely. This is a guaranteed runtime failure that makes the wizard unusable.

The fix is straightforward: either (a) add `#[serde(default)]` to the `server` field in `SaveConfigRequest` and derive `Default` on `SaveServerConfig`, OR (b) include the `server` object in the frontend save request. Both approaches are valid; option (a) preserves the semantic that server config should default to sensible values.

---

## Findings

### Blocking

#### B1: Contract Mismatch — Server Field Omission

**Severity:** CRITICAL
**Status:** BLOCKING

| Location | Evidence |
|----------|----------|
| `src/api/setup.rs:58` | `server: SaveServerConfig` — required, no `Option<>`, no `#[serde(default)]` |
| `src/api/setup.rs:65-71` | `SaveServerConfig` derives only `Deserialize`, not `Default` |
| `frontend/src/lib/types.ts:105` | `server: { host: string; port: number }` — required in TypeScript interface |
| `frontend/src/components/SetupStep4Confirm.svelte:44-58` | Save request body omits `server` key entirely |

**Impact:** Serde will reject the JSON payload with "missing field 'server'" error. The setup wizard cannot complete.

**Required Fix (Option A — Recommended):**
1. Add `#[derive(Default)]` to `SaveServerConfig` in `src/api/setup.rs:65`
2. Add `#[serde(default)]` to the `server` field in `SaveConfigRequest` at line 58

**Required Fix (Option B — Alternative):**
1. Add `server: { host: '127.0.0.1', port: 3141 }` to the save request in `SetupStep4Confirm.svelte:44`

### Non-Blocking

#### NB1: SSH Password Binds to key_passphrase

**Severity:** LOW
**File:** `frontend/src/components/SetupStep1Connection.svelte:190`

The SSH password input field binds to `wizardData.ssh.key_passphrase` instead of a dedicated password field. This is semantically incorrect.

**Mitigating Factor:** Password-based SSH auth is not implemented (`src/ssh/mod.rs:43-47` returns error), so this field is never used. The UI already warns users that password auth "may not be supported."

**Recommended Future Fix:** When implementing password auth, add a `password` field to `SshWizardConfig` and bind the password input to it.

#### NB2: Drop Implementation No-Op

**Severity:** LOW
**File:** `src/ssh/mod.rs:79-84`

The `Drop` impl for `SshTunnel` contains `let _ = &self.session;` which has no effect. The session is dropped automatically. This is misleading dead code but has no functional impact.

#### NB3–NB5: Previously Identified (from prior councils)

These remain as documented in prior council reports and are non-blocking.

### Prior Council Fixes Verified

| Council | Finding | Fix Applied | Status |
|---------|---------|-------------|--------|
| Third | B1: Hardcoded server address `127.0.0.1:3141` | Server field removed from frontend request | ❌ **REGRESSED** — now causes deserialization failure |
| Third | B2: Secrets cleanup on chmod failure | Added cleanup at `setup.rs:317-318` | ✅ Verified |

---

## Withdrawn / Rejected Claims

| Claim | Original Severity | Final Status | Reason |
|-------|-------------------|--------------|--------|
| NB1 TOCTOU data corruption race | MEDIUM-HIGH | LOW | Conceded by CRITIC-1 in rebuttal; single-user first-run wizard makes concurrent saves operationally improbable |
| Polling "spins forever" | MEDIUM | LOW | Conceded by CRITIC-2 in rebuttal; 20×250ms = 5-second controlled timeout with reload, not infinite spin |
| SSH credential binding | MEDIUM | LOW | Password auth not implemented; field never used; UI already warns users |

---

## Panel Positions (Final)

| Agent | Final Position | Key Concessions |
|-------|----------------|-----------------|
| ADVOCATE-1 | CONDITIONAL | Conceded B1 incomplete; conceded SSH binding semantically wrong but LOW severity |
| CRITIC-1 | CONDITIONAL (B1 blocks) | Conceded NB1 TOCTOU to LOW; confirmed B1 blocking |
| CRITIC-2 | CONDITIONAL (B1 blocks) | Conceded polling to LOW; confirmed B1 blocking |
| QUESTIONER | N/A (procedural) | Probes led to concessions on NB1, polling, and Drop no-op |
| ARBITER | CONDITIONAL | B1 verified blocking; B2 verified fixed; SSH binding LOW |

---

## Recommendation

**CONDITIONAL MERGE** — PR #31 may be merged after addressing one blocking fix.

### Fix Checklist

- [ ] **B1:** Add `#[derive(Default)]` to `SaveServerConfig` (setup.rs:65) and `#[serde(default)]` to `server` field in `SaveConfigRequest` (setup.rs:58) — OR — include `server: { host: '127.0.0.1', port: 3141 }` in frontend save request

### Optional Improvements

- Add dedicated `password` field to `SshWizardConfig` for future password auth implementation
- Remove no-op `Drop` impl at `ssh/mod.rs:79-84` or add explanatory comment
- Consider adding integration test that exercises the full save flow to catch contract mismatches

---

## Verification Results

| # | Finding | Citations | Verdict | Action |
|---|---------|-----------|---------|--------|
| B1 | Server field omitted from save request | `setup.rs:58`, `SetupStep4Confirm.svelte:44-58` | **BLOCKING** | Must fix before merge |
| B2-prior | Secrets cleanup on chmod failure | `setup.rs:317-318` | ✅ FIXED | Verified |
| NB1 | SSH password binds to key_passphrase | `SetupStep1Connection.svelte:190` | LOW | Future fix |
| NB2 | Drop no-op | `ssh/mod.rs:79-84` | LOW | Optional cleanup |

---

*Fourth council convened under adversarial-council skill. Arbiter: Claude Opus 4.5.*
