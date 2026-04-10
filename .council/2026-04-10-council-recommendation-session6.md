## Adversarial Council — Merge PR #30: Toolbar, Column Management & Export (Session 6)

> Convened: 2026-04-10 | Advocates: 1 | Critics: 1 | Questioner: DISABLED | Session: 6 (fresh audit of HEAD after 12 prior fixes)

### Motion
Merge PR #30: epic: Toolbar, Column Management & Export. Fresh 1v1 audit of current HEAD after 5 prior council sessions (12 findings, all marked fixed). This session verifies all prior fixes and hunts for newly surfaced defects.

### Prior-Fix Verification
All 12 session 1–5 findings verified against HEAD before debate:

| ID | Description | File | Verified |
|----|-------------|------|----------|
| F1 | role="region" on ColumnDropdown | `ColumnDropdown.svelte:30` | ✓ |
| F2 | TableHeader padding=0 | `TableHeader.svelte:24` | ✓ |
| F3 | hasTable on export button | `Toolbar.svelte:149` | ✓ |
| F4 | isSearchField guard for Ctrl+K | `App.svelte:110-114` | ✓ |
| F5 | Show All disabled={hiddenCount===0} | `ColumnDropdown.svelte:39` | ✓ |
| F6 | Dynamic aria-labels filter/columns | `Toolbar.svelte:102,132` | ✓ |
| F7 | type="text" (was type="search") | `App.svelte:504` | ✓ |
| F8 | Export disabled aria-label/title | `Toolbar.svelte:147-148` | ✓ |
| F9 | role="dialog" → role="region" | `ColumnDropdown.svelte:30` | ✓ |
| F10 | Search button dynamic label | `Toolbar.svelte:91` | ✓ |
| F11 | role="img" on sort indicator | `Toolbar.svelte:114` | ✓ |
| F12 | aria-controls="columns-panel" | `Toolbar.svelte:131` | ✓ |

### Positions

**ADVOCATE-1**: Core PR is production-quality. All 12 prior findings correctly applied. Spec Must-Haves verified:
- Vertical toolbar placement: `App.svelte:476-494`
- 300ms debounce with mutual cancellation: `App.svelte:355-356, 363-372`
- Ctrl+K with isSearchField guard: `App.svelte:110-114`
- Escape priority (columns → search): `App.svelte:123-133`
- localStorage resilience (load + persist both try/catch): `App.svelte:168-212`
- Race guard (selectRequestId): `App.svelte:47, 249, 267, 272`
- CSV export passes search/filter/sort: `App.svelte:414-428`

Conceded Defect 13 as real in Round 2.

**CRITIC-1**: All 12 prior findings verified. One new defect:

**Defect 13** — `searchTitle` at `Toolbar.svelte:59` is derived from `searchActive` (`searchVisible || searchQuery.length > 0`), but `toggleSearch()` at `App.svelte:227-234` branches only on `searchVisible`. When the user collapses the search bar (via Ctrl+K or toggle) while a search term is active, `searchVisible=false` but `searchQuery.length > 0`, so `searchActive=true` and `searchTitle='Close search (Ctrl+K)'`. However, `toggleSearch()` calls `openSearch()` in this state — opening the bar, not closing search. AT users hear "Close search" but the action is "open the bar." Mismatch between accessible label and action.

Fix: derive `searchTitle` from `searchVisible` alone.

### Concessions
- ADVOCATE-1: conceded Defect 13 as real; no rebuttal offered.
- CRITIC-1: withdrew any BLOCKING label; accepted CONDITIONAL as correct treatment.

### Arbiter Recommendation
**CONDITIONAL**

Thirteen findings across 6 sessions. Twelve verified clean. One new defect (D13) is a single-line fix that was applied before this recommendation was written. All agents converged on CONDITIONAL.

### Conditions
- Fix D13 verified as applied at `Toolbar.svelte:59` (changed `searchActive` → `searchVisible` in derivation).
- Manual verification: open search bar, type a term, press Ctrl+K to collapse bar — search button title/tooltip should read "Open search (Ctrl+K)", not "Close search."

### Suggested Fixes

**Fix D13 (APPLIED)** — Changed `searchTitle` derivation at `Toolbar.svelte:59` from `searchActive` to `searchVisible`:
```js
// Before:
let searchTitle = $derived(searchActive ? 'Close search (Ctrl+K)' : 'Open search (Ctrl+K)');
// After:
let searchTitle = $derived(searchVisible ? 'Close search (Ctrl+K)' : 'Open search (Ctrl+K)');
```
The `searchActive` visual state (button `.active` class) already communicates that a filter is running. The label names the action; `toggleSearch()` opens when `searchVisible=false` regardless of `searchActive`.
CITE: `frontend/src/components/Toolbar.svelte:59`, `frontend/src/App.svelte:227-234`

### Critical Discoveries
*(None — no Security, Data Loss, or Compliance findings.)*
