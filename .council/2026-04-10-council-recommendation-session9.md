## Adversarial Council — Merge PR #30: Toolbar, Column Management & Export (Session 9)

> Convened: 2026-04-10 | Advocates: 1 | Critics: 1 | Rounds: 2 | Questioner: DISABLED | Basis: HEAD after 8 sessions / 16 fixes

### Motion
Merge PR #30: epic: Toolbar, Column Management & Export (adds vertical icon toolbar, global search with 300ms debounce + Ctrl+K, column hide/show with localStorage, and CSV export wiring). **Session 9** is a fresh clean-pass audit of HEAD.

### Prior Sessions
Sessions 1–8: 16 findings, all fixed. `council-result.json` records full history.

---

### Finding S9-1 — MEDIUM — FIXED

**Search button missing `aria-expanded` and `aria-controls`**

The columns disclosure button at `Toolbar.svelte:127-137` (fixed sessions 1 and 5) correctly carries `aria-expanded={columnsOpen}` and `aria-controls="columns-panel"`, linking it to `<div id="columns-panel">` in ColumnDropdown. The search button at `Toolbar.svelte:86-96` controls a collapsible panel (`<div class="search-panel">` at `App.svelte:499`) but exposes no `aria-expanded` and no `aria-controls`. AT users cannot determine programmatically whether the search panel is open or closed.

**Fix applied**:
- Added `aria-expanded={searchVisible}` and `aria-controls="search-panel"` to search button (`Toolbar.svelte:97-98`)
- Added `id="search-panel"` to the `.search-panel` div (`App.svelte:500`)

CITE: `frontend/src/components/Toolbar.svelte:86-104`, `frontend/src/App.svelte:499`

---

### Finding S9-2 — LOW — FIXED

**`filterTitle` says "Filters active (0)" when filter panel is open with no filters entered**

`filterTitle` at `Toolbar.svelte:68-72` (before fix):
```js
let filterTitle = $derived(
  filtersVisible || activeFilterCount > 0
    ? `Filters active (${activeFilterCount}) (Ctrl+F)`
    : 'Toggle filters (Ctrl+F)'
);
```
When `filtersVisible=true` and `activeFilterCount=0`, label = "Filters active (0) (Ctrl+F)". Self-contradictory: "active" with zero active filters. The button also glows teal in this state, compounding the confusion.

**Fix applied** — four-way derivation at `Toolbar.svelte:68-76`:
```js
let filterTitle = $derived(
  filtersVisible && activeFilterCount > 0
    ? `Close filters — ${activeFilterCount} active (Ctrl+F)`
    : filtersVisible
      ? 'Close filters (Ctrl+F)'
      : activeFilterCount > 0
        ? `Filters active (${activeFilterCount}) — open panel (Ctrl+F)`
        : 'Toggle filters (Ctrl+F)'
);
```
Each state now maps to an unambiguous label: open+active, open+empty, closed+active, closed+empty.

CITE: `frontend/src/components/Toolbar.svelte:68`

---

### Concessions
- **ADVOCATE-1** conceded S9-1 as a genuine ARIA disclosure gap parallel to the columns fix.
- **ADVOCATE-1** conceded S9-2 as a semantically broken label.
- **CRITIC-1** raised no additional findings in Round 2.

---

### Arbiter Recommendation
**CONDITIONAL → FIXED (post-patch)**

Both fixes applied. `svelte-check` passes 0 errors 0 warnings post-patch. 18 total findings across 9 sessions, all fixed. No remaining conditions. PR is clean for merge.

### Conditions
All 18 findings (16 from S1-8 + 2 from S9) are fixed.

### Suggested Fixes
*(All applied — none pending.)*

#### Critical Discoveries (informational)
*(None — no Security, Data Loss, or Compliance findings.)*
