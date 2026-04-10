## Adversarial Council -- Merge PR #30: Toolbar, Column Management & Export (Round 2 post-fix audit)

> Convened: 2026-04-10 | Advocates: 1 | Critics: 1 | Rounds: 1/4 | Motion type: CODE | Questioner: DISABLED | Round: 2 (post-fix audit of commit 5a9e382)

### Motion
Merge PR #30: epic: Toolbar, Column Management & Export (adds vertical icon toolbar, global search with 300ms debounce + Ctrl+K, column hide/show with localStorage, and CSV export wiring; +818/-327 across 6 files; closes #4). **Round 2** verifies the 4 required fixes from Round 1 and audits for missed defects.

### Prior Council Reference
Round 1 recommendation: `.council/2026-04-10-council-recommendation-round1.md`. Round 1 produced 4 fixes (D1, D2, D3, C6) applied in commit 5a9e382. This round verifies those fixes and hunts for missed defects.

---

### Round-1 Fix Verification

All 4 fixes verified against HEAD by the ARBITER before debate opened:

| Fix | File | Verification |
|-----|------|-------------|
| D1 — `role="list"` removed | `ColumnDropdown.svelte:46` | ✓ — `<div class="list">` has no role attribute |
| D2 — `aria-pressed` added | `ColumnDropdown.svelte:50` | ✓ — `aria-pressed={isVisible(column)}` present; see D2 Phantom note |
| D3 — webkit cancel button CSS | `App.svelte:629-633` | ✓ — `::-webkit-search-cancel-button` suppressed with `appearance: none` |
| C6 — try/catch on `setItem` | `App.svelte:203-210` | ✓ — write path now symmetric with read-path guard at `App.svelte:188` |

#### D2 Phantom (on the record)

The round-1 D2 finding cited `<button role="checkbox" aria-checked=...>` at `ColumnDropdown.svelte:48-50`. CRITIC-1 raised, and ARBITER verified via `git show 0222ab7` and `git show a1b92c8`, that those attributes were **never present** in the code at any commit. The buttons were plain `<button type="button">` elements with no ARIA state attributes. The fix commit (5a9e382) added `aria-pressed={isVisible(column)}` to buttons that had no prior role or state — a valid improvement, but not a repair of the described defect. Round-1 D2's defect citation was a phantom. The `aria-pressed` addition is correct and stands.

---

### Advocate Positions
**ADVOCATE-1**: All 4 round-1 fixes verified as correctly applied with `file:line` citations. Notes bonus hardening in 5a9e382: `isTextEditingTarget()` guard for keyboard shortcuts (`App.svelte:96-101`), correct Escape priority order (columns → search, `App.svelte:122-132`), and duplicate Escape handler in `ColumnDropdown.svelte` removed (the duplicated `onMount` keydown listener was deleted). Conceded both new CRITIC-1 findings as real. Argued for CONDITIONAL (not rejection) on the grounds that both fix surfaces are surgical — ~4 lines total — equivalent in scale to D1–D3 in round 1.

### Critic Positions
**CRITIC-1**: All 4 round-1 fixes verified as mechanically present. Raised D2 phantom (substantiated by ARBITER). Identified two new defects missed by round 1: (1) `role="dialog"` on the ColumnDropdown outer container (`ColumnDropdown.svelte:32`) without any of the three required dialog keyboard behaviors (WAI-ARIA 1.2 § 6.6: focus-on-open, Tab containment, focus-return-on-close); (2) dead `panel` binding left by the fix commit (`ColumnDropdown.svelte:17, 32`). Withdrew BLOCKING label in favor of CONDITIONAL after ADVOCATE-1 established the fix surface is ~4 lines total — consistent with round-1 conditional treatment.

### Questioner Findings
Questioner disabled for this run.

---

### Key Conflicts
- **D2 Phantom** — CRITIC-1 raised; ADVOCATE-1 accepted; ARBITER verified independently via git history. Confirmed: `role="checkbox"` and `aria-checked` were never present in any commit. Resolved — finding recorded.
- **`role="dialog"` BLOCKING vs CONDITIONAL** — CRITIC-1 initially classified as BLOCKING; ADVOCATE-1 argued for CONDITIONAL using round-1 surgical-fix precedent; CRITIC-1 accepted CONDITIONAL in final exchange. Resolved — CONDITIONAL.

### Concessions
- **ADVOCATE-1** conceded: D2 phantom finding is factually correct; `role="dialog"` focus management gap (`ColumnDropdown.svelte:32`) is a real ARIA defect; dead `panel` binding (`ColumnDropdown.svelte:17, 32`) is real dead code.
- **CRITIC-1** conceded: BLOCKING label withdrawn; CONDITIONAL is the correct treatment; both new fixes are pre-merge viable at ~4 lines total.

---

### Arbiter Recommendation
**CONDITIONAL**

The core PR remains sound: all 4 round-1 fixes are correctly applied, and the PR's core features — debounce coordination, `selectRequestId` race guard, localStorage resilience, and every spec Must-Have — are verified correct and untouched by 5a9e382. Round 2 surfaces two new defects that round 1 missed: (1) `role="dialog"` on the ColumnDropdown outer container is declared without any of the three mandatory dialog keyboard behaviors (WAI-ARIA 1.2 § 6.6 — all MUST), and (2) the `panel` variable is dead code left behind by the fix commit. Both are surgical (~4 lines total) and both agents converged on CONDITIONAL.

### Conditions
1. Apply Fix R2-1 before merging.
2. Apply Fix R2-2 before merging.
3. Verify in a screen reader or Lighthouse accessibility audit that the ColumnDropdown announces correctly as a region/disclosure (not a dialog) and that no "dialog opened but keyboard focus did not move" AT error is reported.

---

### Suggested Fixes

#### Fixes (all in-PR)

- **Fix R2-1** — Replace `role="dialog"` with `role="region"` or remove the role on the ColumnDropdown outer container; update `aria-haspopup="dialog"` on the trigger to `aria-haspopup="true"` or remove it. The column panel is a non-modal Disclosure widget — it does not interrupt workflow, users can Tab out naturally, and the trigger already carries `aria-expanded`. The Disclosure Button pattern (WAI-ARIA APG) has no focus-trap requirement and is the correct semantic fit. Alternatively, keep `role="dialog"` and implement all three required behaviors: `tick()` + `.focus()` on open, Tab containment, and focus-return on close (~15–20 lines). The role-swap path is recommended (~4 lines across 2 files). -- Bug -- `role="dialog"` without focus-on-open, Tab containment, and focus-return breaks keyboard AT navigation: users are told a dialog opened but can never keyboard-navigate into it.
  CITE: `frontend/src/components/ColumnDropdown.svelte` L:32
  CITE: `frontend/src/components/Toolbar.svelte` L:129

- **Fix R2-2** — Remove dead `panel` binding: delete `let panel: HTMLDivElement | null = null;` at `ColumnDropdown.svelte:17` and remove `bind:this={panel}` from the outer div at `ColumnDropdown.svelte:32`. The `bind:this` writes a DOM reference that is never consumed — the `onMount` block that could have used it was removed in 5a9e382. -- Improvement -- Dead code created by the fix commit; ~2-line removal with no behavior change.
  CITE: `frontend/src/components/ColumnDropdown.svelte` L:17
  CITE: `frontend/src/components/ColumnDropdown.svelte` L:32

#### PR Description Amendments
- Update the Accessibility notes section (from round-1 amendment) to reflect the `role="region"` or Disclosure pattern choice (whichever ships), so reviewers understand why `role="dialog"` is absent from the final implementation.

#### Critical Discoveries (informational)
*(None — no Security, Data Loss, or Compliance findings were raised in the debate.)*
