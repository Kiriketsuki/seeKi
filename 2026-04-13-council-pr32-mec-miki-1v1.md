# Adversarial Council — PR #32 MEC-Miki Validation (1v1)

> Convened: 2026-04-13 | Advocates: 1 | Critics: 1 | Rounds: 1/4 (converged early — prior-session remediation verified) | Motion type: CODE | Session: 5 (MEC-Miki focus)

## Motion

Merge PR #32 (epic: End-to-End QA). The Playwright E2E suite actually works against MEC-Miki data — both the synthetic `seed.sql` fixture (which mirrors the MEC-Miki schema) AND a real MEC-Miki PostgreSQL instance if pointed at one via `SEEKI_SKIP_SEED=1`. The tests would catch real regressions when run against MEC-Miki data, and the harness does what the PR description claims.

## Advocate Position

**ADVOCATE-1**: The suite satisfies the motion on both paths.

- **Synthetic path (seed.sql):** `tests/fixtures/seed.sql:5-24` defines `vehicle_logs` and `soc_readings` with the MEC-Miki column pattern — VARCHAR `vehicle_id`, NUMERIC `speed_kmh`/`soc_percent`/`voltage_v`, BOOLEAN `is_active`/`is_charging`, TIMESTAMPTZ, and NULL-capable columns. 200 + 80 rows are sufficient for pagination (>50) and varied-cell assertions.
- **Real MEC-Miki path:** `frontend/e2e/global-setup.ts:168` honors `SEEKI_SKIP_SEED=1`; when set, setup skips psql invocation entirely and leaves the DB untouched. The Rust binary then connects to whatever DB `.env.test` points at.
- **Schema-agnostic specs:** `frontend/e2e/data-grid.spec.ts` has zero references to `vehicle_logs`, `soc_readings`, `VH-`, or MEC-Miki specifics (verified by Grep). Every assertion is generic: `toBeGreaterThan(0)` on row count, header count > 0, sort changes first cell, filter `'1'` narrows (with proper-subset assertion at L134-135), pagination gates on `> 50` rows (L219), NULL/boolean/numeric styling tests `test.skip()` if no such cells (L288, L300, L316).
- **Regressions would be caught:** If a backend change broke numeric rendering, NULL badges, sort ordering, or pagination math against MEC-Miki data, the suite would surface the regression because the assertions target DOM/CSS behavior independent of the underlying table names.

## Critic Position

**CRITIC-1**: The suite works as claimed for the common case, but has one MEC-Miki-specific edge that the motion language overstates.

- **Filter test first-column coupling:** `data-grid.spec.ts:134-135` asserts `filteredTotal < initialTotal` after filtering the first column by the string `'1'`. Against a real MEC-Miki table whose first column (alphabetically, post-`ORDER BY column_ordinal`) happens to contain `'1'` in every row — e.g. a `version` or `schema_rev` column populated uniformly — the assertion `toBeLessThan(initialTotal)` would fail. This is not a defect, but the motion claim "would catch real regressions when run against MEC-Miki data" carries an asterisk: false positives are possible on pathological first-column distributions. Severity: LOW. Already accepted as a trade-off in session 3 (`toBeLessThan` is deliberate to catch silent-pass zero-match bugs).
- **All other MEC-Miki paths:** no objection. The seed mirrors the schema correctly, `SEEKI_SKIP_SEED=1` guard is clean, and specs are schema-agnostic.

## Key Conflicts

- **Filter assertion strictness vs. real-DB false-positive risk** — ADV: "proper-subset assertion is the correct bug-catcher"; CRIT: "motion language overstates universality". **Resolved**: the risk is real but narrow, and the trade-off (stronger assertion vs. first-column coupling) was already litigated and accepted in session 3. Not a MEC-Miki blocker.

## Concessions

- CRITIC-1: concedes the synthetic path (seed.sql) and the real-DB path (`SEEKI_SKIP_SEED=1`) both function as claimed. Concedes the schema-agnostic design of the spec assertions. The one objection is a known trade-off, not a motion-defeating defect.
- ADVOCATE-1: acknowledges the filter-test first-column coupling as a legitimate edge case against some MEC-Miki tables; suite cannot claim 100% universality without per-table configuration.

## Independent Verification (Arbiter)

| Claim | Source | Verified |
|---|---|---|
| seed.sql mirrors MEC-Miki columns | `tests/fixtures/seed.sql:5-24` | Yes |
| Spec files contain no MEC-Miki hardcoding | Grep for `vehicle_logs\|soc_readings\|MEC\|Miki\|VH-` in `frontend/e2e/` | Confirmed: zero matches |
| `SEEKI_SKIP_SEED=1` cleanly bypasses seed | `frontend/e2e/global-setup.ts:168` | Yes — condition gates entire psql block |
| Credential guard covers all 4 env vars | `frontend/e2e/global-setup.ts:174-178` | Yes — `user`, `host`, `name`, `pass` all checked |
| Session 4 Condition 1 (CD-1 fix) applied | `frontend/e2e/global-setup.ts:106-113` | Yes — uses `dstContent !== configContent` |
| Session 4 Condition 2 (Justfile warning) applied | `Justfile:29-34` | Yes — WARNING comment present |
| Pagination test gates on seed row count | `frontend/e2e/data-grid.spec.ts:219` | Yes — `if (totalRows <= 50) test.skip()` |
| Filter test has lower + upper bound | `frontend/e2e/data-grid.spec.ts:134-135` | Yes — `toBeGreaterThan(0)` and `toBeLessThan(initialTotal)` |

All claims verified against source at the cited lines.

## Arbiter Recommendation

**FOR**

The E2E suite satisfies the motion as stated. The synthetic `seed.sql` fixture accurately mirrors the MEC-Miki schema shape; the `SEEKI_SKIP_SEED=1` escape hatch cleanly supports real MEC-Miki DB runs; and the spec assertions are schema-agnostic, meaning they target generic UI behavior that would fail on real regressions regardless of whether the data comes from seed or a live instance. All prior-session blockers (CD-1 Bug 1, O-1 Justfile warning) were remediated in commits d72decb and bbda9c1 and verified against source. The one remaining edge case (filter test first-column coupling on pathological real-DB distributions) is a known trade-off accepted in session 3 and does not defeat the motion.

## Suggested Fixes

No issues identified.

## Critical Discoveries (informational)

None. No Security / Data Loss / Compliance issues specific to MEC-Miki compatibility were substantiated.

## PR Description Amendments

Optional, non-blocking: the PR description could note the filter-test first-column caveat — "if pointed at a real DB whose first-sorted column is uniformly populated, the filter test may report a false positive; prefer a dedicated test DB or the synthetic seed for reproducible runs." This is a documentation nicety, not a merge condition.

---

*Arbiter: Claude Opus 4.6 — Session 5 of 5 — 2026-04-13*
*Prior sessions: 2026-04-12 (sessions 1–4)*
