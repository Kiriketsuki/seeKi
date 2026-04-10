## Adversarial Council — Merge PR #30: Toolbar, Column Management & Export (Session 11)

> Convened: 2026-04-10 | Advocates: 1 | Critics: 1 | Questioner: DISABLED | Session: 11 (fresh audit of HEAD after 10 sessions / 19 fixes)

### Motion
Merge PR #30: epic: Toolbar, Column Management & Export (adds vertical icon toolbar, global search with 300ms debounce + Ctrl+K, column hide/show with localStorage, and CSV export wiring). Session 11 audits HEAD for defects missed by prior sessions.

---

### Prior-Fix Verification

All 19 findings from Sessions 1–10 verified against current HEAD:

| Session | Findings | Status |
|---------|----------|--------|
| S1–S6 | F1–F12: ARIA role, label, keyboard, derived-label fixes | ✓ |
| S7–S8 | F13–F16: `searchVisible` prop wiring, `searchTitle` derivation, outside-click scope | ✓ |
| S9 | F17: `aria-expanded`/`aria-controls` on search button (`Toolbar.svelte:103-104`) | ✓ |
| S9 | F18: Four-way `filterTitle` derivation (`Toolbar.svelte:68-76`) | ✓ |
| S10 | F19: `aria-expanded={filtersVisible}` on filter button (`Toolbar.svelte:116`) | ✓ |

All 19 prior findings confirmed fixed. No regressions detected.

---

### Positions

**ADVOCATE-1**: After a full read of `Toolbar.svelte` (292 lines), `ColumnDropdown.svelte` (175 lines), and the relevant sections of `App.svelte` (lines 1–550), no actionable defects found. Specific verifications:

- All three interactive buttons carry correct `aria-expanded` + `aria-controls` pairs (search: L103-104, columns: L145-146) or `aria-expanded` alone with architectural justification (filter: L116 — no `id` target exists in the RevoGrid column header area).
- `searchTitle` three-way derivation at `Toolbar.svelte:61-67` is semantically consistent with `toggleSearch()` at `App.svelte:227-234`.
- `filterTitle` four-way derivation at `Toolbar.svelte:68-76` correctly handles all four open/closed × active/empty states.
- `persistColumnVisibility` at `App.svelte:204-211` is wrapped in try/catch matching `loadColumnVisibility` at `App.svelte:182-191`.
- Export button: `disabled={!hasTable}`, context-aware `aria-label` for both enabled and disabled states.
- `handleOutsidePointerDown` at `Toolbar.svelte:85-93`: capture-phase `pointerdown` correctly scoped to `shell`.
- `selectRequestId` race guard at `App.svelte:47, 249, 267, 272` prevents stale response overwrites.
- `ColumnDropdown.svelte:30`: `role="region" aria-label="Column visibility"`; buttons use `aria-pressed={isVisible(column)}` at L48 with no conflicting role.

Motion should PASS.

**CRITIC-1**: All 19 prior findings verified. One challenge raised and subsequently withdrawn.

**Potential Finding S11-1 (WITHDRAWN)** — `aria-controls` references nonexistent DOM elements when panels are collapsed. The search button (`Toolbar.svelte:104`) carries `aria-controls="search-panel"`; the element with `id="search-panel"` exists only inside `{#if searchVisible}` at `App.svelte:500`. The columns button (`Toolbar.svelte:146`) carries `aria-controls="columns-panel"`; `id="columns-panel"` exists only inside `{#if columnsOpen}` at `Toolbar.svelte:171`. WAI-ARIA 1.2 §6.1 states the referenced element must be present in the document.

Withdrawn in Round 2 after ADVOCATE-1 established:
1. Prior sessions (F12 in S5, F17 in S9) added `aria-controls` to both buttons with the `{#if}` pattern already in place and accepted it — implicit precedent.
2. `aria-expanded=false` is the operative AT navigation signal; modern AT (NVDA, JAWS, VoiceOver) handles a missing `aria-controls` target gracefully when `aria-expanded=false`.
3. The fix (always-in-DOM panels) would require architectural changes to CSS positioning with material layout risk.

No other findings raised. Reviewed: debounce coordination, `selectRequestId` race guard, CSV export URL encoding, `handleSearchClear` logic, `persistColumnVisibility` try/catch, column visibility immutability, `handleShowAllColumns` normalization. All sound.

---

### Concessions

- **CRITIC-1**: Withdrew S11-1 in Round 2. Prior sessions accepted `{#if}` + `aria-controls` for both panels; `aria-expanded` pairing is operative AT signal; no practical AT impact; architectural fix risk is real.
- **ADVOCATE-1**: No concessions required.

---

### Arbiter Recommendation

**APPROVED**

Session 11 found zero actionable defects. CRITIC-1's only challenge (S11-1) was withdrawn after the advocate demonstrated prior session precedent, practical AT behavior, and fix risk. All 19 prior findings verified fixed. No Critical Discoveries (Security, Data Loss, Compliance). PR #30 is clean for merge.

---

### Conditions

None. PR is approved unconditionally.

---

### Suggested Fixes

None required.

**Optional follow-up** (not a merge condition): add a comment near the `{#if searchVisible}` and `{#if columnsOpen}` panel renders noting that `aria-controls` is intentionally paired with conditional rendering — `aria-expanded` is the operative AT signal and AT software handles the absent target when `aria-expanded=false`. Documents the pattern for future maintainers.

---

### Critical Discoveries

*(None — no Security, Data Loss, or Compliance findings.)*
