# Council Recommendation: PR #31 — First-Run Setup Wizard (Third Council)

**Date:** 2026-04-11
**Motion:** PR #31 (epic: First-Run Setup Wizard) — after applying all prior council conditions, the implementation is correct, complete, and secure and is ready to merge into main.
**Motion Type:** CODE
**Branch:** `epic/5-first-run-setup-wizard`
**Prior Councils:** 2 (2026-04-11, 2026-04-11)
**Rounds Completed:** 2
**Panel:** 1 Advocate, 2 Critics, 1 Questioner, 1 Arbiter
**Verdict:** CONDITIONAL

---

## Summary

The First-Run Setup Wizard implements a multi-step Svelte 5 frontend and Axum backend flow that configures SeeKi on first launch without requiring users to manually edit TOML files. All five first-council fixes and all three second-council conditions were verified as correctly implemented. However, this third council surfaced two new findings at blocking severity: a hardcoded server address in the frontend confirmation step that silently misconfigures every wizard save, and a missing secrets-file cleanup on chmod failure that can leave SSH credentials world-readable permanently.

---

## Verdict Rationale

Two verified findings meet or exceed the MEDIUM blocking threshold. Finding B1 (hardcoded server config) is rated HIGH — it affects every single wizard run, writing `localhost:8080` into `seeki.toml` instead of the backend's actual bind address `127.0.0.1:3141`, causing the server to bind to the wrong address on next restart. Finding B2 (secrets file orphaned on chmod failure) is rated MEDIUM — on chmod failure (SELinux, unusual filesystem), `.seeki.secrets` containing SSH credentials persists with default umask (0644) while the error handler only removes `seeki.toml`. Neither finding was successfully rebutted. Both have clear, surgical fixes.

---

## Findings

### Blocking

#### B1 — Hardcoded Server Address in Frontend (HIGH)

**File:** `frontend/src/components/SetupStep4Confirm.svelte:45`
```javascript
server: { host: 'localhost', port: 8080 },
```
**Expected:** `{ host: '127.0.0.1', port: 3141 }` (matching `config.rs:96-101` defaults)

The frontend sends explicit `host` and `port` values in `SaveConfigRequest`, overriding the `serde(default)` annotations on `SaveServerConfig` (`setup.rs:66-71`). The backend's `build_config_toml` (`setup.rs:407-410`) faithfully writes these into `seeki.toml`. On next restart, the server binds to `localhost:8080` instead of the expected `127.0.0.1:3141`. Every wizard completion produces a silently misconfigured server.

**Independently verified:** `config.rs:96` returns `"127.0.0.1"`, `config.rs:100` returns `3141`. `setup.rs:408-409` writes `req.server.host` and `req.server.port` directly. No override mechanism exists in `WizardData` or any prop.

**Fix:**
- **Option A (preferred):** Remove the `server` key from the `setupSaveConfig` call body entirely, letting `serde(default)` on `SaveServerConfig` produce `127.0.0.1:3141`.
- **Option B:** Accept the server bind address as a prop from the parent component, or read it from the window origin.

---

#### B2 — Secrets File Not Deleted on chmod Failure (MEDIUM)

**File:** `src/api/setup.rs:314-323` (error handler) and `src/api/setup.rs:511-521` (`write_secrets_file`)

`write_secrets_file` performs a two-step operation: `std::fs::write` at default umask (L:512), then `set_permissions(0o600)` (L:516-519). If `set_permissions` fails (SELinux denial, read-only FS attribute, unusual mount options), the function returns `Err`. The caller at L:314-323 removes `seeki.toml` (L:317) but does **not** remove `.seeki.secrets`. The file persists world-readable (typically 0644) containing SSH passphrases or passwords.

Compare with the later error paths at L:329-330 and L:343-344, which correctly remove both files. The L:317 path is the sole inconsistency.

**Fix:**
```rust
// src/api/setup.rs:317 — add before the seeki.toml removal:
let _ = std::fs::remove_file(".seeki.secrets");
let _ = std::fs::remove_file("seeki.toml");
```

---

### Non-Blocking

#### NB1 — Mode Guard TOCTOU in save_config (LOW)
**File:** `setup.rs:221` (read lock) → `setup.rs:358` (write lock)

The read lock at L:221 drops after the guard check. A concurrent `save_config` request could pass the guard before the first acquires the write lock at L:358. Mitigated: local-only, 10-second first-run window, worst case is config written twice with identical data. No security impact.

**Recommended improvement:** Hold a write lock for the entire `save_config` body.

---

#### NB2 — seeki.toml Written at Default Umask (LOW)
**File:** `setup.rs:296-304`

`seeki.toml` is not chmod'd after write. The DB URL (with embedded credentials) is world-readable. Downgraded to LOW — consistent with ecosystem convention (`.pgpass`, `DATABASE_URL`, Rails `database.yml`).

---

#### NB3 — SSRF Surface via test_connection (LOW)
**File:** `setup.rs:17-19`

`TestConnectionRequest.url` is passed to the PostgreSQL driver (PG wire protocol only, not HTTP). Attack surface is port-probing, mode-guarded to setup phase, localhost-only by default.

---

#### NB4 — Drop Impl No-Op on SshTunnel (LOW)
**File:** `src/ssh/mod.rs:79-84`

`let _ = &self.session;` is a no-op reference borrow. `openssh::Session` has its own Drop that closes the ControlMaster. No resource leak. Comment is misleading dead code.

---

#### NB5 — SSH Credential Cross-Binding in UI (LOW)
**File:** `frontend/src/components/SetupStep1Connection.svelte:190`

SSH password input binds to `wizardData.ssh.key_passphrase` — same field as key passphrase. No field-clearing on auth method switch. Harmless today because password-based SSH auth is unimplemented (`ssh/mod.rs:43-47` unconditionally bails). Should be fixed before implementing password auth.

---

### Prior Council Fixes Verified

| # | Condition | Status | Citation |
|---|-----------|--------|----------|
| 1 | `KnownHosts::Add` instead of `KnownHosts::Accept` | ✅ Verified | `ssh/mod.rs:26` |
| 2 | Passphrase → `ssh-add {key_path}` error message | ✅ Verified | `ssh/mod.rs:33-35` |
| 3 | TOCTOU port allocation documented | ✅ Verified | `ssh/mod.rs:19-20` |
| 4 | Secrets cleaned up on rollback (config parse + DB connect) | ✅ Verified | `setup.rs:329-330`, `setup.rs:343-344` |
| 5 | Mode guards return HTTP 409 Conflict | ✅ Verified | `setup.rs:118-128`, `setup.rs:221-229` |
| 6 | Tests assert 409 on non-setup mode | ✅ Verified | `setup.rs:841-862`, `setup.rs:865-890` |

---

## Withdrawn / Rejected Claims

| Claim | Proponent | Outcome | Reason |
|-------|-----------|---------|--------|
| Race condition = CRITICAL | CRITIC-1 | **Withdrawn to LOW** | Local-only, 10s window, no corruption. |
| Secrets TOCTOU = HIGH (nanosecond window) | CRITIC-1 | **Refined** | Replaced by the more concrete chmod-failure cleanup bug (B2). |
| seeki.toml umask = MEDIUM | CRITIC-1 | **Withdrawn to LOW** | Ecosystem convention. |
| Credential cross-contamination = MEDIUM | CRITIC-2 | **Downgraded to LOW** | Password auth is unimplemented dead code (`ssh/mod.rs:43-47`). |

---

## Panel Positions (Final)

| Role | Final Position | Key Concessions |
|------|---------------|-----------------|
| ADVOCATE-1 | CONDITIONAL | Accepted B1 (hardcoded server) as HIGH. Accepted B2 as real bug. |
| CRITIC-1 | CONDITIONAL | Withdrew race CRITICAL→LOW. Withdrew seeki.toml MEDIUM→LOW. Refined secrets finding to chmod-failure path. |
| CRITIC-2 | CONDITIONAL | Withdrew Drop no-op to LOW. Maintained B1 as HIGH (confirmed by all). Accepted credential binding as LOW. |

---

## Recommendation

**Merge after fixing the two blocking conditions above.** Total fix effort is < 15 lines of code.

### Fix Checklist

- [ ] **B1:** Remove or correct hardcoded `{ host: 'localhost', port: 8080 }` at `SetupStep4Confirm.svelte:45`
- [ ] **B2:** Add `let _ = std::fs::remove_file(".seeki.secrets");` at `setup.rs:317`, before the existing `remove_file("seeki.toml")`

### Optional Improvements

- Write `.seeki.secrets` via `OpenOptions` with `mode(0o600)` atomically (removes TOCTOU class entirely)
- Clear `wizardData.ssh.key_passphrase` and `wizardData.ssh.password` on auth method change in `SetupStep1Connection.svelte`
- Remove the no-op `Drop` impl on `SshTunnel` (or replace with explicit comment that field-drop handles teardown)

---

## Verification Results

| # | Finding | Citations | Verdict | Action |
|---|---------|-----------|---------|--------|
| B1 | Hardcoded `localhost:8080` in frontend save | `SetupStep4Confirm.svelte:45`, `config.rs:96-101`, `setup.rs:407-410` | **VERIFIED — HIGH** | MUST FIX |
| B2 | `.seeki.secrets` not deleted on chmod failure | `setup.rs:314-323`, `setup.rs:511-521` | **VERIFIED — MEDIUM** | MUST FIX |
| NB1 | Mode guard TOCTOU | `setup.rs:221`, `setup.rs:358` | VERIFIED — LOW | Note |
| NB2 | `seeki.toml` default umask | `setup.rs:296-304` | VERIFIED — LOW | Note |
| NB3 | SSRF via test_connection | `setup.rs:17-19` | VERIFIED — LOW | Note |
| NB4 | Drop no-op on SshTunnel | `ssh/mod.rs:79-84` | VERIFIED — LOW | Note |
| NB5 | SSH credential cross-binding | `SetupStep1Connection.svelte:190` | VERIFIED — LOW | Note |

Verification: 7 verified, 0 phantom, 4 withdrawn/downgraded. All prior council fixes confirmed.

*Third council convened under adversarial-council skill. Arbiter: Claude Opus 4.6.*
