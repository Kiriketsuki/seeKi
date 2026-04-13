# Fix Manifest — PR #32 Council Session 3 (MEC-Miki Validation)

Council verdict: **CONDITIONAL** | 2026-04-12 22:15 UTC | 3 findings (all verified) + 1 Critical Discovery

## Fixes Required (Merge Blockers)

### 1. Apply seed.sql by the harness (CI reproducibility)
- **File**: `frontend/e2e/global-setup.ts`
- **Line**: insert after line 149 (`await waitForHealthy()`)
- **Severity**: HIGH
- **Verification**: VERIFIED — global-setup.ts:55-153 contains no SQL execution
- **Fix**: Add a seed application step so the "ALL PASSING" claim is reproducible on a fresh checkout.

```typescript
// After await waitForHealthy() at line 149:
const seedPath = path.join(PROJECT_ROOT, 'tests/fixtures/seed.sql');
if (fs.existsSync(seedPath)) {
  console.log('[global-setup] Applying seed.sql...');
  const connectUrl = `postgres://${process.env.SEEKI_TEST_DB_USER}:${process.env.SEEKI_TEST_DB_PASSWORD}@${process.env.SEEKI_TEST_DB_HOST}:${process.env.SEEKI_TEST_DB_PORT ?? '5432'}/${process.env.SEEKI_TEST_DB_NAME}`;
  execSync(`psql "${connectUrl}" -f "${seedPath}"`, {
    cwd: PROJECT_ROOT,
    stdio: 'inherit',
    timeout: 30_000,
  });
}
```
- **Citations**: `frontend/e2e/global-setup.ts:55-153`

### 2. Add filter lower-bound assertion
- **File**: `frontend/e2e/data-grid.spec.ts`
- **Line**: 131 (insert new line before)
- **Severity**: MEDIUM
- **Verification**: VERIFIED
- **Fix**: Add `expect(filteredTotal).toBeGreaterThan(0)` before the existing `toBeLessThanOrEqual` assertion.

```typescript
const filteredTotal = await seeki.getTotalRows();
expect(filteredTotal).toBeGreaterThan(0);          // NEW
expect(filteredTotal).toBeLessThanOrEqual(initialTotal);
```
- **Citations**: `frontend/e2e/data-grid.spec.ts:131`

## PR Description Amendment

Recharacterize the suite: "Generic behavioral regression harness. When run against MEC-Miki it additionally exercises boolean badge rendering, NULL cell styling, two-table navigation, and sort data-order. Does NOT assert MEC-Miki schema properties — schema drift (column rename / type change / timestamp format) is outside its regression surface."

## Critical Discovery — CD-1 (Informational, Not Merge Blocker)

### Config backup overwritten on interrupted retry
- **File**: `frontend/e2e/global-setup.ts:98-103`
- **Category**: Data Loss
- **Mechanism**: Run #1 killed after line 103 leaves test config in `seeki.toml`. Run #2 line 99 then copies that test config over `seeki.toml.user-backup`, destroying the user's original.
- **Recommended fix**:

```typescript
if (!fs.existsSync(CONFIG_BACKUP) ||
    fs.readFileSync(CONFIG_BACKUP, 'utf-8') === fs.readFileSync(CONFIG_DST, 'utf-8')) {
  fs.copyFileSync(CONFIG_DST, CONFIG_BACKUP);
}
```

## Test Command
`just test-e2e`

## Raw Data
- Full recommendation: `2026-04-12-221500-council-pr32-mec-miki-session3.md`
- Structured result: `.council/council-result.json`
