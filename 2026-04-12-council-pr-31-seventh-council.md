## Adversarial Council — PR #31 First-Run Setup Wizard (Seventh Council)

> Convened: 2026-04-12 | Advocates: 1 | Critics: 1 | Rounds: 3/4 | Motion type: CODE

### Motion

PR #31 (epic: First-Run Setup Wizard), after applying all six prior council fix sessions — including the sixth council's two CONDITIONAL fixes (C-New-2: JSON error parsing in setup functions; C-New-1: poll /api/status before surfacing save timeout error) — is correct, complete, and ready to merge into main.

### Advocate Positions

**ADVOCATE-1**: Both sixth council conditions are fully implemented in commit `4341224`. C-New-2 (JSON error parsing) is verified at `api.ts:157-166` and `api.ts:198-207`, matching the `apiFetch` pattern exactly. C-New-1 (abort-path status poll) is verified at `api.ts:184-193`, polling `/api/status` once and reloading on `mode === 'normal'` — matching the sixth council's stated requirement verbatim. The localhost deployment target (`127.0.0.1:3141`) means `ECONNREFUSED` fires instantly on loopback, eliminating CRITIC's timeout-on-poll concern. The council process across six sessions constitutes the merge review standard; none required test coverage. The test gap is real but not a blocker under the accumulated review record.

### Critic Positions

**CRITIC-1**: Conceded Defects 1 and 2 during the debate. The sole surviving objection is Defect 3: `setupTestConnection` and `setupSaveConfig` are new functions with zero test coverage in `api.test.ts` — specifically, no tests for the C-New-1 abort-path reload (`api.ts:184-193`) or the C-New-2 JSON error extraction (`api.ts:157-166`, `api.ts:198-207`). Acknowledged ADVOCATE-1's strongest counter (six councils declined to require tests) as a legitimate constraint, but argued the narrower framing: these are specifically the two fix paths the council identified as real defects, now shipped with no automated regression protection. Whether this constitutes an incomplete PR under the motion's "complete" standard is the ARBITER's judgment.

### Questioner Findings

No questioner in this 1v1 council format.

### Key Conflicts

- **Defect 1 (no timeout on inner poll fetch)** — CRITIC argued the `fetch('/api/status')` at `api.ts:187` could hang for 60-120 seconds with no timeout. ADVOCATE rebutted that SeeKi runs on loopback (`127.0.0.1:3141`) where `ECONNREFUSED` is instant. **Resolved**: CRITIC conceded; loopback eliminates the silent-hang scenario.

- **Defect 2 (single poll insufficient for mid-restart)** — CRITIC argued the single poll misses the restart window, leading to a false timeout and a 409 dead-end on retry via the "Fix connection settings" CTA at `SetupStep4Confirm.svelte:104`. ADVOCATE argued the sixth council explicitly mandated "poll once." **Resolved**: CRITIC self-corrected, recognizing the 409-on-retry scenario fails the pre-existence test — the dead-end existed before C-New-1 and the fix strictly improves the outcome. Conceded.

- **Defect 3 (zero test coverage for new functions)** — CRITIC argued `api.test.ts` covers only `assertShape`, with no tests for `setupTestConnection` or `setupSaveConfig`. ADVOCATE acknowledged the gap but argued six councils declined to require tests, making this an ex post facto standard. **Unresolved**: Both sides agree the gap exists; they disagree on whether it blocks merge under the motion's "complete" standard.

### Concessions

- **CRITIC-1** conceded to **ADVOCATE-1**: Defect 1 (localhost fast-fail eliminates the silent-hang scenario); Defect 2 (the 409 dead-end fails the pre-existence test — it predates C-New-1 and the fix is strictly additive).
- **ADVOCATE-1** conceded to **CRITIC-1**: The "try again" recovery path was incorrectly framed — the error CTA at `SetupStep4Confirm.svelte:104` routes to Step 1 via `onGoToStep(1)`, not a simple retry. The re-traversal loop is tighter and more severe than initially argued. (Moot after CRITIC conceded Defect 2.)
- **ADVOCATE-1** acknowledged: The test coverage gap is real and worth a follow-up.

### Regression Lineage

The sixth council's two conditions (C-New-1 and C-New-2) were applied in commit `4341224`. No regressions were identified from these fixes. The C-New-1 abort-path poll at `api.ts:184-193` does not introduce new failure modes beyond the pre-existing timeout behavior. The C-New-2 JSON parsing at `api.ts:157-166` and `api.ts:198-207` is additive error extraction that cannot degrade existing behavior. All prior council findings (councils 1-5) remain resolved and verified in source.

### Arbiter Recommendation

**FOR**

Both sixth council conditions are implemented correctly and verified against source. C-New-2 JSON error parsing matches the `apiFetch` pattern in both `setupTestConnection` and `setupSaveConfig`. C-New-1 polls `/api/status` on abort and reloads on success, matching the sixth council's stated requirement. The CRITIC's two substantive code defects (Defects 1 and 2) were withdrawn after honest self-correction — Defect 1 fails on the localhost deployment target, and Defect 2 fails the pre-existence test. The sole surviving objection — zero test coverage for the new functions — is factually correct but does not constitute a blocking defect: six prior council sessions reviewed this PR's code, including the functions in question, and none required test coverage as a condition. The accumulated council process is the governing review standard for this PR, and retroactively adding a requirement that six reviews declined to impose would undermine the authority of that process. The test gap is a legitimate improvement opportunity but does not rise to a correctness, security, or data-integrity concern that would warrant blocking merge after seven rounds of adversarial review.

### Conditions

None.

### Suggested Fixes

No issues identified.

The test coverage gap for `setupTestConnection` and `setupSaveConfig` is acknowledged by both sides as real. It is not included as a fix because: (1) it is not a code defect — it is an absence of tests; (2) six prior councils declined to require it; (3) no Critical Discovery threshold (security, data loss, compliance) is met. The gap is noted here for the record should the project choose to address it.
