---
## Adversarial Council -- Merge PR #32: epic: End-to-End QA

> Convened: 2026-04-12T21:35:00Z | Advocates: 1 | Critics: 2 | Rounds: 3/4 | Motion type: CODE

### Motion
Merge PR #32: epic: End-to-End QA

---

### Advocate Positions

**ADVOCATE-1** (FOR):

1. **Coverage is comprehensive and structurally sound.** Five spec files cover every major user flow: data-grid loading/sorting/filtering/search/pagination/cell formatting; search panel, column visibility, CSV export; default load, table switching, sidebar search, sidebar collapse; API errors, SQL injection, edge cases; and the full 4-step setup wizard. Each spec tests actual observable behaviour with explicit DOM assertions.

2. **Infrastructure handles real-world hazards correctly.** `global-setup.ts:57-61` checks port availability before binding. Lines 98-104 back up the user's existing `seeki.toml` before overwriting. Lines 70-83 load `.env.test` and expand env vars — no credentials hardcoded in tracked files (`seeki.toml.test:11`). Teardown at `global-teardown.ts:42-48` restores the original config unconditionally.

3. **Selectors are grounded in actual DOM.** Every CSS selector in fixtures is verified against `DataGrid.svelte` — `.sk-grid-header__label`, `.sk-grid-cell--null`, `.sk-grid-badge`, `.sk-grid-cell--number` all exist in the component's light DOM output.

4. **SQL injection tests are substantive.** `error-states.spec.ts:59-85` sends the injection payload through the filter input and then queries `/api/tables` to verify the table still exists. `error-states.spec.ts:89-118` asserts the returned row count is ≤ the pre-injection count — a genuine semantic check that OR-expansion did not occur. Both backed by `postgres.rs:268-316` parameterized binds.

5. **Numeric null fix is correct.** `postgres.rs:461-530` returns `Value::Null` via `.unwrap_or(Value::Null)` on every `try_get` failure for all numeric types, correctly mapping nullable columns.

6. **Wizard mocking is an architectural necessity.** The server boots in `normal` mode because `seeki.toml.test` exists. `AppMode::Setup` requires no config file. Testing the wizard against a normal-mode server is architecturally impossible without intercepting `/api/status`. Backend endpoint integration for `setup/test-connection` and `setup/save` belongs in unit/API-layer tests, not wizard UI specs.

7. **Absent CI wiring is normal sequencing.** Every test suite against a live external database requires out-of-band credential provisioning. `seeki.toml.test` documents the expected env var names. A CI workflow operator knows exactly what secrets to inject. Merging test infrastructure first and adding CI wiring in a subsequent PR is standard practice.

---

### Critic Positions

**CRITIC-1** (CONDITIONAL):

1. **F3 — `pendingRowsResponse()` accepts HTTP 4xx as success.** `fixtures.ts:28`: `resp.status() < 500`. ADVOCATE's "intentional dual use" argument does not hold — the backend's parameterized query layer returns 200 (not 400) even on injection payloads. Future validation changes could cause filter tests to silently proceed on error-state UI. Fix: `resp.ok()`.
   CITE: `frontend/e2e/fixtures.ts` L:28

2. **F4 — `vehicle_logs` hardcoded in injection test assertion.** `error-states.spec.ts:85`: `expect(tableNames).toContain('vehicle_logs')` fails unconditionally in any environment without that specific table — not because injection succeeded, but because the assertion is wrong. The URL injection test at lines 129-130 already demonstrates the correct pattern of fetching a table name dynamically.
   CITE: `frontend/e2e/error-states.spec.ts` L:85

3. **F6 — `waitForGridLoaded` permanently times out on empty tables.** `fixtures.ts:44-47`: `!statusBar.textContent?.includes('Showing 0')` returns false for "Showing 0 - 0 of 0", spinning 15 seconds before failing. Shipping broken assertions trains contributors to treat red CI as "probably a test issue."
   CITE: `frontend/e2e/fixtures.ts` L:44-47

4. **F1 — Double release build on every `test-e2e` run.** `Justfile:29-33` calls `cargo build --release`; then `global-setup.ts:109` calls it again unless `SEEKI_SKIP_BUILD=1` is passed. Accepted as follow-up (one-line fix), not a merge blocker.
   CITE: `frontend/e2e/global-setup.ts` L:107-114

5. **F2 / F5 — Wizard mocking, Chrome-only.** Both softened to tracked gaps. Architectural constraints acknowledged for F2; Chrome-first is standard for initial E2E suites.

**CRITIC-2** (AGAINST → converged to CONDITIONAL):

1. **Finding A — Tautological filter/search assertions.** `toolbar.spec.ts:59`: `expect(filteredTotal).toBeGreaterThanOrEqual(0)` — mathematically vacuous, passes on any return value. `data-grid.spec.ts:167`: same. A backend that completely ignores filter/search parameters passes both assertions silently. ADVOCATE conceded the `>=0` vacuity and that `<=initialTotal` is weaker than it should be.
   CITE: `frontend/e2e/toolbar.spec.ts` L:59 | `frontend/e2e/data-grid.spec.ts` L:167

2. **Finding B — Sort test verifies UI indicator only; never validates sorted data.** `data-grid.spec.ts:48-68` has no `pendingRowsResponse()` call and makes no assertion on row content. A backend silently ignoring `sort_column` passes this test. ADVOCATE acknowledged the gap but framed it as an enhancement (UI state machine testing is valid in isolation).
   CITE: `frontend/e2e/data-grid.spec.ts` L:48-68

3. **Finding C — No CI integration path.** No `.github/workflows/` file. `global-setup.ts:87-94` silently degrades to `postgres://:@::/` on a clean clone. Tests exist in the repo but cannot run automatically. ADVOCATE rebuts this as normal sequencing; critics maintain that "automated E2E coverage" in the PR title requires a path to automation.
   CITE: `frontend/e2e/global-setup.ts` L:87-94

4. **Finding D — Config crash recovery requires manual step.** `global-setup.ts:99-103` has no SIGTERM handler; a SIGKILL leaves `seeki.toml` pointing to the test database. Data is not lost (`seeki.toml.user-backup` survives) but recovery is manual. Downgraded from blocker to documentation gap.
   CITE: `frontend/e2e/global-setup.ts` L:99-103

---

### Questioner Findings

QUESTIONER was stalled throughout — no probes delivered on record. Arbiter conducted independent verification of key claims:

- **Credential safety**: SUBSTANTIATED. `seeki.toml.test:11` uses `${SEEKI_TEST_DB_*}` placeholders. `.env.test` gitignored at `.gitignore:39`. `seeki.toml` gitignored at `.gitignore:24`.
- **Identifier validation allows hyphens/spaces**: UNSUBSTANTIATED AS SECURITY CONCERN. `postgres.rs:235-237` allows `-` and ` `, but all interpolated identifiers are wrapped in double-quotes (verified at L:307, L:313, L:337, L:369, L:379). Safe per SQL standard.
- **`waitForGridLoaded` empty-table hang**: SUBSTANTIATED. `fixtures.ts:44-47` confirmed.
- **F7 localStorage bleed**: UNSUBSTANTIATED. Playwright's `page` fixture creates an isolated BrowserContext per test. localStorage does not persist between test functions. Both CRITIC-1 and CRITIC-2 conceded this point after ADVOCATE-1's rebuttal.
- **`test.skip()` inside test body**: UNSUBSTANTIATED AS ANTI-PATTERN. Both `test.skip(condition, reason)` (`navigation.spec.ts:55`) and `if (cond) { test.skip(); return; }` (`data-grid.spec.ts:113-117`) are valid Playwright APIs.

---

### Key Conflicts

- **F3 (`status() < 500`)** — ADVOCATE: intentional dual use for injection tests; CRITIC-1: parameterized queries return 200 not 400 on injection, so the dual-use rationale is unfounded. *Unresolved: CRITIC-1's technical rebuttal stands; ADVOCATE did not counter the 200-not-400 point.*
- **Finding C (no CI path)** — ADVOCATE: normal sequencing, credentials pattern is standard; both critics: "automated coverage" claim requires a path to automation. *Unresolved: scope/framing dispute.*
- **Finding B (sort test is UI-only)** — ADVOCATE: UI state machine testing is valid and valuable in isolation; CRITIC-2: data-order correctness is the user-visible behavior. *Partially resolved: ADVOCATE acknowledged the gap while maintaining the test has value.*
- **Finding A (`>=0` tautology vs. pipeline coverage)** — Both sides agree `>=0` is vacuous. Dispute is whether the surrounding test still exercises the filter pipeline meaningfully. *Partially resolved: ADVOCATE conceded `>=0` and `<=` weakness; CRITIC-2 maintains untestable assertions are structurally misleading.*

---

### Concessions

- **CRITIC-1** conceded F7 (localStorage bleed) — Playwright's BrowserContext isolation makes this a non-issue.
- **CRITIC-1** conceded F2 (wizard mocking) — downgraded from blocker to tracked gap; architectural constraint acknowledged.
- **CRITIC-1** conceded F5 (Chrome-only) — downgraded to tracked gap.
- **CRITIC-1** conceded F1 (double build) — follow-up acceptable, not a merge blocker.
- **CRITIC-2** conceded F7 — independently validated ADVOCATE-1's Playwright isolation argument.
- **CRITIC-2** conceded Finding D severity — downgraded from blocker to documentation gap; backup file preserves data.
- **ADVOCATE-1** conceded F4 (`vehicle_logs` assertion is environment-dependent).
- **ADVOCATE-1** conceded F6 (`waitForGridLoaded` logic defect for empty tables).
- **ADVOCATE-1** conceded Finding A (`>=0` assertions are vacuous; `<=` is weaker than needed).
- **ADVOCATE-1** conceded Finding B (sort test does not verify data order end-to-end).
- **ADVOCATE-1** conceded F3 (latent imprecision — did not counter CRITIC-1's 200-not-400 rebuttal).

---

### Regression Lineage
No regression lineage — no prior fix involvement.

---

### Arbiter Recommendation

**CONDITIONAL**

The test infrastructure is well-conceived: config backup/restore is robust, credentials are properly separated, Playwright wait strategies are correct, and the security injection tests are substantively meaningful. The backend numeric null fix is correct. However, five code-level defects were conceded by ADVOCATE-1 and substantiated against the actual source: the `vehicle_logs` hardcoded assertion fails on any non-MEC-Miki environment (`error-states.spec.ts:85`); the `waitForGridLoaded` condition times out on empty tables (`fixtures.ts:44-47`); `pendingRowsResponse` accepts 4xx as a success signal (`fixtures.ts:28`); the `>=0` filter assertions are mathematically vacuous (`toolbar.spec.ts:59`, `data-grid.spec.ts:167`); and the sort test verifies only the UI state indicator, never sorted data (`data-grid.spec.ts:48-68`). Fix these five before merge; they are all one-to-five-line corrections.

Finding C (no CI integration path) is a known limitation — documented as a tracked gap, not a merge blocker. The test suite is locally runnable and correctly designed; CI wiring can follow in a subsequent PR.

---

### Conditions

1. Fix `pendingRowsResponse()` condition — `frontend/e2e/fixtures.ts:28` — change `resp.status() < 500` to `resp.ok()`.
2. Fix `waitForGridLoaded` empty-table condition — `frontend/e2e/fixtures.ts:44-47` — resolve when status bar has any non-empty text, not only when it doesn't contain "Showing 0".
3. Fix hardcoded `vehicle_logs` assertion — `frontend/e2e/error-states.spec.ts:85` — use a dynamically fetched table name (pattern already present at L:129-130 in the same file).
4. Fix tautological `>=0` assertions — `frontend/e2e/toolbar.spec.ts:59` and `frontend/e2e/data-grid.spec.ts:167` — remove or replace with a meaningful bound (e.g., `expect(filteredTotal).toBeLessThan(initialTotal)`).
5. Strengthen sort test — `frontend/e2e/data-grid.spec.ts:48-68` — add a `pendingRowsResponse()` wait and at least one assertion verifying that a row count change (or stable count with different ordering) occurred after the sort click.

---

### Suggested Fixes

#### Fixes (all in-PR)

- **`pendingRowsResponse` accepts 4xx as loaded** — change `resp.status() < 500` to `resp.ok()` — `frontend/e2e/fixtures.ts` L:28 — MEDIUM — future validation changes could silently proceed on error-state UI; parameterized backends return 200 not 400, so the "intentional dual use" rationale was rebutted.
  CITE: `frontend/e2e/fixtures.ts` L:28

- **`waitForGridLoaded` times out on empty tables** — change condition to resolve when status bar has any non-empty text — `frontend/e2e/fixtures.ts` L:44-47 — MEDIUM — the condition `!statusBar.textContent?.includes('Showing 0')` is false for "Showing 0 - 0 of 0", causing 15s timeout on any empty table.
  CITE: `frontend/e2e/fixtures.ts` L:44-47
  ```ts
  // Before (L44-47):
  return statusBar && !statusBar.textContent?.includes('Showing 0');
  // After:
  return statusBar !== null && (statusBar.textContent?.trim() ?? '') !== '';
  ```

- **Hardcoded `vehicle_logs` assertion in injection test** — replace `expect(tableNames).toContain('vehicle_logs')` with assertion against the dynamically fetched table name — `frontend/e2e/error-states.spec.ts` L:85 — HIGH — fails unconditionally on any database without a table named `vehicle_logs`, producing misleading test failures unrelated to injection success.
  CITE: `frontend/e2e/error-states.spec.ts` L:85

- **Tautological `>=0` filter/search assertions** — replace `toBeGreaterThanOrEqual(0)` with `toBeLessThan(initialTotal)` or remove — `frontend/e2e/toolbar.spec.ts` L:59 and `frontend/e2e/data-grid.spec.ts` L:167 — MEDIUM — mathematically vacuous; passes even if filter/search returns the full unfiltered dataset.
  CITE: `frontend/e2e/toolbar.spec.ts` L:59 | `frontend/e2e/data-grid.spec.ts` L:167

- **Sort test verifies only ARIA label, not sorted data** — add `pendingRowsResponse()` call before sort click and at minimum assert row count stability — `frontend/e2e/data-grid.spec.ts` L:48-68 — LOW — backend ignoring `sort_column` entirely would pass the current test; ADVOCATE acknowledged this gap.
  CITE: `frontend/e2e/data-grid.spec.ts` L:48-68

#### Tracked Gaps (post-merge follow-ups)

- **F1: Double release build** — `Justfile:29-33` + `global-setup.ts:109` — pass `SEEKI_SKIP_BUILD=1` to Playwright in the Justfile recipe. One-line fix; wastes CI time but does not corrupt results.
- **Finding C: No CI integration path** — add `.github/workflows/e2e.yml` with secrets injection for `SEEKI_TEST_DB_*`. Until CI exists, the "automated E2E coverage" framing is aspirational.
- **Finding D: Config crash recovery** — add `process.on('SIGTERM', ...)` and `process.on('SIGINT', ...)` handlers in `global-setup.ts` that invoke teardown logic. Data is not lost on crash (backup survives), but recovery requires a manual `mv`.
- **F5: Chrome-only coverage** — add WebKit/Firefox projects to `playwright.config.ts` in a follow-on PR.
