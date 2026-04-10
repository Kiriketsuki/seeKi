# Fix Manifest — PR #30: Toolbar, Column Management & Export (Round 2 post-fix audit)

Council verdict: **CONDITIONAL** | 2026-04-10 | 2 findings (2 verified) | Rounds: 1/4 | Questioner: disabled
Prior council: `.council/2026-04-10-council-recommendation-round1.md` (round 1 produced 4 fixes D1, D2, D3, C6 applied in commit 5a9e382)

All 4 round-1 fixes verified correctly applied. Round 2 identified 2 new defects that round 1 missed.

## Round-1 Fix Audit (commit 5a9e382)

| Fix | File:Line | Verification |
|-----|-----------|--------------|
| D1 — `role="list"` removed | `ColumnDropdown.svelte:46` | ✅ verified |
| D2 — `aria-pressed` added | `ColumnDropdown.svelte:50` | ✅ verified (round-1 cited phantom `role="checkbox"` — never existed; improvement stands) |
| D3 — webkit cancel CSS | `App.svelte:629-633` | ✅ verified |
| C6 — try/catch setItem | `App.svelte:203-210` | ✅ verified |

## Fixes Required (round 2, all in-PR, all verified)

### R2-1. `role="dialog"` without focus management (WAI-ARIA 1.2 § 6.6 violation)
- **Files**:
  - `frontend/src/components/ColumnDropdown.svelte` L32
  - `frontend/src/components/Toolbar.svelte` L129
- **Type**: bug
- **Severity**: high
- **Verification**: verified
- **Current code**:
  - `ColumnDropdown.svelte:32`: `<div class="dropdown" bind:this={panel} role="dialog" aria-label="Column visibility">`
  - `Toolbar.svelte:129`: `aria-haspopup="dialog"`
- **Fix**: Swap to Disclosure pattern.
  - `ColumnDropdown.svelte:32`: remove `role="dialog"`, `aria-label="Column visibility"`, and `bind:this={panel}` → `<div class="dropdown">`
  - `Toolbar.svelte:129`: remove `aria-haspopup="dialog"` line (the trigger already has `aria-expanded={columnsOpen}`)
- **Why**: `role="dialog"` mandates (1) focus-on-open, (2) Tab containment, (3) focus-return-on-close. None of these are implemented: no `tick()` + `.focus()` anywhere when `columnsOpen` becomes true, no focus trap in ColumnDropdown, and Escape/outside-click close without restoring focus. The trigger's `aria-haspopup="dialog"` makes an AT promise the implementation doesn't keep. The panel is a non-modal Disclosure widget (users can Tab out naturally, `aria-expanded` already signals state) so the Disclosure pattern is the correct semantic fit and has no focus-trap requirement. ~4 lines total.

### R2-2. Dead `panel` binding left behind by fix commit
- **File**: `frontend/src/components/ColumnDropdown.svelte`
- **Lines**: 17 (declaration), 32 (`bind:this`)
- **Type**: improvement
- **Severity**: low
- **Verification**: verified
- **Current code**:
  - L17: `let panel: HTMLDivElement | null = null;`
  - L32: `bind:this={panel}`
- **Fix**: Delete both the declaration at L17 and the `bind:this={panel}` from the outer div at L32. (Combined with R2-1 above — L32 collapses to `<div class="dropdown">`.)
- **Why**: `panel` is written by `bind:this` and never read anywhere in the script block. The `onMount` block that could have consumed it was removed in commit 5a9e382. Dead code, trivial removal, no behavior change.

## Conditions (from arbiter)
- Apply Fixes R2-1 and R2-2 on this branch before merging.
- Verify in a screen reader or Lighthouse accessibility audit that the ColumnDropdown announces as a region/disclosure (not a dialog) and no "dialog opened but focus did not move" error is reported.

## D2 Phantom (informational)

Round-1 D2 cited `<button role="checkbox" aria-checked={isVisible(column)}>` at `ColumnDropdown.svelte:48-50`. CRITIC-1 verified via `git show 0222ab7` and `git show a1b92c8` that those attributes were never present in any commit — the buttons were plain `<button type="button">` elements with no ARIA state. The fix commit added `aria-pressed={isVisible(column)}` to buttons that had no prior role, which is a valid improvement but was not repairing the described defect. On the record to avoid future confusion.

## Test Command
```
cd frontend && npm run check && npm run build
```
(No frontend test suite — svelte-check + production build is the available gate.)

## Raw Data
- Round 2 recommendation: `.council/council-recommendation.md`
- Round 2 structured result: `.council/council-result.json`
- Round 1 archived recommendation: `.council/2026-04-10-council-recommendation-round1.md`
