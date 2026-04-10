# Feature: Toolbar, Column Management & Export

## Overview

**User Story**: As a non-technical database viewer, I want a vertical toolbar with search, column visibility controls, and CSV export so that I can quickly find data, focus on relevant columns, and download results without writing SQL.

**Problem**: The current UI has placeholder toolbar buttons (disabled search, columns, export) split across two components (horizontal Toolbar and vertical ToolStrip). Users cannot search across columns, hide irrelevant columns, or export data — the three most-requested data exploration features.

**Out of Scope**:
- Backend API changes (search, filter, sort, and CSV export endpoints already implemented)
- Column reordering / drag-and-drop
- Export formats other than CSV (Excel, JSON)
- Saved column visibility presets / profiles
- Full-text search configuration (which columns to include)

---

## Success Condition

> This feature is complete when users can type a search term to filter rows across all text columns, toggle column visibility via a popover with per-table localStorage persistence, and export the current filtered/sorted view as a CSV download — all from a unified vertical toolbar.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | None identified | — | [x] |

---

## Scope

### Must-Have
- **Layout restructure**: Replace horizontal `Toolbar.svelte` + vertical `ToolStrip.svelte` with a single vertical `Toolbar.svelte` icon bar between sidebar and grid, plus a minimal `TableHeader.svelte` (table name + row count)
- **Global search**: Search icon in vertical toolbar expands a search bar below `TableHeader`; 300ms debounce; ILIKE across text columns; clear button + Escape collapses; Ctrl+K toggles; teal highlight when active; resets on table switch
- **Column hide/show**: Columns icon in vertical toolbar opens a popover anchored to the right; checkbox list of all columns; unchecked columns are struck through and hidden from DataGrid; "Show All" reset link; per-table localStorage persistence (`sk-column-visibility-{table}`); clicking outside closes popover
- **CSV export**: Export icon in vertical toolbar triggers `window.open()` to `/api/export/{table}/csv` with current search, filters, and sort params; browser handles download natively (no loading state needed)
- **Search in export**: Search term is included in the CSV export URL so the export reflects the current search filter
- **ToolStrip cleanup**: Remove disabled Columns and Export buttons from ToolStrip (component deleted entirely as part of layout restructure)

### Should-Have
- **Keyboard shortcut hints**: Tooltips on toolbar icons showing shortcuts (Ctrl+K for search, Ctrl+F for filters)

### Nice-to-Have
- **Column count badge**: Show count of hidden columns on the Columns icon (e.g., badge showing "2" when 2 columns hidden)

---

## Technical Plan

**Affected Components**:

| File | Action | Description |
|:-----|:-------|:------------|
| `frontend/src/components/Toolbar.svelte` | Rewrite | Vertical icon bar: search, filter, sort indicator, separator, columns, export |
| `frontend/src/components/ToolStrip.svelte` | Delete | Replaced by vertical Toolbar |
| `frontend/src/components/TableHeader.svelte` | New | Minimal header: table name + row count |
| `frontend/src/components/ColumnDropdown.svelte` | New | Popover with column visibility checkboxes |
| `frontend/src/App.svelte` | Modify | New state (`searchTerm`, `columnVisibility`, `searchVisible`), layout update, wire new props |
| `frontend/src/lib/constants.ts` | Modify | Add `COLUMN_VISIBILITY_KEY_PREFIX` constant |

**Data Model Changes**: None — all state is client-side.

**API Contracts**: No new endpoints. Existing endpoints used:
- `GET /api/tables/{table}/rows?search={term}` — already supports `search` param
- `GET /api/export/{table}/csv?search={term}&sort_column=...&filter.*=...` — already supports all params

**Dependencies**: None — all features use existing backend APIs and frontend libraries (lucide-svelte for icons).

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| RevoGrid column filtering may not respond to dynamic column list changes | Low | Test with `columns` prop reactivity; RevoGrid Svelte wrapper re-renders on prop change |
| Popover positioning may overflow viewport on small screens | Low | Use CSS `max-height` with overflow-y scroll; anchor right of icon so it opens into main content area |
| localStorage quota exceeded on tables with many columns | Very Low | Column visibility is tiny data (~100 bytes per table); not a real risk |

---

## Acceptance Scenarios

```gherkin
Feature: Toolbar, Column Management & Export
  As a non-technical database viewer
  I want search, column controls, and export in a vertical toolbar
  So that I can explore data without SQL

  Background:
    Given the app is loaded with a database containing tables
    And a table with columns [id, name, email, created_at, status] is selected

  Rule: Layout shows vertical toolbar between sidebar and grid

    Scenario: Vertical toolbar replaces horizontal toolbar and ToolStrip
      Given the app renders the main layout
      Then a vertical icon bar appears between the sidebar and the grid area
      And a minimal header above the grid shows the table name and row count
      And there is no horizontal toolbar component
      And there is no separate ToolStrip component

  Rule: Global search filters rows across all text columns

    Scenario: User searches and results update after debounce
      Given the search bar is not visible
      When the user clicks the search icon in the toolbar
      Then a search input bar appears below the table header
      When the user types "alice" into the search input
      And 300ms passes without further typing
      Then the grid reloads showing only rows matching "alice" across text columns
      And the row count in the header updates to reflect filtered results

    Scenario: User clears search
      Given the search bar shows "alice" with filtered results
      When the user clicks the clear button
      Then the search input clears
      And the grid reloads showing all rows
      And the search bar collapses

    Scenario: Keyboard shortcut toggles search
      When the user presses Ctrl+K
      Then the search bar appears and the input is focused
      When the user presses Escape
      Then the search bar collapses and search clears

    Scenario: Search resets on table switch
      Given the user has an active search for "alice"
      When the user selects a different table
      Then the search term clears
      And the search bar collapses

  Rule: Column visibility controls which columns appear in the grid

    Scenario: User hides a column via the popover
      When the user clicks the Columns icon in the toolbar
      Then a popover appears to the right of the icon showing all column names with checkboxes
      And all checkboxes are checked by default
      When the user unchecks "created_at"
      Then "created_at" appears struck through in the popover
      And the "created_at" column disappears from the grid

    Scenario: User restores all columns
      Given the user has hidden "created_at" and "email"
      When the user clicks "Show All" in the column popover
      Then all checkboxes become checked
      And all columns reappear in the grid

    Scenario: Column visibility persists across page reloads
      Given the user has hidden "created_at" on the "users" table
      When the user reloads the page and selects the "users" table
      Then "created_at" is still hidden
      And the popover shows "created_at" unchecked

    Scenario: Column visibility is independent per table
      Given the user has hidden "created_at" on the "users" table
      When the user switches to the "logs" table
      Then all columns on "logs" are visible (default state)

    Scenario: Popover closes on outside click
      Given the column popover is open
      When the user clicks anywhere outside the popover
      Then the popover closes

  Rule: CSV export downloads current view

    Scenario: User exports CSV with active search and filters
      Given the user has searched for "alice"
      And column "status" is filtered to "active"
      And rows are sorted by "name" ascending
      When the user clicks the Export icon
      Then the browser opens a download for the CSV
      And the CSV contains only rows matching the search, filter, and sort

    Scenario: Export works with no active filters
      Given no search, filters, or sort are active
      When the user clicks the Export icon
      Then the browser downloads a CSV of all rows in default order
```

---

## Task Breakdown

| ID   | Task | Priority | Dependencies | Status  |
|:-----|:-----|:---------|:-------------|:--------|
| T1   | Create `TableHeader.svelte` — minimal header with table name + row count props | High | None | pending |
| T2   | Rewrite `Toolbar.svelte` as vertical icon bar — search, filter, sort indicator, separator, columns, export icons with tooltip titles | High | None | pending |
| T3   | Wire layout in `App.svelte` — replace horizontal Toolbar + ToolStrip with vertical Toolbar + TableHeader; update grid-area layout | High | T1, T2 | pending |
| T4   | Add search state to `App.svelte` — `searchTerm`, `searchVisible`, debounce logic; pass `search` param through `buildRowsParams`; reset on table switch | High | T3 | pending |
| T5   | Wire search bar UI — expanding search input below TableHeader, controlled by `searchVisible`; clear button; Ctrl+K shortcut; Escape to close; teal highlight on active | High | T4 | pending |
| T6   | Pass search term to CSV export URL in `exportCsv()` | High | T4 | pending |
| T7   | Create `ColumnDropdown.svelte` — popover with checkboxes, "Show All" link, click-outside-to-close | High | None | pending |
| T8   | Add column visibility state to `App.svelte` — `columnVisibility: Record<string, boolean>`, localStorage read/write with `sk-column-visibility-{table}` key, reset on table switch (load from localStorage or default all visible) | High | T7 | pending |
| T9   | Filter `columns` prop passed to DataGrid based on visibility state | High | T8 | pending |
| T10  | Wire Columns icon in Toolbar to toggle ColumnDropdown popover | High | T7, T3 | pending |
| T11  | Delete `ToolStrip.svelte` and remove all references | Med | T3 | pending |
| T12  | Add `COLUMN_VISIBILITY_KEY_PREFIX` to constants.ts | Low | None | pending |
| T13  | Manual QA — verify search, column toggle, export, localStorage persistence, keyboard shortcuts, table switching resets | High | T5, T9, T10, T6 | pending |

---

## Exit Criteria

- [ ] All Must-Have scenarios pass manual verification
- [ ] Search debounce fires correctly (300ms, no redundant requests)
- [ ] Column visibility persists in localStorage per table and survives page reload
- [ ] CSV export includes search term in URL params
- [ ] No regressions on existing filter, sort, and pagination features
- [ ] `ToolStrip.svelte` fully removed with no dead references
- [ ] Vertical toolbar renders correctly at common viewport widths (1280px+)

---

## References

- Epic issue: #4
- Sub-issues: #22 (search), #23 (column visibility), #24 (CSV export)
- PR: #30
- Backend API: `src/api/mod.rs` — search, filter, sort, CSV export endpoints

---
*Authored by: Clault KiperS 4.6*
