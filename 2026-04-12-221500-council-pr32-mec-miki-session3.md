# Adversarial Council Result — PR #32 (MEC-Miki Session 3)

**Motion:** Merge PR #32 (epic: End-to-End QA) — verify the Playwright E2E suite genuinely validates SeeKi against the real MEC-Miki PostgreSQL database (vehicle_logs, soc_readings). Focus: (1) real DB coupling, (2) catches MEC-Miki-specific regressions, (3) tests that pass on empty/wrong DB, (4) seed.sql vs real MEC-Miki divergence.

**Classification:** CODE

**Session:** 3 (prior sessions: 2026-04-12 session 1, 2026-04-13 session 2 — 7 prior findings, all fixed)

**Date:** 2026-04-12 22:15 UTC (session 3 start)

**Verdict:** CONDITIONAL

---

## Participants

| Role | Filed |
|---|---|
| ADVOCATE-1 | Round 1 (opening), Round 2 (rebuttal to CRITIC-1), Round 3 (final, filed after debate called) |
| CRITIC-1 | Round 1, Round 2, Round 3 |
| CRITIC-2 | Round 1, Round 2, Round 3 |
| QUESTIONER | Did not file any round. No claims were marked unsubstantiated. All cited claims evaluated on their own merits. |

---

## Advocate Positions

**ADVOCATE-1** opened FOR the motion, citing:
- Empty DB (0 rows, 0 tables) produces hard failures: `data-grid.spec.ts:17` (`toBeGreaterThan(0)`) and `error-states.spec.ts:53` (`tables.length > 0`).
- Unreachable/bad credentials: `main.rs:37` exits with error; `global-setup.ts:37-53` throws after 30s, aborting the run.
- `seed.sql` faithfully mirrors the MEC-Miki schema (vehicle_logs, soc_readings — column names, types, and data distribution).
- Skip guards (NULL/bool/numeric formatting) cannot fire given seed data (200 rows with NULLs every 7th row, booleans on every row, two tables).
- Filter baseline captured pre-filter; sort conditional entered for MEC-Miki's `id SERIAL` (asc first = 1, desc first = 200).

**Final ADVOCATE-1 position (Round 3):** Conceded seed.sql CI gap, Gap 3 data-loss path, Gap 4 reachable-wrong-DB gap, and filter lower-bound weakness. Maintained FOR: the suite exercises boolean/NULL rendering, sort data-order, and two-table navigation against MEC-Miki runtime characteristics that a generic dummy table would skip over. The "ALL PASSING" Session 2 run was genuine against real MEC-Miki data. CI process gaps should be fixed; the suite itself is a valid behavioral harness.

---

## Critic Positions

**CRITIC-1** argued AGAINST (motion as stated), citing six objections:
- C1: No test names `vehicle_logs`, `soc_readings`, `speed_kmh`, `soc_percent`, or any MEC-Miki column. All table references are positional. A CRM DB produces an identical green suite.
- C2: `toBeGreaterThan(0)` passes on 1 row; pagination skip (not fail) on ≤50 rows.
- C3: NULL/boolean/numeric formatting tests each have `test.skip()` guards that fire silently if those types aren't visible on page 1.
- C4: Filter narrowing assertion (`data-grid.spec.ts:131`) has no lower bound — zero-match result satisfies `0 ≤ initialTotal`.
- C5: Sort conditional at `data-grid.spec.ts:83` can be bypassed if `ascFirst === descFirst`.
- C6: `seed.sql` is not executed by `global-setup.ts`; the Session 2 "ALL PASSING" is a snapshot of a manually-prepared DB run.

**Final CRITIC-1 position:** Conceded C5 for MEC-Miki's `id SERIAL` case. Conceded C4 practical impact (filter does return matches on MEC-Miki data in practice). Narrowed C4 to: assertion is structurally too loose to distinguish working filter from zero-match filter. Held C1 (motion's language of "MEC-Miki-specific regressions" is falsified by ADVOCATE-1's own admission that the suite is generic by design), C3 (soft coverage), C6 (ADVOCATE-1 conceded).

**CRITIC-2** argued AGAINST, citing four gaps:
- Gap 1: `seed.sql` dead artifact — never loaded by `global-setup.ts` (153 lines, no SQL execution).
- Gap 2: All schema-sensitive assertions are intentionally schema-blind — `toBeGreaterThan(0)`, positional table references, no named columns.
- Gap 3: `global-setup.ts:98-103` config backup corrupted on interrupted retry — data-loss path for user's `seeki.toml`.
- Gap 4: Reachable wrong DB produces only a `console.warn` (line 94), not an abort. DB identity never verified anywhere in the harness.

**Final CRITIC-2 position:** Conceded Gap 4a (empty/unreachable credentials abort after 30s timeout — ADVOCATE correct). Conceded C5 for MEC-Miki's serial primary key. Held Gap 1, Gap 2, Gap 3 (uncontested by ADVOCATE across all rounds), Gap 4 (reachable-wrong-DB; confirmed by ADVOCATE-1 Round 3 concession). Noted that ADVOCATE-1's C1 reframing ("suite is generic by design") directly confirms Gap 2.

---

## Questioner Findings

QUESTIONER did not file in any round. No claims were marked "unsubstantiated." All findings below are evaluated solely on the evidence cited by ADVOCATE-1 and the two critics, which was verified against source files by the ARBITER prior to the debate opening.

---

## Key Conflicts and Resolutions

| Conflict | Resolution |
|---|---|
| "Empty DB = silent green" vs. "Empty DB = hard fail" | RESOLVED — CRITIC-1 conceded empty DB fails (`data-grid.spec.ts:17`, `error-states.spec.ts:53`). Revised CRITIC position: concern is **reachable non-empty wrong DB**, not empty DB. |
| "seed.sql guarantees skip guards won't fire" vs. "seed.sql not executed" | RESOLVED — ADVOCATE-1 conceded seed.sql is not auto-applied. Skip guards CAN fire on a fresh checkout or wrong DB. |
| "Suite is MEC-Miki-specific" vs. "Suite is generic by design" | CONVERGED — ADVOCATE-1 Round 3 explicitly states "none of these behavioral properties are specific to vehicle_logs vs accounts." Both sides now agree the suite is a generic behavioral harness that happens to exercise MEC-Miki's runtime characteristics (boolean columns, NULLs, two tables, sortable numeric id) when correctly pointed at it. |
| "Sort conditional bypassed" vs. "Entered for id SERIAL" | RESOLVED — Both CRITIC-1 and CRITIC-2 conceded C5 for MEC-Miki's `id SERIAL` case. Not a finding for MEC-Miki specifically. |
| "Filter assertion catches zero-match" | CONVERGED — ADVOCATE-1 concedes the structural weakness. CRITIC-1 concedes practical impact is limited on MEC-Miki data. Standing as a structural weakness, not a silent-green producer against MEC-Miki. |
| "Bad credentials = abort" | RESOLVED via CRITIC-2 citing `src/db/mod.rs:102-105` and `main.rs:37` — confirmed: binary fails, 30s health timeout, setup throws. Both sides agree on mechanism. |
| "Reachable wrong DB is caught" | RESOLVED — ADVOCATE-1 Round 3 concedes Gap 4: any reachable PostgreSQL with valid credentials passes global-setup. DB identity unverified. |

---

## Concessions (Permanent Record)

**ADVOCATE-1 conceded:**
- F1: `seed.sql` not auto-applied by `global-setup.ts` — genuine CI reproducibility gap
- F3: DB identity unverified — reachable wrong DB produces green suite (Gap 4)
- CD-1: Config backup corrupted on interrupted retry (Gap 3) — data-loss path confirmed
- C3 (partial): NULL/boolean formatting tests are soft coverage contingent on live page-1 data; not harness-enforced
- C4 (partial): Filter assertion has no lower bound (`filteredTotal > 0` absent)

**CRITIC-1 conceded:**
- Empty DB (0 rows, 0 tables) produces hard failures — not silent green
- Unreachable host causes setup abort
- C5 for MEC-Miki case: sort conditional IS entered for `id SERIAL` column
- C4 practical impact: filter does return positive matches against MEC-Miki data in practice

**CRITIC-2 conceded:**
- Empty/bad credentials cause setup abort (binary exits at `main.rs:37`, 30s timeout fires)
- C5 for MEC-Miki case: sort conditional IS entered for serial primary key

---

## Regression Lineage

This is Session 3. Sessions 1 and 2 identified and fixed 7 prior findings (all confirmed fixed per prior context). Session 3 findings do not re-raise any prior item.

---

## Arbiter Recommendation

### Verdict: CONDITIONAL

**Rationale:**

The suite is a genuine behavioral regression harness that was verified against real MEC-Miki PostgreSQL data in Session 2. It fails hard on empty databases and unreachable hosts. Against MEC-Miki's actual runtime characteristics — boolean columns (`is_active`, `is_charging`), nullable numerics (`speed_kmh`, `latitude`, `longitude`), two-table schema, and `id SERIAL` — the suite exercises four test paths that would silently skip against a generic dummy database: boolean badge rendering, NULL cell styling, two-table navigation/sort-reset, and sort data-order verification. These are genuine behavioral regression checks that run against MEC-Miki and not against a substitute.

However, three substantiated gaps prevent characterizing this suite as a reliable MEC-Miki-specific regression net:

**F1** *(CI Reproducibility)* — seed.sql is never applied by the harness. The "ALL PASSING" claim from Session 2 reflects a manually-prepared DB state that is not reproducible on a fresh checkout or clean CI runner. On a fresh instance, NULL/boolean/numeric formatting tests may skip silently, and the two-table navigation test may also skip.

**F2** *(Assertion Strength)* — The filter narrowing assertion (`data-grid.spec.ts:131`) has no lower bound: `filteredTotal > 0` is not asserted. A structurally broken filter that always returns zero results passes the test. Against MEC-Miki data the filter does return matches, but the test cannot prove this.

**F3** *(DB Identity)* — No mechanism in the harness verifies that the configured database is actually MEC-Miki. A developer with `.env.test` pointing to any reachable PostgreSQL database gets an identical green suite. This is a consequence of the suite being generic by design — which is a valid architectural choice — but it means CI green does not imply "ran against MEC-Miki."

These three gaps are fixable. The suite's core architecture is sound and its behavioral coverage is real. Merge CONDITIONAL on fixing F1, F2, and CD-1.

---

## Conditions (Required Before Merge)

### Condition 1 — Seed application in global-setup.ts

Add a step in `global-setup.ts` that executes `tests/fixtures/seed.sql` against the configured database after the server confirms healthy (or before server start, against the DB directly via `psql` or equivalent). This makes the test environment self-contained and reproducible regardless of pre-existing DB state.

**Suggested Fix:**

```
CITE: `frontend/e2e/global-setup.ts` L:149
```

Insert after the `waitForHealthy()` call (or before server spawn, against the DB directly):
```typescript
// Apply seed data to ensure reproducible test state
const { execSync } = await import('child_process');
const seedPath = path.join(PROJECT_ROOT, 'tests/fixtures/seed.sql');
if (fs.existsSync(seedPath)) {
  console.log('[global-setup] Applying seed.sql...');
  // Note: seed uses IF NOT EXISTS — safe to re-apply but may skip inserts on existing data.
  // Consider adding TRUNCATE statements to seed.sql for full reproducibility.
  execSync(
    `psql "${connectUrl}" -f "${seedPath}"`,
    { cwd: PROJECT_ROOT, stdio: 'inherit', timeout: 30_000 }
  );
}
```

Where `connectUrl` is reconstructed from the same env vars used to populate `seeki.toml.test`. Implementation detail is flexible; the requirement is that seed.sql is applied by the harness, not manually.

### Condition 2 — Filter lower bound assertion

Add `expect(filteredTotal).toBeGreaterThan(0)` at `data-grid.spec.ts` after the existing upper-bound assertion, to confirm the filter actually matched rows and did not silently return zero.

**Suggested Fix:**

```
CITE: `frontend/e2e/data-grid.spec.ts` L:131
```

Change:
```typescript
const filteredTotal = await seeki.getTotalRows();
expect(filteredTotal).toBeLessThanOrEqual(initialTotal);
```

To:
```typescript
const filteredTotal = await seeki.getTotalRows();
expect(filteredTotal).toBeGreaterThan(0);
expect(filteredTotal).toBeLessThanOrEqual(initialTotal);
```

Note: This requires that seed data or the live MEC-Miki DB contains at least one row matching `'1'` in the first filterable column. Given seed.sql inserts 200 rows with `id SERIAL` values 1–200, filtering on `id` for `'1'` returns rows 1, 10, 11, 12... etc. The `> 0` assertion will pass.

---

## Critical Discoveries (Informational Only — Not Merge Blockers)

### CD-1 — Config backup overwritten on interrupted retry

**Category:** Data Loss

**Mechanism:** `global-setup.ts:98-103` backs up `seeki.toml` before overwriting it with test config. If Run #1 is interrupted after line 103 (test config written to `seeki.toml`) but before teardown executes, and the user retries (Run #2), line 99 of Run #2 copies the current `seeki.toml` (test config from Run #1) over `seeki.toml.user-backup` — destroying the only copy of the user's original config. Teardown of Run #2 then "restores" the test config as the user's config. The original `seeki.toml` is unrecoverable.

```
CITE: `frontend/e2e/global-setup.ts` L:98-103
```

**Recommended Fix (informational):** Before overwriting `seeki.toml.user-backup`, check whether it already exists AND differs from `CONFIG_DST`. If both exist and differ, the backup is genuine user config; do not overwrite it. If they are identical, the backup is already a test config copy from a prior interrupted run; safe to overwrite.

```typescript
// Safe backup: only write if backup doesn't already exist (true first run)
// or if the existing backup is identical to the current DST (previous test config)
if (!fs.existsSync(CONFIG_BACKUP) ||
    fs.readFileSync(CONFIG_BACKUP, 'utf-8') === fs.readFileSync(CONFIG_DST, 'utf-8')) {
  fs.copyFileSync(CONFIG_DST, CONFIG_BACKUP);
}
```

```
CITE: `frontend/e2e/global-setup.ts` L:98-99
```

This finding is out of scope for the motion (does not affect MEC-Miki regression detection). It is logged as an informational Critical Discovery under the Data Loss threshold.

---

## PR Description Amendments

The PR description should be amended to accurately characterize the suite's validation scope:

**Current implied claim:** suite "validates SeeKi against the real MEC-Miki PostgreSQL database"

**Accurate characterization:** "The E2E suite is a generic behavioral regression harness that exercises SeeKi's rendering, navigation, sorting, filtering, pagination, and SQL injection safety. When run against the MEC-Miki database, it additionally exercises boolean badge rendering, NULL cell styling, two-table navigation, and sort data-order verification — behaviors that skip against a schema-less substitute database. The suite does NOT assert MEC-Miki schema properties (specific table names, column names, or value formats); schema drift is not within its regression surface."

---

## Summary Table

| # | Finding | Category | Severity | Condition |
|---|---|---|---|---|
| F1 | `seed.sql` not applied by harness — CI reproducibility gap | MEC-Miki validation | HIGH | **Merge blocker — Condition 1** |
| F2 | Filter assertion missing lower bound (`filteredTotal > 0` absent) | Assertion strength | MEDIUM | **Merge blocker — Condition 2** |
| F3 | DB identity unverified — reachable wrong DB produces green suite | Design scope | LOW (design choice, informational) | PR description amendment |
| CD-1 | Config backup overwritten on interrupted retry — data-loss path | Data Loss (Critical Discovery) | HIGH (operational) | Recommended fix, not merge blocker |
