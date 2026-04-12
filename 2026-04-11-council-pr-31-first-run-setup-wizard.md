# Council Recommendation: PR #31 — First-Run Setup Wizard

**Date:** 2026-04-11  
**Motion:** "PR #31 (epic: First-Run Setup Wizard) — the implementation is correct, complete, and secure"  
**Motion Type:** CODE  
**Branch:** `epic/5-first-run-setup-wizard`  
**Verdict:** CONDITIONAL  

---

## Summary

The First-Run Setup Wizard delivers a sound architectural foundation with correct hot-swap mechanics (`Arc<RwLock<AppMode>>`), proper secrets isolation (0600 permissions), and a reasonable 4-step wizard flow. However, the council identified **5 verified issues** — 1 High security, 1 Medium security, 2 bugs, and 1 improvement — that must be addressed before merge.

The advocate made **major concessions** on SSH passphrase handling, `KnownHosts::Accept`, and missing mode guards. These concessions were verified against source code by the team lead.

---

## Verdict Rationale

**CONDITIONAL** — The core architecture is sound, but the following blocking issues prevent a clean PASS:

1. **Security (High):** SSH MITM vulnerability via `KnownHosts::Accept` allows transparent credential interception
2. **Security (Medium):** `save_config` endpoint has no mode guard, enabling config overwrite post-setup by any localhost process
3. **Bug:** SSH passphrase is collected but never used, breaking encrypted key authentication entirely

These issues are fixable within the PR scope. No architectural changes required.

---

## Suggested Fixes (In-PR)

### 1. SSH Passphrase Never Used — Bug

**Problem:** The `_secrets` parameter is accepted but never used. Users with passphrase-protected SSH keys cannot authenticate.

**Evidence:** Parameter has underscore prefix (Rust convention for unused), `builder.keyfile()` is called without passphrase.

**Fix:** When `SshAuthMethod::Key` and `secrets.ssh_key_passphrase.is_some()`, pass the passphrase to the openssh builder or use `ssh-add` pre-loading.

CITE: `src/ssh/mod.rs` L:15

---

### 2. KnownHosts::Accept — Security (High)

**Problem:** Unconditionally accepts any SSH host key. Enables transparent MITM attack to capture database credentials sent through the tunnel.

**Evidence:** Hardcoded `KnownHosts::Accept` with no user confirmation or fingerprint display.

**Fix:** Replace with one of:
- `KnownHosts::Strict` (require pre-existing entry)
- TOFU flow: display fingerprint on first connect, require user confirmation, persist to known_hosts
- `KnownHosts::AddKeys` (accept-new semantics, warn if changed)

CITE: `src/ssh/mod.rs` L:25

---

### 3. `save_config` No Mode Guard — Security (Medium)

**Problem:** Endpoint accepts `Extension(mode)` but never checks it. Post-setup, any localhost process can overwrite `seeki.toml` and `.seeki.secrets`.

**Evidence:** Function proceeds directly to validation and file writes without mode check. CORS only restricts browser requests; curl/scripts bypass it.

**Fix:** Add early guard:
```rust
if !matches!(*mode.read().await, AppMode::Setup) {
    return Err((StatusCode::CONFLICT, "Setup already complete"));
}
```

CITE: `src/api/setup.rs` L:200

---

### 4. `test_connection` No Mode Guard — Improvement

**Problem:** Endpoint lacks mode guard. While it doesn't persist state, it allows credential probing post-setup.

**Evidence:** No mode check in handler.

**Fix:** Add mode guard returning 409 Conflict if `AppMode::Normal`.

CITE: `src/api/setup.rs` L:113

---

### 5. TOCTOU Port Race — Bug (Low)

**Problem:** Ephemeral port is allocated, listener dropped, then port used for SSH tunnel. Another process could claim the port in between.

**Evidence:** `drop(listener)` at L:21, `request_port_forward` later.

**Fix (Optional):** Either:
- Pass port 0 directly to openssh's local forwarding and read the assigned port back
- Accept the microsecond race window with a documenting comment (failure is immediate and obvious)

CITE: `src/ssh/mod.rs` L:19

---

## Findings Not Requiring Fixes

| Finding | Disposition |
|---------|-------------|
| Concurrent `save_config` race | LOW risk — `RwLock` makes AppMode swap atomic; file write race is same-user double-submit edge case. Mode guard fix (#3) eliminates post-setup variant. |
| HTTP over localhost | Mitigated by `127.0.0.1` binding. Requires local privilege escalation. |

---

## Test Command

```bash
cargo test
```

---

## Debate Summary

| Participant | Key Positions |
|-------------|---------------|
| ADVOCATE-1 | Defended architecture, conceded all major security/bug findings |
| CRITIC-1 | Identified `_secrets` dead code and TOCTOU race |
| CRITIC-2 | Identified `KnownHosts::Accept` MITM and missing mode guards |
| QUESTIONER | Probed concurrent save race, SSH passphrase runtime behavior |

**Convergence:** Early (Round 1) — advocate made major concessions on 3 core issues.

---

## Regression Lineage

No regression lineage — this is the initial implementation of the First-Run Setup Wizard (epic/5).

---

## Critical Discoveries

None. All findings are within the PR's implementation scope.

---

## Arbiter Notes

The PR demonstrates solid Rust engineering (proper `Arc<RwLock>` for shared state, rust-embed for assets, rollback-on-failure). The issues found are implementation gaps rather than architectural flaws:

1. **SSH passphrase** — oversight in wiring already-collected data to the SSH builder
2. **KnownHosts** — common mistake when prioritizing "it works" over security defaults
3. **Mode guards** — incomplete state machine enforcement

All fixes are surgical (< 20 lines each) and don't require rearchitecting. Recommend merge after addressing items 1-4. Item 5 (TOCTOU) is at author discretion.

---

**Verdict:** CONDITIONAL  
**Blocking Issues:** 4 (items 1-4)  
**Non-Blocking Issues:** 1 (item 5)

---

### Verification Results
| # | Finding | Citations | Verdict | Action |
|---|---------|-----------|---------|--------|
| 1 | SSH passphrase never used (`_secrets` ignored) | `src/ssh/mod.rs` L:15 | VERIFIED | Retained |
| 2 | `KnownHosts::Accept` MITM vulnerability | `src/ssh/mod.rs` L:25 | VERIFIED | Retained |
| 3 | `save_config` missing mode guard | `src/api/setup.rs` L:200 | VERIFIED | Retained |
| 4 | `test_connection` missing mode guard | `src/api/setup.rs` L:113 | VERIFIED | Retained |
| 5 | TOCTOU port race | `src/ssh/mod.rs` L:19 | VERIFIED | Retained |

Verification: 5 verified, 0 phantom (purged), 0 unverified. All findings verified against codebase.
