## Adversarial Council -- Merge PR #30: Toolbar, Column Management & Export

> Convened: 2026-04-10 | Advocates: 1 | Critics: 1 | Rounds: 2/4 | Motion type: CODE | Questioner: DISABLED

### Motion
Merge PR #30: epic: Toolbar, Column Management & Export (adds vertical icon toolbar, global search with 300ms debounce + Ctrl+K, column hide/show with localStorage, and CSV export wiring; +818/-327 across 6 files; closes #4)

### Advocate Positions
**ADVOCATE-1**: The PR delivers every Must-Have in `toolbar-column-export-spec.md` with verifiable line citations:
- Layout restructure complete: `ToolStrip.svelte` deleted (192 lines); vertical `Toolbar.svelte` rendered between Sidebar and main at `App.svelte:471-488`; new `TableHeader.svelte` at `App.svelte:491`.
- 300ms search debounce at `App.svelte:358-367` (`scheduleSearchReload`); keystroke handler at `App.svelte:369-372`.
- Keyboard shortcuts gated by `isTextEditingTarget()` at `App.svelte:96-101`; Ctrl+K at `App.svelte:110-114`; Escape priority (columns → search) at `App.svelte:122-133`.
- Search state reset on table switch at `App.svelte:256, 260` via `resetSearchState()` + explicit `''` parameter to `buildRowsParams`.
- Column visibility: `loadColumnVisibility()` at `App.svelte:167-191` has try/catch around `JSON.parse`; `normalizeColumnVisibility()` at `App.svelte:158-165` drops stale keys; localStorage key prefix at `constants.ts:5`.
- CSV export includes search at `App.svelte:413` (`searchParams.set('search', params.search)`).
- Debounce coordination: filter and search timers mutually cancel at `App.svelte:350-351` and `App.svelte:359`.
- Race protection preserved via `selectRequestId` at `App.svelte:244, 262`.
- Outside-click dismissal uses capture-phase `pointerdown` at `Toolbar.svelte:77-78`.

**CRITIC-1**: Four defects with verified citations. The backend wiring and state coordination are sound, but the accessibility and UX layer ships defects:
- **Defect 1** — `ColumnDropdown.svelte:46` wraps the list in `role="list"`, then renders `role="checkbox"` children at `ColumnDropdown.svelte:48-54`. ARIA § 6.1.4 requires `role="list"` to own `role="listitem"` children. This fails `axe-core aria-required-children`.
- **Defect 2** — `<button role="checkbox">` at `ColumnDropdown.svelte:48-50` produces conflicting keyboard instructions for AT: native `<button>` fires on Space AND Enter, but ARIA checkbox pattern expects Space-only.
- **Defect 3** — `<input type="search">` at `App.svelte:498` renders a native UA clear glyph in Chrome/Edge. The native X fires an `input` event hitting `handleSearchInput` at `App.svelte:369-372` which calls `scheduleSearchReload()` but does NOT call `handleSearchClear()` — so the search bar stays open. The custom `.clear-search` at `App.svelte:506-513` collapses the bar correctly. Chrome users see two divergent clear controls. Arbiter verified: no `::-webkit-search-cancel-button` CSS suppression exists anywhere in `frontend/`.
- **Defect 4** — Export button at `Toolbar.svelte:142-150` has no `disabled`/`aria-disabled`; prior implementation had `disabled={!onExport || !tableName}`.
- **Concern 5** — Escape handler at `App.svelte:122-133` is not gated by `!inTextField`; Ctrl+K/Ctrl+F at `App.svelte:110, 116` are.
- **Concern 6** — `persistColumnVisibility` at `App.svelte:203` calls `localStorage.setItem` with no try/catch around potential `QuotaExceededError`.

### Key Conflicts
- **Defects 1 & 2 (ARIA correctness)** — ADVOCATE-1 initially argued these warranted follow-up commits on main; CRITIC-1 argued that no CI accessibility gate exists in this repo (verified: `cargo test` covers Rust only; no axe-core or pa11y in CI), so "fix after merge" equals "fix when someone notices." **RESOLVED**: ADVOCATE-1 conceded the timing argument in Round 3. Both sides agree the fixes must land on this branch.
- **Defect 3 (spec classification)** — ADVOCATE-1 argued the native ⊗ is not a spec violation because the spec never addressed UA-injected chrome; CRITIC-1 accepted the framing revision but maintained the user-visible UX defect remains. **RESOLVED**: Both agree it's a real Chrome/Edge UX defect warranting a pre-merge CSS fix.
- **Defect 4 (export disabled)** — ADVOCATE-1 rebutted by showing the Toolbar renders only in the `{:else}` branch at `App.svelte:459`, and `onMount` auto-selects `tables[0]` at `App.svelte:85-86`, leaving the empty-`selectedTable` window limited to zero-tables databases. **RESOLVED**: CRITIC-1 withdrew the finding.
- **Concern 5 (Escape gating)** — ADVOCATE-1 rebutted the `columnsOpen` branch by showing the `pointerdown` capture handler at `Toolbar.svelte:77-78` closes the panel before focus transfers to a filter input. The `searchVisible` branch remains technically possible but was characterized as a minor polish item. **RESOLVED**: CRITIC-1 withdrew the finding.
- **Concern 6 (QuotaExceededError)** — CRITIC-1 withdrew as "blocking" but both sides acknowledged the defensive gap is real.

### Concessions
- ADVOCATE-1 conceded Defects 1 and 2 as real ARIA bugs in Round 2 (citing WAI-ARIA 1.2 § 6.1.4).
- ADVOCATE-1 conceded Defect 3 as a real cross-X-button inconsistency in Round 2, disputing only the "spec violation" label.
- ADVOCATE-1 conceded the timing argument for Defects 1, 2, 3 in Round 3 — the small fix surface is precisely why they should land on this branch, not post-merge.
- ADVOCATE-1 acknowledged Concern 6 as a real defensive coding gap.
- CRITIC-1 conceded debounce coordination (filter/search mutual cancellation) is correct in Round 1.
- CRITIC-1 conceded `loadColumnVisibility` has correct try/catch and schema drift handling in Round 1.
- CRITIC-1 withdrew Defect 4 (export disabled state) in Round 2 after the auto-select rebuttal at `App.svelte:85-86`.
- CRITIC-1 withdrew Concern 5 `columnsOpen` branch after the `pointerdown` capture rebuttal, and the `searchVisible` branch as minor polish.
- CRITIC-1 accepted ADVOCATE-1's framing revision on Defect 3 (spec does not address UA chrome).

### Arbiter Recommendation
**CONDITIONAL**

The core PR is well-implemented: debounce coordination, `selectRequestId` race guard, localStorage resilience, and every spec Must-Have map to verified lines. Both debaters converged on CONDITIONAL in Round 3 after ADVOCATE-1 conceded the timing argument — there is no CI accessibility gate in this repo, so a post-merge follow-up for ARIA defects is de facto "fix when noticed." The contested defects are surgical (total fix surface ~8-10 lines) and both sides explicitly agreed they should land on this branch before merge.

### Conditions (if CONDITIONAL)
- Apply Fixes 1-4 below on this branch before merging.
- Verify the fixed `ColumnDropdown` passes a local axe-core / Lighthouse accessibility scan (manual QA acceptable given no CI gate).
- Verify in Chrome/Edge that only one clear glyph is visible in the search input when text is present.

### Suggested Fixes

#### Fixes (all in-PR)

- **Fix D1** — Remove invalid `role="list"` ARIA ownership in ColumnDropdown. Either drop the `role="list"` attribute (the `<div>` has no implicit list role that needs reinforcing) or replace it with `role="group" aria-label="Column visibility"`. If you keep list semantics, wrap each child button in a `<div role="listitem">`. `frontend/src/components/ColumnDropdown.svelte:46` — Bug — Ships an ARIA ownership tree violation that fails the axe-core `aria-required-children` rule; screen readers cannot reliably announce the column list.
  CITE: `frontend/src/components/ColumnDropdown.svelte` L:46

- **Fix D2** — Remove `role="checkbox"` from the `<button>` elements at `ColumnDropdown.svelte:48-50`, or replace the whole pattern with a native `<input type="checkbox">` visually styled to match. A `<button>` with `role="checkbox"` gives AT users conflicting keyboard guidance (buttons toggle on Space+Enter; ARIA checkbox pattern is Space-only). The simplest correct fix is to keep the button element but drop `role="checkbox"` and `aria-checked`, and instead add an `aria-pressed={isVisible(column)}` attribute which matches the native button keyboard contract. `frontend/src/components/ColumnDropdown.svelte:48` — Bug — Mismatched ARIA role/element keyboard contract produces wrong AT instructions.
  CITE: `frontend/src/components/ColumnDropdown.svelte` L:48

- **Fix D3** — Suppress the native webkit search cancel button so only the custom `.clear-search` control is visible. Add to the `.search-input` style block in `App.svelte`:
  ```css
  .search-input::-webkit-search-cancel-button,
  .search-input::-webkit-search-decoration {
    -webkit-appearance: none;
    appearance: none;
  }
  ```
  Or alternatively switch `type="search"` to `type="text"` at `App.svelte:498`. `frontend/src/App.svelte:498` — Bug — In Chrome/Edge, two clear controls are visible with divergent behavior: the native ⊗ fires `handleSearchInput` which clears the term but leaves `searchVisible=true`, while the custom button calls `handleSearchClear()` which collapses the bar. One clear control, one behavior.
  CITE: `frontend/src/App.svelte` L:498

- **Fix C6** — Wrap the `localStorage.setItem` call in `persistColumnVisibility` with a try/catch so a `QuotaExceededError` (or other DOMException) does not bubble uncaught during normal column toggling. Log-and-continue is appropriate; the feature degrades gracefully to in-memory state. `frontend/src/App.svelte:203` — Hardening — `loadColumnVisibility` at `App.svelte:188` already guards with try/catch around `JSON.parse`; the write path should be symmetric. Both debaters acknowledged the gap is real, even after CRITIC-1 withdrew it as "blocking."
  CITE: `frontend/src/App.svelte` L:203

#### PR Description Amendments
- Add a short "Accessibility notes" section documenting that the column dropdown uses `aria-pressed` (or whichever pattern ships) on button toggles, and that the search input suppresses the native webkit cancel button in favor of a single custom clear control.
- Note the debounce coordination pattern (filter and search timers mutually cancel at `App.svelte:350-351, 359`) so reviewers and future maintainers understand the design intent.

#### Critical Discoveries (informational -- not fix targets)
*(None — no Security, Data Loss, or Compliance findings were raised in the debate.)*
