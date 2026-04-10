## Adversarial Council — Merge PR #30: Toolbar, Column Management & Export (Session 8)

> Convened: 2026-04-10 | Advocates: 1 | Critics: 1 | Rounds: 2/2 | Questioner: DISABLED | Basis: HEAD after 7 sessions / 14 fixes

### Motion
Merge PR #30: epic: Toolbar, Column Management & Export (adds vertical icon toolbar, global search with 300ms debounce + Ctrl+K, column hide/show with localStorage, and CSV export wiring). **Session 8** is a final clean-pass audit.

### Prior Sessions
Sessions 1–7: 14 findings fixed. `council-result.json` records full history.

---

### Finding S8-1 — MEDIUM — FIXED

**Search button aria-label doesn't convey active-filter state when panel is closed**

`Toolbar.svelte:61` (before fix):
```typescript
let searchTitle = $derived(searchVisible ? 'Close search (Ctrl+K)' : 'Open search (Ctrl+K)');
```

When `searchVisible=false` but `searchQuery.length > 0` (panel closed, rows filtered), sighted users see the teal-glowing button and understand a filter is active. Screen reader users tabbed to the button and heard **"Open search (Ctrl+K)"** — no indication that rows are currently filtered. The `searchActive` prop was already available in Toolbar.svelte (L:20, L:39) — no new prop needed.

**Fix applied** — three-way derivation at `Toolbar.svelte:61`:
```typescript
let searchTitle = $derived(
  searchVisible
    ? 'Close search (Ctrl+K)'
    : searchActive
      ? 'Search active — open search bar (Ctrl+K)'
      : 'Open search (Ctrl+K)'
);
```

CITE: `frontend/src/components/Toolbar.svelte:61`, `frontend/src/components/Toolbar.svelte:20`

---

### Finding S8-2 — LOW — FIXED (optional cleanup)

**Dead CSS: .search-panel transition and .search-panel.active transform are inert**

`App.svelte:585-594` (before fix):
```css
.search-panel {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.search-panel.active {
  transform: translateY(0);
}
```

The panel is conditionally rendered via `{#if searchVisible}` (`App.svelte:499`). `{#if}` removes the DOM node — CSS transitions on a non-existent node cannot fire. Additionally `translateY(0)` is CSS's default; there was no initial `translateY(N)` to transition from. The `opacity` transition was also never triggered. Both rules removed.

CITE: `frontend/src/App.svelte:499`, `frontend/src/App.svelte:585-594`

---

### Concessions
- **ADVOCATE-1** conceded S8-1 (AT gap, `searchActive` not reflected in aria-label) is real and MEDIUM.
- **ADVOCATE-1** conceded S8-2 (dead CSS) is real and LOW.
- **CRITIC-1** raised no additional findings.

---

### Arbiter Recommendation
**CONDITIONAL → FIXED (post-patch)**

Both fixes applied in this session. `svelte-check` passes 0 errors 0 warnings post-patch.

### Conditions
All 16 findings (14 from S1-7 + 2 from S8) are fixed. No remaining conditions. PR is clean for merge.

### Suggested Fixes
*(All applied — none pending.)*

#### Critical Discoveries (informational)
*(None — no Security, Data Loss, or Compliance findings were raised.)*
