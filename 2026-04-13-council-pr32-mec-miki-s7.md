# Council Session 7 — PR #32 (MEC-Miki E2E)

**Motion:** Merge PR #32 — E2E suite works against MEC-Miki data and emits visible warnings for skipped coverage.

**Verdict:** FOR

## Scope

Session 6 concluded FOR with two follow-up silent-skip sites (data-grid.spec.ts:151, :220). Commit 81d657b patched both. This session verifies the patches and sweeps for any remaining bare skips.

## Evidence

### Gap 2 follow-ups — PATCHED

**data-grid.spec.ts:151-153** (multi-filter AND coverage):
```
console.warn('[e2e] Multi-filter AND coverage skipped: loaded table has fewer than 2 filterable columns. If running against a real DB (SEEKI_SKIP_SEED=1), point at a table with 2+ columns to exercise this assertion.');
test.skip();
return;
```
`console.warn` precedes `test.skip()`. Confirmed.

**data-grid.spec.ts:221-223** (pagination coverage):
```
console.warn('[e2e] Pagination coverage skipped: loaded table has 50 or fewer rows. If running against a real DB (SEEKI_SKIP_SEED=1), point at a table with >50 rows to exercise this assertion.');
test.skip();
return;
```
`console.warn` precedes `test.skip()`. Confirmed.

### Full skip-site inventory (grep `test\.skip()` across `frontend/e2e/`)

| Line | Test | Warn line | Status |
|------|------|-----------|--------|
| 152  | multi-filter AND | 151 | Covered |
| 222  | pagination forward/back | 221 | Covered |
| 292  | NULL cell styling | 291 | Covered |
| 305  | boolean Yes/No badge | 304 | Covered |
| 322  | numeric right-align | 321 | Covered |

All 5 bare `test.skip()` sites are preceded by a `console.warn` using the consistent `[e2e]` prefix and SEEKI_SKIP_SEED guidance. No silent skips remain.

## Synthesis

- **ADVOCATE-1 (synthesized):** Patches match the Session 6 CONDITIONAL. Warning format is consistent across all 5 skip sites, includes actionable operator guidance (SEEKI_SKIP_SEED=1 + what table shape to point at). Merge.
- **CRITIC-1 (synthesized):** No regressions introduced; the patches are pure additions above existing `test.skip()` calls. Sites 292/305/322 were already compliant from prior sessions — Session 6 only flagged 151/220 as the delta, and the delta is closed. No new findings.

## Findings

No issues identified.

## Recommendation

**FOR — merge PR #32.** Gap 2 is closed; the E2E suite now emits visible, actionable warnings at every skip site.
