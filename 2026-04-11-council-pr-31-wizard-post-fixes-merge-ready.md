# Adversarial Council Recommendation

**Motion:** PR #31 (epic: First-Run Setup Wizard) — after applying prior council fixes, the implementation is correct, complete, and secure and is ready to merge into main.

**PR:** #31  
**Date:** 2026-04-11  
**Rounds:** 2 of 4 (convergence achieved)  
**Panel:** 1 Advocate, 2 Critics, 1 Questioner

---

## Verdict: CONDITIONAL

The core implementation is sound. All 5 issues from the prior council (2026-04-11) were verified as correctly fixed. However, three new gaps emerged during this review that require resolution before merge.

### Conditions for Merge

| # | Severity | Issue | Required Fix |
|---|----------|-------|--------------|
| 1 | HIGH | Secrets file not removed on rollback | Add `remove_file(".seeki.secrets")` at L:308 and L:320 |
| 2 | MEDIUM | Mode guards return HTTP 200 instead of 409 | Return `Err((StatusCode::CONFLICT, ...))` |
| 3 | MEDIUM | Zero test coverage for mode guard rejection | Call `test_router_with_mode` with `AppMode::Normal` |

All three are mechanical fixes totaling <30 lines of code.

---

## Findings

### Finding 1: Secrets File Leak on Rollback (HIGH)

**Description:** When `save_config` fails after writing `.seeki.secrets` (either at config parse or DB connection), the rollback removes `seeki.toml` but leaves `.seeki.secrets` on disk containing SSH passphrases and database passwords in cleartext.

**Evidence:**
- Config parse error path removes only `seeki.toml`:
  CITE: `src/api/setup.rs` L:308-312
- DB connection error path removes only `seeki.toml`:
  CITE: `src/api/setup.rs` L:319-325
- Secrets file written at L:293-302 is never cleaned up in either failure path

**Risk:** CWE-312 (Cleartext Storage of Sensitive Information). Malicious process running as same user can read persisted credentials. Window exists on flaky network or misconfigured database.

**Required Fix:**
```rust
// At L:308, before remove_file("seeki.toml"):
let _ = std::fs::remove_file(".seeki.secrets");

// At L:320, before remove_file("seeki.toml"):
let _ = std::fs::remove_file(".seeki.secrets");
```

**Debate Status:** Advocate conceded. Both critics held firm. Unanimous agreement this requires fix.

---

### Finding 2: Mode Guards Return HTTP 200 (MEDIUM)

**Description:** Mode guards reject requests when `AppMode::Normal` but return HTTP 200 with `{success: false}` instead of HTTP 409 Conflict.

**Evidence:**
- test_connection mode guard:
  CITE: `src/api/setup.rs` L:117-125
- save_config mode guard:
  CITE: `src/api/setup.rs` L:211-217

**Risk:** API contract inconsistency. Frontend must check both HTTP status AND `response.success` field, creating integration burden. Prior council suggested HTTP 409 as the recommended pattern.

**Required Fix:**
```rust
// Replace Json return with:
return Err((StatusCode::CONFLICT, "Setup is already complete"));
```

**Debate Status:** Advocate conceded. CRITIC-2 downgraded from blocking to medium (prior council suggested, did not mandate). Improvement recommended.

---

### Finding 3: Mode Guard Tests Missing (MEDIUM)

**Description:** Test helper `test_router_with_mode` exists but is never called. Zero test coverage for the mode guard rejection path.

**Evidence:**
- Unused test helper defined:
  CITE: `src/api/setup.rs` L:515-520
- All existing tests use `test_router()` which starts in `AppMode::Setup`
- No tests verify that `AppMode::Normal` correctly rejects setup endpoints

**Risk:** Regression risk. Mode guards are a security boundary but have no automated verification.

**Required Fix:**
```rust
#[tokio::test]
async fn test_connection_rejects_when_setup_complete() {
    let mode = Arc::new(RwLock::new(AppMode::Normal));
    let app = test_router_with_mode(mode);
    // ... assert 409 response
}
```

**Debate Status:** Advocate conceded. Both critics held firm. Unanimous agreement tests are required.

---

## Prior Council Fixes — Verification

All 5 issues from the first council (2026-04-11) were verified as correctly implemented:

| Issue | Status | Location |
|-------|--------|----------|
| SSH passphrase silently ignored | ✓ FIXED | `src/ssh/mod.rs` L:31-35 |
| KnownHosts::Accept MITM risk | ✓ FIXED | `src/ssh/mod.rs` L:26 (KnownHosts::Add) |
| save_config missing mode guard | ✓ FIXED | `src/api/setup.rs` L:211-217 |
| test_connection missing mode guard | ✓ FIXED | `src/api/setup.rs` L:117-125 |
| TOCTOU port race | ✓ DOCUMENTED | `src/ssh/mod.rs` L:19-20 |

---

## Withdrawn / Rejected Claims

### KnownHosts::Add vs TOFU with Confirmation

**Claim:** KnownHosts::Add provides "silent TOFU" without fingerprint confirmation.

**Resolution:** WITHDRAWN by CRITIC-1. Prior council offered three options; author chose Option 3 (KnownHosts::Add). This provides accept-new semantics and fails loudly on key change. Valid implementation choice.

---

## Panel Positions (Final)

| Role | Position | Key Concessions |
|------|----------|-----------------|
| ADVOCATE-1 | CONDITIONAL | Conceded all three findings; supports mechanical fix before merge |
| CRITIC-1 | CONDITIONAL | Withdrew KnownHosts objection; holds on secrets and tests |
| CRITIC-2 | CONDITIONAL | Downgraded HTTP 409 to medium; holds on secrets and tests |

---

## Recommendation

**Merge after fixing the three conditions above.** The implementation is fundamentally sound — prior council fixes are correct, security model is appropriate, and the gaps are mechanical (<30 lines total). Estimated fix time: 15-30 minutes.

### Fix Checklist

- [ ] Add `remove_file(".seeki.secrets")` at setup.rs L:308
- [ ] Add `remove_file(".seeki.secrets")` at setup.rs L:320  
- [ ] Change mode guard returns to HTTP 409 at L:118-124 and L:211-217
- [ ] Add test calling `test_router_with_mode(AppMode::Normal)` asserting 409

---

### Verification Results
| # | Finding | Citations | Verdict | Action |
|---|---------|-----------|---------|--------|
| 1 | .seeki.secrets not removed — config parse failure | `src/api/setup.rs` L:308 | VERIFIED | Retained |
| 2 | .seeki.secrets not removed — DB connect failure | `src/api/setup.rs` L:320 | VERIFIED | Retained |
| 3 | Mode guard returns HTTP 200 (test_connection) | `src/api/setup.rs` L:118 | VERIFIED | Retained |
| 4 | Mode guard returns HTTP 200 (save_config) | `src/api/setup.rs` L:212 | VERIFIED | Retained |
| 5 | test_router_with_mode defined but never called | `src/api/setup.rs` L:515 | VERIFIED | Retained |

Verification: 5 verified, 0 phantom (purged), 0 unverified — All findings verified against codebase.

*Council convened under adversarial-council skill. Arbiter: Claude.*
