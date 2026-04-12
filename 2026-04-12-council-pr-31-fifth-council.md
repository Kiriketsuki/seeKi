## Adversarial Council — PR #31 First-Run Setup Wizard (Fifth Council)

> Convened: 2026-04-12 | Advocates: 1 | Critics: 1 | Rounds: 3/4 | Motion type: CODE

### Motion
PR #31 (epic: First-Run Setup Wizard) — after applying all prior council fixes (four councils total), the implementation is correct, complete, and secure and ready to merge into main.

### Advocate Positions
**ADVOCATE-1**: All four prior council blocking findings are resolved. The B1 fix (`#[serde(default)]` on `server` field, `#[derive(Default)]` on `SaveServerConfig`) is verified at `src/api/setup.rs:59,67` with a regression test at line 832. Mode guards (409 CONFLICT), secrets cleanup on failure, atomic TOML write, and post-save polling are all correct. The SSH `password` field binding is correct in current code. The remaining C1 timeout issue is a bounded (~75–120s), recoverable (page refresh) UX gap in a net-new feature — not a correctness, security, or data integrity issue. In final rounds, ADVOCATE-1 conceded C1 is higher severity than "quality-of-life gap" — it is a real UX defect — but argued CONDITIONAL is the proportionate ruling over AGAINST. Also noted that setup operations (SSH tunnel negotiation) may legitimately need a longer timeout than the 30s used by `apiFetch`, suggesting 60s as setup-appropriate.

### Critic Positions
**CRITIC-1**: The setup API functions `setupTestConnection` and `setupSaveConfig` (`api.ts:133-161`) use raw `fetch` without the `AbortController` timeout established by `apiFetch` (`api.ts:36-48`) in the same file. When testing SSH connections to unreachable hosts, the wizard's primary action button freezes for ~75–120 seconds before the user can take any action. This is an internal inconsistency within the PR itself — `apiFetch` establishes the timeout contract and the setup functions break it. In final rounds, CRITIC-1 withdrew "permanently frozen / no recovery path" (page refresh works) and "wait indefinitely" (OS TCP timeout bounds it), but maintained the corrected characterization is still blocking: a 75–120 second frozen primary action button, where the fix is established and available in the same file.

### Questioner Findings
No questioner in this 1v1 council format.

### Key Conflicts
- **C1 severity (timeout bypass)** — ADVOCATE-1 argued the 75-second wait is bounded, recoverable via page refresh, and does not affect correctness/security. CRITIC-1 argued the primary wizard action freezing for 75 seconds with no recovery path (besides refresh) is unacceptable for merge into main. **Resolved by ARBITER: the fix is trivial enough to apply in-PR; CONDITIONAL on fixing.**
- **C2 (type contract mismatch)** — CRITIC-1 raised `error_source: "ssh_config"` not in frontend union type. ADVOCATE-1 demonstrated the code path is unreachable from wizard UI (hard-coded `<select>` options). **Resolved: CRITIC-1 conceded to LOW.**
- **C3 (TOCTOU race)** — Previously conceded in fourth council. ARBITER ruled out of scope. **Resolved: CRITIC-1 accepted ruling.**

### Concessions
- **CRITIC-1** conceded C2 (type mismatch) to LOW severity after ARBITER challenged the user-visible impact and ADVOCATE-1 demonstrated the code path is unreachable from the wizard UI.
- **CRITIC-1** conceded C3 (TOCTOU race) as out of scope, accepting the ARBITER's ruling that it was already conceded in the fourth council.
- **CRITIC-1** withdrew "permanently frozen / no recovery path" and "wait indefinitely" characterizations of C1, accepting that page refresh is a real (if inelegant) recovery path and the wait is bounded by OS TCP timeout.
- **ADVOCATE-1** conceded C1 is higher severity than "quality-of-life gap" — it is a real UX defect in the wizard's primary action. Disputed whether it crosses the merge threshold, but accepted CONDITIONAL as proportionate if the ARBITER finds it does.

### Regression Lineage
The fourth council's B1 blocking issue (contract mismatch — `SaveConfigRequest.server` required but frontend omitted it) is **verified fixed**:
- `src/api/setup.rs:59`: `#[serde(default)]` on `server` field
- `src/api/setup.rs:67`: `#[derive(Deserialize, Default)]` on `SaveServerConfig`
- `src/api/setup.rs:832-865`: Regression test `save_config_accepts_request_without_server_field`
- `frontend/src/components/SetupStep4Confirm.svelte:44-58`: Correctly omits `server` from payload

The fourth council's NB1 (SSH password binds to `key_passphrase`) appears resolved in current code — `SetupStep1Connection.svelte:190` now binds to `wizardData.ssh.password`, and `wizard_to_ssh` (`setup.rs:394-397`) correctly maps both `key_passphrase` and `password` fields.

The fourth council's NB2 (Drop no-op in `ssh/mod.rs:79-84`) remains as documented with an explanatory comment. Clippy warns about the unused `session` field but the field is held for its `Drop` side effect.

No new regression was introduced by the fourth council fixes.

### Arbiter Recommendation
**CONDITIONAL**

The PR delivers a complete, well-structured first-run setup wizard across 70 files and ~12,900 lines. All prior council blocking findings (four councils) are verified fixed. All 50 Rust tests pass. The single remaining issue — C1, the missing `AbortController` timeout on the two setup API functions — is a genuine UX deficiency in the wizard's primary user flow. While the ADVOCATE correctly notes this is bounded (~75s, not infinite) and recoverable (page refresh), the CRITIC correctly notes this is an internal inconsistency within the PR's own `api.ts` file and the fix is trivial. A 75-second frozen UI on the wizard's primary action, in a common error scenario (unreachable SSH host), should be fixed before merging into main. The fix does not require architectural changes — it requires applying the existing `apiFetch` timeout pattern to the two setup functions.

### Conditions (if CONDITIONAL)
1. Add `AbortController` timeout to `setupTestConnection` and `setupSaveConfig` in `frontend/src/lib/api.ts`, consistent with the `apiFetch` pattern established at lines 36-48 of the same file. The timeout duration should be appropriate for setup operations — ADVOCATE-1 raised a valid point that SSH tunnel negotiation may legitimately take longer than the 30s used by `apiFetch`. A setup-specific timeout of 60 seconds is a reasonable choice; the key requirement is that the timeout exists and eventually sets `testState = 'error'` to re-enable the button.

### Suggested Fixes

#### Fixes (all in-PR)

| # | Fix | File | Severity | Rationale |
|---|-----|------|----------|-----------|
| F1 | Add `AbortController` with timeout to `setupTestConnection` | `frontend/src/lib/api.ts` L:133-148 | MEDIUM | Primary wizard action freezes for ~75–120s on unreachable SSH hosts. Use 60s timeout (setup operations need longer than the 30s `apiFetch` default due to SSH tunnel negotiation). The `apiFetch` pattern at L:36-48 provides the implementation template. |
| F2 | Add `AbortController` with timeout to `setupSaveConfig` | `frontend/src/lib/api.ts` L:150-161 | MEDIUM | Same timeout omission as F1. `save_config` also tests the connection before saving, so the same unreachable-host scenario applies. Use same 60s timeout. |

#### Non-Blocking Suggestions

| # | Suggestion | File | Severity | Rationale |
|---|------------|------|----------|-----------|
| S1 | Add `'ssh_config'` to `error_source` union type | `frontend/src/lib/types.ts` L:81 | LOW | Type contract does not match backend emission at `setup.rs:162`. Unreachable through wizard UI but misleading for future developers. |
| S2 | Remove or suppress no-op `Drop` impl | `src/ssh/mod.rs` L:79-84 | LOW | Clippy warns about unused `session` field. The `Drop` impl is empty. Consider `#[allow(dead_code)]` with a comment explaining the field is held for its `Drop` effect, or remove the custom `Drop` impl entirely. |

#### PR Description Amendments
- Note that the setup API functions now use a 60-second timeout (longer than the 30s default to accommodate SSH tunnel negotiation).

#### Critical Discoveries (informational)
None. No security, data loss, or compliance issues were identified in this council.

---

### Prior Council Fix Verification

| Council | Finding | Status | Evidence |
|---------|---------|--------|----------|
| Third | B1: Hardcoded server address | FIXED (via fourth council's approach) | `setup.rs:59` `#[serde(default)]`, `setup.rs:67` `Default` derive |
| Third | B2: Secrets cleanup on chmod failure | FIXED | `setup.rs:319-320` removes both files on failure |
| Fourth | B1: Contract mismatch (server field) | FIXED | `setup.rs:59,67` + regression test at L:832-865 |
| Fourth | NB1: SSH password → key_passphrase | FIXED | `SetupStep1Connection.svelte:190` binds to `.password` |
| Fourth | NB2: Drop no-op | ACKNOWLEDGED | `ssh/mod.rs:79-84` with explanatory comment |

### Test Results
- `cargo test`: 50 passed, 0 failed
- `cargo clippy`: 1 warning (expected — `session` field held for Drop effect)

---

*Fifth council convened under adversarial-council skill. Arbiter: Claude Opus 4.6.*
