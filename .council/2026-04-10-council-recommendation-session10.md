## Adversarial Council — Merge PR #30: Toolbar, Column Management & Export (Session 10)

> Convened: 2026-04-10 | Advocates: 1 | Critics: 1 | Questioner: DISABLED | Session: 10

### Motion

Merge PR #30: epic: Toolbar, Column Management & Export. Session 10 — fresh 1v1 audit of HEAD after 9 sessions (18 findings, all fixed).

---

### Prior-Fix Verification

All 18 findings from sessions 1–9 verified against current HEAD:

| Session | Finding | Status |
|---------|---------|--------|
| S1–S6 | F1–F12: aria-label, role, keyboard, derived-label fixes | ✓ |
| S7–S8 | F13–F16: columns panel aria-controls, export disabled, outside-click | ✓ |
| S9 | F17: `aria-expanded`/`aria-controls` on search button (`Toolbar.svelte:103-104`) | ✓ |
| S9 | F18: Four-way `filterTitle` derivation (`Toolbar.svelte:68-76`) | ✓ |

All prior findings confirmed fixed. No regressions detected.

---

### Positions

**ADVOCATE-1**: All 18 prior findings verified. Core PR is production-quality. The filter button communicates state through the four-way `filterTitle` label (F18, verified in session 9): "Close filters — N active", "Close filters", "Filters active (N) — open panel", "Toggle filters". Both `aria-label` and `title` are bound to `filterTitle`, meaning AT receives precise state text at all times.

In Round 2: Concedes that `aria-expanded` is the canonical programmatic state attribute for toggle controls and cannot be substituted by label text alone. Accepts CONDITIONAL with F19 as the fix condition. Disputes only severity framing (SHOULD vs. MUST), not the fix itself.

**CRITIC-1**: One defect with verified citation.

**Defect 19** — `Toolbar.svelte:112-126` — filter button lacks `aria-expanded`. The three interactive buttons in the toolbar have the following ARIA disclosure state:

| Button | `aria-expanded` | `aria-controls` |
|--------|-----------------|-----------------|
| Search (`Toolbar.svelte:99-110`) | `aria-expanded={searchVisible}` | `aria-controls="search-panel"` |
| Columns (`Toolbar.svelte:140-156`) | `aria-expanded={columnsOpen}` | `aria-controls="columns-panel"` |
| **Filter (`Toolbar.svelte:112-126`)** | **absent** | absent (architecturally justified) |

WCAG 4.1.2 requires that user interface components expose their state programmatically. `aria-expanded` is the machine-readable state hook that AT uses for its verbosity and formatting decisions. The `filterTitle` label conveys intent in human-readable text but is not a substitute — a screen reader user navigating with arrow keys hears the label but does not receive the canonical `expanded=true/false` state attribute.

Session 9 added `aria-expanded={searchVisible}` to the search button (F17) but did not update the filter button in the same pass. This is a carry-forward miss. The `aria-controls` omission is architecturally justified — filter inputs render inline inside RevoGrid column headers with no top-level `id`. But `aria-expanded` alone is valid per ARIA 1.2 §6.2.4 when no `aria-controls` target exists.

No other defects found. Reviewed: export disabled state (`Toolbar.svelte:163`), outside-click handler (`Toolbar.svelte:85-93`), sort indicator (`Toolbar.svelte:128`), `searchActive` derivation (`App.svelte:52`), column visibility persistence (`App.svelte:194-212`), CSV export encoding (`App.svelte:426`). All sound.

---

### Concessions

- **ADVOCATE-1**: Concedes `aria-expanded={filtersVisible}` must be added; accepts CONDITIONAL approval.
- **CRITIC-1**: Accepts that `aria-controls` omission on the filter button is architecturally justified and is not a finding.

---

### Arbiter Recommendation

**CONDITIONAL**

The PR is structurally sound. All 18 prior findings verified fixed. One carry-forward omission from session 9: the filter button does not expose `aria-expanded`, while the search and columns buttons — fixed in earlier sessions — do. The four-way `filterTitle` label text communicates state in natural language, but `aria-expanded` is the programmatic attribute required by WCAG 4.1.2 for AT software. The fix is one attribute on one element with zero architectural risk.

No Critical Discoveries (security, data loss, compliance).

---

### Conditions

1. **F19** — Add `aria-expanded={filtersVisible}` to the filter button at `Toolbar.svelte:112`. `aria-controls` is waived (no `id` target exists). Upon fix, the filter button matches the accessible pattern of the search and columns buttons.

---

### Suggested Fixes

**Fix F19** — `frontend/src/components/Toolbar.svelte:112-119`:

```svelte
<!-- Before -->
<button
  type="button"
  class="tool-button"
  class:active={filtersVisible || activeFilterCount > 0}
  aria-label={filterTitle}
  title={filterTitle}
  onclick={() => onToggleFilters?.()}
>

<!-- After -->
<button
  type="button"
  class="tool-button"
  class:active={filtersVisible || activeFilterCount > 0}
  aria-expanded={filtersVisible}
  aria-label={filterTitle}
  title={filterTitle}
  onclick={() => onToggleFilters?.()}
>
```

This is the complete fix. No other changes required.

---

### Critical Discoveries

*(None — no Security, Data Loss, or Compliance findings.)*
