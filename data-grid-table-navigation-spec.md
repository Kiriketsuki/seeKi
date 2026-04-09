# Feature: Data Grid & Table Navigation (Epic 3)

## Overview

**User Story**: As a non-technical database user, I want a spreadsheet-like grid with sorting, filtering, and type-aware formatting so that I can explore large datasets without writing SQL or using DBeaver.

**Problem**: The current frontend renders raw data in a bare grid — no sort indicators, no column filters, no display formatting (timestamps show as ISO strings, booleans as `true`/`false`, NULLs as blank). Users cannot narrow results or understand data types at a glance. The sidebar table list has no search, making navigation painful with 20+ tables.

**Out of Scope**:
- Multi-column sort (single column only for MVP)
- Client-side data caching or optimistic updates
- Column reordering or resizing (RevoGrid default resizing is acceptable)
- Global search wiring (Epic 4 — Toolbar)
- Column hide/show (Epic 4)
- CSV export button (Epic 4)
- Setup wizard (Epic 5)

---

## Success Condition

> This feature is complete when a user can select a table from a searchable sidebar, view data in a virtualized grid with type-aware formatting, click column headers to sort (server-side), toggle per-column text filters from a left tool strip, and paginate through results — all working in both mock mode and against a live PostgreSQL database.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | RevoGrid `columnTemplate` vs custom header overlay for sort/filter UI — need to verify RevoGrid 4.x Svelte 5 template API | Brainstorm | [x] Resolved: `columnTemplate(h, props)` returns VNodes via hyperscript; `cellTemplate(h, { value })` for cell formatting; `beforesorting` event supports `preventDefault()`. All confirmed via Context7 docs. |

---

## Scope

### Must-Have

- **RevoGrid as dumb renderer**: Disable built-in sort/filter; use `readonly` prop; CSS flex height cascade for virtualization (T9 / #17)
  - *Acceptance*: Grid renders 50 rows with virtual scrolling active; no built-in RevoGrid sort/filter UI visible
- **Click-to-sort columns**: Three-state cycle (asc -> desc -> unsorted) on column headers; single column at a time; chevron indicator on active column; server-side sort via `sort_column`/`sort_direction` params (T11 / #18)
  - *Acceptance*: Clicking a column header cycles through asc/desc/unsorted; sort chevron visible on active column; data re-fetches from server with correct params
- **Per-column text filters**: Filter inputs inside header cells, hidden by default; toggle via funnel icon in left tool strip; 300ms debounce; AND logic across columns; active filter count badge on funnel icon when filters hidden (T12 / #19)
  - *Acceptance*: Toggling filter shows input in each column header; typing triggers debounced server fetch with `filter.{col}` params; multiple filters combine with AND; badge shows count when collapsed
- **Display formatting**: Type-aware cell renderers for timestamps, booleans, numbers, NULLs (T13 / #20)
  - *Acceptance*: Timestamps show "Apr 7, 3:58 PM" with full ISO tooltip; booleans render as colored badges (green "Yes" / red "No"); numbers are locale-formatted; NULLs show dim italic "NULL" text on hatched background
- **Sidebar table search**: Client-side text filter input at top of table list, case-insensitive match against display name (T8 / #16)
  - *Acceptance*: Typing in search box filters table list instantly; clearing restores full list
- **Status bar verification**: Confirm existing status bar meets acceptance criteria — "Showing X-Y of N" with working pagination (T16 / #21)
  - *Acceptance*: Status bar shows correct counts; prev/next buttons work; disabled at boundaries
- **Left vertical tool strip** (new component): ~36-40px wide icon strip between sidebar and grid; top section for data manipulation tools (filter toggle, sort indicator); bottom section reserved for future utility tools (export, column visibility); visual separator between sections; icon buttons with tooltips
  - *Acceptance*: Tool strip renders between sidebar and grid area; funnel icon toggles filter row; sort state visible; tooltips on hover

### Should-Have

- **Mock filter support**: Extend `mock.ts` to support `filter.{col}` params so filter UI works in `VITE_MOCK=true` mode
- **Keyboard shortcut for filter toggle**: `Ctrl+F` / `Cmd+F` to toggle filter row visibility

### Nice-to-Have

- **Clear all filters button**: Single action to reset all active column filters
- **Sort indicator in tool strip**: Small label or icon showing current sort column and direction

---

## Technical Plan

**Affected Components**:

| File | Change |
|:-----|:-------|
| `frontend/src/App.svelte` | Add ToolStrip to layout; wire sort/filter state; pass callbacks down |
| `frontend/src/components/DataGrid.svelte` | Major rework — `columnTemplate(h, props)` for custom headers (sort chevrons + filter inputs); `cellTemplate(h, { value })` for display formatting; `beforesorting` event with `preventDefault()` to block built-in sort |
| `frontend/src/components/ToolStrip.svelte` | **New** — vertical icon strip with filter toggle, sort indicator |
| `frontend/src/components/Sidebar.svelte` | No change (already done) |
| `frontend/src/components/TableList.svelte` | Add search input prop and filtering logic |
| `frontend/src/components/StatusBar.svelte` | Verify only — likely no changes |
| `frontend/src/components/Toolbar.svelte` | Remove filter/sort responsibilities (moved to ToolStrip); may simplify |
| `frontend/src/lib/api.ts` | No change — `fetchRows` already supports all needed params |
| `frontend/src/lib/mock.ts` | Add per-column filter support to `mockFetchRows` |
| `frontend/src/lib/types.ts` | Add `SortState` and `FilterState` types |
| `frontend/src/theme/tokens.css` | Add `--sk-null-hatch` pattern, any tool strip tokens |

**Data Model Changes**: None — backend API already supports all required params.

**API Contracts**: No new endpoints. Existing endpoints used:
- `GET /api/tables/{table}/rows?sort_column=X&sort_direction=asc|desc&filter.{col}=value&page=N` — already implemented
- Sort/filter params already validated server-side with identifier validation

**Dependencies**:
- `@revolist/svelte-datagrid` ^4.21.4 (already installed)
- `lucide-svelte` (already installed — icons: `Filter`, `ArrowUpDown`, `ChevronUp`, `ChevronDown`)
- No new packages required

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| RevoGrid Svelte 5 `columnTemplate` API is undocumented or broken | Low (verified via Context7 docs — hyperscript `h` function works) | Fallback: render custom header row outside RevoGrid as an overlay div, position-synced with grid scroll |
| CSS height cascade doesn't trigger RevoGrid virtualization | Low | Fallback: ResizeObserver to set explicit pixel height on grid container |
| RevoGrid intercepts header clicks despite `readonly` | Low | Use `beforesorting` event with `preventDefault()` as additional guard |

---

## Acceptance Scenarios

```gherkin
Feature: Data Grid & Table Navigation
  As a non-technical database user
  I want a spreadsheet-like grid with sorting, filtering, and formatting
  So that I can explore large datasets without SQL

  Background:
    Given the app is loaded with mock data (VITE_MOCK=true)
    And the "Users" table is selected

  Rule: Grid renders data with virtual scrolling

    Scenario: Initial grid load
      Given the "Users" table has 42 rows
      When the grid renders
      Then 42 rows are displayed across pages
      And RevoGrid virtual scrolling is active (DOM contains fewer elements than total rows)
      And column headers show display names from config

    Scenario: Grid fills available height
      Given the browser window is 900px tall
      When the sidebar is collapsed
      Then the grid expands to fill the main area height
      And no unnecessary scrollbars appear on the page body

  Rule: Click-to-sort cycles through three states

    Scenario: Sort ascending on first click
      Given no column is currently sorted
      When I click the "Name" column header
      Then the grid re-fetches with sort_column=name and sort_direction=asc
      And an upward chevron appears on the "Name" header

    Scenario: Sort descending on second click
      Given the "Name" column is sorted ascending
      When I click the "Name" column header again
      Then the grid re-fetches with sort_column=name and sort_direction=desc
      And a downward chevron appears on the "Name" header

    Scenario: Clear sort on third click
      Given the "Name" column is sorted descending
      When I click the "Name" column header again
      Then the grid re-fetches with no sort params
      And no sort chevron appears on any header

    Scenario: Sorting a different column replaces current sort
      Given the "Name" column is sorted ascending
      When I click the "Email" column header
      Then the grid re-fetches with sort_column=email and sort_direction=asc
      And the chevron moves from "Name" to "Email"

  Rule: Per-column filters are toggleable and debounced

    Scenario: Toggle filter visibility
      Given filters are hidden (default state)
      When I click the filter icon in the tool strip
      Then a text input appears inside each column header
      And the filter icon shows an active state

    Scenario: Typing a filter triggers debounced fetch
      Given filters are visible
      When I type "alice" in the "Name" filter input
      And I wait 300ms
      Then the grid re-fetches with filter.name=alice
      And the results show only rows matching "alice" in the Name column

    Scenario: Multiple filters combine with AND logic
      Given I have filter.name=alice active
      When I type "active" in the "Status" filter input and wait 300ms
      Then the grid re-fetches with filter.name=alice AND filter.status=active
      And only rows matching both filters are shown

    Scenario: Active filter count badge
      Given 2 column filters are active
      When I hide the filter row via the tool strip icon
      Then the funnel icon shows a "2" badge
      And filters remain active (data is still filtered)

    Scenario: Clearing a filter re-fetches without it
      Given filter.name=alice is active
      When I clear the "Name" filter input and wait 300ms
      Then the grid re-fetches without the name filter

  Rule: Cells render with type-aware formatting

    Scenario Outline: Display formatting by data type
      Given a cell contains <raw_value> with data_type <type>
      When the grid renders
      Then the cell displays <formatted>
      And <tooltip_behaviour>

      Examples:
        | type                         | raw_value                 | formatted         | tooltip_behaviour                        |
        | timestamp without time zone  | 2026-04-07T15:58:00      | Apr 7, 3:58 PM    | hover shows "2026-04-07T15:58:00"        |
        | boolean                      | true                     | green "Yes" badge  | no tooltip                               |
        | boolean                      | false                    | red "No" badge     | no tooltip                               |
        | integer                      | 1234567                  | 1,234,567          | no tooltip                               |
        | numeric                      | 1234567.89               | 1,234,567.89       | no tooltip                               |
        | null (any type)              | null                     | italic "NULL" on hatched bg | no tooltip                      |

  Rule: Sidebar table list is searchable

    Scenario: Filter table list by typing
      Given 5 tables are loaded in the sidebar
      When I type "act" in the table search input
      Then only tables with "act" in their display name are shown (e.g., "Activity Log")

    Scenario: Clearing search restores full list
      Given the table search input contains "act"
      When I clear the input
      Then all 5 tables are shown

  Rule: Status bar shows correct pagination info

    Scenario: Status bar reflects current page
      Given the "Activity Log" table has 200 rows with page_size 50
      When page 1 is loaded
      Then the status bar shows "Showing 1 - 50 of 200"
      And "1 of 4" page indicator is visible

    Scenario: Pagination buttons at boundaries
      Given I am on page 1 of 4
      Then the "Previous" button is disabled
      And the "Next" button is enabled

  Rule: Tool strip provides data manipulation controls

    Scenario: Tool strip renders with filter toggle
      When the app loads
      Then a vertical tool strip appears between the sidebar and grid
      And it contains a funnel icon button with tooltip "Toggle filters"

    Scenario: Tool strip sort indicator shows current state
      Given the "Name" column is sorted ascending
      Then the tool strip shows a sort indicator reflecting "Name asc"
```

---

## Task Breakdown

| ID | Task | Issue | Branch | Priority | Dependencies | Status |
|:---|:-----|:------|:-------|:---------|:-------------|:-------|
| T9 | RevoGrid dumb renderer: disable built-in interactions, CSS height cascade, verify virtual scrolling | #17 | `feature/17-*` | High | None | pending |
| T9.1 | Add `SortState`, `FilterState` types to `types.ts` | #17 | `feature/17-*` | High | None | pending |
| T9.2 | Create `ToolStrip.svelte` component shell with layout slots | — | `feature/17-*` | High | None | pending |
| T9.3 | Wire ToolStrip into `App.svelte` layout (sidebar | toolstrip | main) | — | `feature/17-*` | High | T9.2 | pending |
| T11 | Click-to-sort: header click handler, three-state cycle, chevron indicators | #18 | `feature/18-*` | High | T9 | pending |
| T11.1 | Wire sort state in App.svelte, pass to DataGrid and ToolStrip | #18 | `feature/18-*` | High | T9.3, T11 | pending |
| T12 | Per-column filter UI: inputs inside headers, toggle visibility | #19 | `feature/19-*` | High | T11 | pending |
| T12.1 | Filter debounce logic (300ms) and API wiring in App.svelte | #19 | `feature/19-*` | High | T12 | pending |
| T12.2 | Active filter count badge on ToolStrip funnel icon | #19 | `feature/19-*` | High | T12.1 | pending |
| T12.3 | Add per-column filter support to `mock.ts` | #19 | `feature/19-*` | Med | T12 | pending |
| T13 | Cell formatters: timestamp, boolean badge, number locale, NULL hatched | #20 | `feature/20-*` | High | T9 | pending |
| T8 | Sidebar table search: text input + client-side filter in TableList | #16 | `feature/16-*` | Med | None | pending |
| T16 | Status bar verification: confirm acceptance criteria met | #21 | `feature/21-*` | Low | T9 | pending |

---

## Exit Criteria

- [ ] All Must-Have acceptance scenarios pass manually in `just dev-mock` mode
- [ ] Grid loads first page in <2 seconds on local PostgreSQL (`just dev`)
- [ ] Virtual scrolling confirmed active (DOM element count << total rows)
- [ ] Sort, filter, and pagination work end-to-end against live PostgreSQL
- [ ] No SQL injection vectors via sort_column, filter params (server-side validation already in place — verify not bypassed)
- [ ] No regressions on existing sidebar, toolbar, status bar functionality
- [ ] Single binary build (`just build`) succeeds with embedded assets
- [ ] `cargo test` and `npm run check` pass
- [ ] `npm run test` passes (existing + any new unit tests)

---

## References

- Epic issue: [#3](https://github.com/Kiriketsuki/seeKi/issues/3)
- Epic PR: [#29](https://github.com/Kiriketsuki/seeKi/pull/29)
- Feature issues: [#16](https://github.com/Kiriketsuki/seeKi/issues/16), [#17](https://github.com/Kiriketsuki/seeKi/issues/17), [#18](https://github.com/Kiriketsuki/seeKi/issues/18), [#19](https://github.com/Kiriketsuki/seeKi/issues/19), [#20](https://github.com/Kiriketsuki/seeKi/issues/20), [#21](https://github.com/Kiriketsuki/seeKi/issues/21)
- Backend API: `src/api/mod.rs` — sort/filter/pagination params already implemented
- MVP spec: `seeki-mvp-spec.md`
- Brainstorming session: `.brainstorm/` directory in repo root

---

*Authored by: Clault KiperS 4.6*
