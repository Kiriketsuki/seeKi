# Feature: SeeKi MVP

## Overview

**User Story**: As a non-technical team member (simulation analyst, data analyst, or engineer), I want to browse our PostgreSQL database through a clean spreadsheet-like interface so that I can look up vehicle data, inspect telemetry, and generate reports without needing SQL knowledge or DBA tools.

**Problem**: The team currently relies on DBeaver to query the AutoConnect database. DBeaver exposes SQL editors, commit/rollback controls, schema diagrams, and developer-centric jargon that intimidate non-technical users. This creates a bottleneck where engineers are repeatedly asked to pull data for analysts and ops staff.

**Out of Scope**:
- Write operations (insert, update, delete) -- SeeKi is read-only by design
- Authentication/authorization -- MVP targets a trusted internal network
- SQLite or MySQL support -- PostgreSQL only for MVP
- Mobile-responsive layout -- desktop-first for MVP
- Real-time data streaming / live updates
- Saved views or bookmarks
- Row detail side panel
- Multi-column sort panel

---

## Success Condition

> This feature is complete when a non-technical user can run the SeeKi binary, complete a first-run setup wizard in the browser, and then browse, search, filter, sort, and export CSV data from any exposed PostgreSQL table through a glassmorphic spreadsheet-like UI with no visible SQL.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | RevoGrid Svelte 5 integration maturity -- fallback to AG Grid Community if blockers arise | Brainstorm | [ ] |
| 2 | Glassmorphism backdrop-filter performance on large grids -- needs profiling during implementation | Brainstorm | [ ] |
| 3 | Column name auto-heuristic edge cases (e.g. `posn_lat` -> what?) -- tune during implementation | Brainstorm | [ ] |

---

## Scope

### Must-Have

- **Collapsible sidebar with table list**: searchable table list showing friendly names and row count badges; collapsible to maximize grid space. Acceptance: sidebar renders all exposed tables, filters by substring, shows estimated row counts.
- **Data grid with virtual scrolling**: RevoGrid-powered grid with server-side pagination for 500K+ row tables. Acceptance: grid loads first 50 rows in under 2 seconds, virtual scroll renders only visible rows.
- **Click-to-sort columns**: click column header to cycle ascending -> descending -> unsorted. Acceptance: sort indicator (SVG chevron) appears on active column; server-side sort for large tables.
- **Global search**: search bar in toolbar that filters across all text columns. Acceptance: debounced input (300ms) triggers server-side ILIKE search; row count updates to show matched rows.
- **Per-column text filter**: filter input in each column header for exact column filtering. Acceptance: typing in a column filter adds a WHERE clause for that column; multiple column filters combine with AND.
- **Column hide/show**: "Columns" dropdown in toolbar to toggle column visibility. Acceptance: hidden columns are removed from the grid; preference persists in localStorage.
- **CSV export**: "Export" button exports current view (with active filters/sort) to CSV. Acceptance: streams CSV download with correct headers; respects display names, not raw column names.
- **Auto display formatting**: timestamps show as "Apr 7, 3:58 PM" (tooltip shows full value), numbers right-aligned in monospace, booleans render as read-only checkboxes, NULLs show as gray italic "null". Acceptance: all five data types render distinctly without configuration.
- **Column header casualification**: auto-heuristic converts snake_case to Title Case, drops `_id` suffixes; config overrides in `seeki.toml` take precedence. Acceptance: `supervisor_id` displays as "Supervisor" by default; `[display.columns]` overrides apply on reload.
- **First-run setup wizard**: when no `seeki.toml` exists, the browser shows a setup flow: paste connection string, test connection, select tables to expose, set app title, save config. Acceptance: wizard writes valid `seeki.toml`; app redirects to grid view after save.
- **Table include/exclude config**: `seeki.toml` supports `[tables]` section with `include` or `exclude` list. Acceptance: only configured tables appear in the sidebar.
- **App branding**: configurable app title and subtitle in `seeki.toml`, displayed in sidebar header. "Powered by SeeKi" attribution link always present in sidebar footer. Acceptance: custom title renders; attribution link points to GitHub repo.

### Should-Have

- **Filter bar with typed operators**: Airtable-style `[Column] [Operator] [Value]` chip builder with type-aware operators (text: contains/equals, numbers: >/</between, dates: before/after). First post-MVP feature.
- **Pagination controls**: page number display and prev/next navigation in status bar alongside virtual scroll.

### Nice-to-Have

- **Dark mode**: theme toggle with alternate palette
- **Column resize and reorder**: drag handles on headers, persist to localStorage
- **Row detail side panel**: click a row to see all fields in a slide-out panel
- **Saved views**: per-user view configurations (visible columns, sort, filters)
- **JSON cell expand**: click to expand JSON columns with syntax highlighting
- **Export filtered/selected rows**: partial export of visible/selected data only

---

## Technical Plan

**Affected Components**:

| Layer | Files | Changes |
|:------|:------|:--------|
| Config | `src/config.rs` | Add `[tables]`, `[display]`, `[branding]` sections to AppConfig |
| API | `src/api/mod.rs` | Add `/api/export/{table}/csv`, `/api/config/display`, `/api/setup/*` routes; add per-column filter params to `/api/tables/{table}/rows` |
| DB | `src/db/mod.rs`, `src/db/postgres.rs` | Add per-column filter support to `query_rows`; add `test_connection` function for setup wizard |
| Main | `src/main.rs` | Conditionally serve setup wizard when no config exists; serve embedded frontend assets |
| Frontend | `frontend/` (new) | Svelte 5 + Vite project with RevoGrid, sidebar, toolbar, status bar |
| Embed | `Cargo.toml` | Configure rust-embed to serve `frontend/dist/` |

**Data Model Changes**: None -- SeeKi is read-only and introspects existing schemas.

**API Contracts**:

| Method | Path | Description | Response |
|:-------|:-----|:------------|:---------|
| `GET` | `/api/tables` | List exposed tables | `{ tables: [{ name, row_count_estimate }] }` |
| `GET` | `/api/tables/{table}/columns` | Column metadata | `{ columns: [{ name, data_type, display_type, display_name, is_nullable, is_primary_key }] }` |
| `GET` | `/api/tables/{table}/rows` | Paginated rows | `{ columns, rows, total_rows, page, page_size }` |
| | | Query params: `page`, `page_size`, `sort_column`, `sort_direction`, `search`, `filter.{column}` | |
| `GET` | `/api/export/{table}/csv` | Stream CSV export | `text/csv` attachment with display names as headers |
| `GET` | `/api/config/display` | Display config | `{ columns: { "table.col": "Display Name" }, branding: { title, subtitle } }` |
| `POST` | `/api/setup/test-connection` | Test DB connection | `{ success: bool, error?: string, tables?: string[] }` |
| `POST` | `/api/setup/save` | Write seeki.toml | `{ success: bool }` |

**Dependencies**:
- `@aspect-ui/revogrid-svelte` or `@revolist/svelte-datagrid` -- RevoGrid Svelte wrapper
- `lucide-svelte` -- SVG icon library
- `csv` crate (Rust) -- CSV streaming export

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| RevoGrid Svelte 5 bindings immature | Medium | Fallback to AG Grid Community (MIT) or use RevoGrid as raw web component |
| backdrop-filter performance on large grids | Low | Limit glass effects to sidebar/toolbar; grid card uses simpler semi-transparent background |
| rust-embed + Vite dev workflow friction | Low | Proxy Vite dev server to axum in dev; only embed for release builds |
| Column name heuristic produces bad guesses | Medium | Keep heuristic simple (snake_case -> Title Case); config overrides are the escape hatch |
| Setup wizard writes config while server is running | Low | Server re-reads config on setup save; no hot-reload needed since it's a first-run flow |

---

## Acceptance Scenarios

```gherkin
Feature: SeeKi MVP
  As a non-technical team member
  I want to browse our database through a spreadsheet-like UI
  So that I can look up data and generate reports without SQL

  Background:
    Given SeeKi is running with a valid seeki.toml pointing to a PostgreSQL database
    And the database contains tables "vehicles", "vehicles_log" (427K rows), and "events"

  Rule: Table navigation via sidebar

    Scenario: Browse available tables
      Given the user opens SeeKi in a browser
      When the page loads
      Then the sidebar shows "Vehicles", "Vehicles Log", and "Events" with row count badges
      And "Vehicles Log" is selected by default (first table alphabetically or by config order)

    Scenario: Search tables in sidebar
      Given the sidebar shows 20+ tables
      When the user types "veh" in the table search input
      Then only tables containing "veh" in their name are shown

    Scenario: Switch between tables
      Given "Vehicles Log" is currently displayed
      When the user clicks "Events" in the sidebar
      Then the grid reloads with data from the "events" table
      And the toolbar title updates to "Events"

  Rule: Data grid with virtual scrolling and pagination

    Scenario: Load large table
      Given the user selects "Vehicles Log" (427K rows)
      When the grid loads
      Then the first 50 rows are displayed within 2 seconds
      And the status bar shows "Showing 1 - 50 of 427,229"

    Scenario: Navigate pages
      Given the grid shows page 1 of "Vehicles Log"
      When the user clicks the "Next" pagination button
      Then page 2 loads (rows 51-100)
      And the status bar updates to "Showing 51 - 100 of 427,229"

  Rule: Click-to-sort

    Scenario: Sort by column ascending
      Given the grid shows "Vehicles Log" unsorted
      When the user clicks the "Vehicle" column header
      Then rows are sorted by vehicle_id ascending
      And a down-chevron icon appears on the "Vehicle" header

    Scenario: Cycle sort direction
      Given the "Vehicle" column is sorted ascending
      When the user clicks the "Vehicle" header again
      Then rows are sorted descending
      And the chevron icon flips to up-chevron
      When the user clicks the "Vehicle" header a third time
      Then the sort is removed and rows return to default order

  Rule: Global search

    Scenario: Search across text columns
      Given the grid shows "Vehicles Log"
      When the user types "ADT3-008" in the global search bar
      Then only rows where any text column contains "ADT3-008" are shown
      And the status bar updates to "Showing 1 - 50 of 12,345" (filtered count)

    Scenario: Empty search restores all rows
      Given a search filter is active showing 12,345 rows
      When the user clears the search bar
      Then all 427,229 rows are accessible again

  Rule: Per-column text filter

    Scenario: Filter by specific column
      Given the grid shows "Vehicles Log"
      When the user types "ADT3-008" in the "Vehicle" column filter input
      Then only rows where vehicle_id contains "ADT3-008" are shown

    Scenario: Combine multiple column filters
      Given the user has filtered "Vehicle" to "ADT3-008"
      When the user also types "Local" in the "Supervisor" column filter
      Then only rows matching both filters are shown (AND logic)

  Rule: Column visibility

    Scenario: Hide a column
      Given the grid shows all columns
      When the user opens the "Columns" dropdown and unchecks "Latitude"
      Then the "Latitude" column disappears from the grid

    Scenario: Column visibility persists
      Given the user has hidden "Latitude"
      When the user refreshes the page
      Then "Latitude" remains hidden (persisted in localStorage)

  Rule: CSV export

    Scenario: Export with filters applied
      Given the grid is filtered to vehicle_id = "ADT3-008" and sorted by timestamp descending
      When the user clicks "Export CSV"
      Then a CSV file downloads with display names as headers ("Vehicle", not "vehicle_id")
      And the CSV contains only the filtered rows in the current sort order

  Rule: Display formatting (casualification)

    Scenario: Timestamps display in friendly format
      Given a row has timestamp "2026-04-07 15:58:44.000 +0800"
      When the grid renders the cell
      Then it displays "Apr 7, 3:58 PM"
      And hovering shows the full timestamp as a tooltip

    Scenario: Column headers use friendly names
      Given the table has column "supervisor_id"
      And no config override exists for this column
      When the grid renders
      Then the header displays "Supervisor" (auto-heuristic: drop _id, Title Case)

    Scenario: Config overrides take precedence
      Given seeki.toml contains [display.columns] with "vehicles_log.posn_lat" = "Latitude"
      When the grid renders
      Then the header displays "Latitude" instead of the auto-heuristic result

  Rule: First-run setup wizard

    Scenario: No config triggers wizard
      Given no seeki.toml exists
      When the user opens SeeKi in a browser
      Then the setup wizard is displayed instead of the grid

    Scenario: Complete setup flow
      Given the setup wizard is displayed
      When the user pastes a valid PostgreSQL connection string
      And clicks "Test Connection"
      Then a success message appears with the list of discovered tables
      When the user selects which tables to expose and enters an app title
      And clicks "Save"
      Then seeki.toml is written to disk
      And the browser redirects to the main grid view

    Scenario: Invalid connection string
      Given the setup wizard is displayed
      When the user pastes an invalid connection string and clicks "Test Connection"
      Then an error message displays explaining the connection failure

  Rule: Table include/exclude

    Scenario: Only included tables are visible
      Given seeki.toml contains [tables] include = ["vehicles", "vehicles_log"]
      When the user opens SeeKi
      Then only "Vehicles" and "Vehicles Log" appear in the sidebar
      And "Events" is not shown

  Rule: App branding

    Scenario: Custom branding displays
      Given seeki.toml contains [branding] title = "AutoConnect" and subtitle = "Fleet Telemetry"
      When the user opens SeeKi
      Then the sidebar header shows "AutoConnect" with subtitle "Fleet Telemetry"
      And the sidebar footer shows "Powered by SeeKi" with a link to the GitHub repo
```

---

## Task Breakdown

| ID | Task | Priority | Dependencies | Status |
|:---|:-----|:---------|:-------------|:-------|
| T1 | **Backend: Config extensions** -- Add `[tables]`, `[display.columns]`, `[branding]` sections to `AppConfig` in `config.rs`; update `seeki.toml.example` | High | None | pending |
| T2 | **Backend: Per-column filter support** -- Add `filter.{column}` query params to `query_rows` in `postgres.rs` and the `/api/tables/{table}/rows` handler | High | None | pending |
| T3 | **Backend: Display config endpoint** -- Add `GET /api/config/display` returning column display names, branding, and table list config | High | T1 | pending |
| T4 | **Backend: CSV export endpoint** -- Add `GET /api/export/{table}/csv` that streams filtered/sorted data as CSV with display name headers | High | T2, T3 | pending |
| T5 | **Backend: Setup wizard endpoints** -- Add `POST /api/setup/test-connection` and `POST /api/setup/save`; conditionally serve wizard when no config exists | High | T1 | pending |
| T6 | **Backend: Table filtering** -- Apply `include`/`exclude` from config to `list_tables`; add `display_name` field to `ColumnInfo` | High | T1 | pending |
| T7 | **Frontend: Scaffold Svelte 5 + Vite project** -- Initialize `frontend/` with Vite, Svelte 5, RevoGrid, Lucide icons; configure dev proxy to axum | High | None | pending |
| T8 | **Frontend: Sidebar component** -- Collapsible sidebar with table list, search filter, row counts, branding, attribution footer | High | T7, T3 | pending |
| T9 | **Frontend: Data grid component** -- RevoGrid integration with virtual scrolling, server-side pagination, column headers with casualified names | High | T7, T3 | pending |
| T10 | **Frontend: Toolbar component** -- Global search, "Columns" visibility dropdown, "Export CSV" button | High | T7 | pending |
| T11 | **Frontend: Click-to-sort** -- Wire column header clicks to sort API params; render sort chevron SVG | High | T9 | pending |
| T12 | **Frontend: Per-column filters** -- Filter inputs in column headers; debounced API calls with `filter.{column}` params | High | T9, T2 | pending |
| T13 | **Frontend: Display formatting** -- Cell renderers for timestamps, numbers, booleans, NULLs; tooltip for full values | High | T9 | pending |
| T14 | **Frontend: Column hide/show** -- "Columns" dropdown toggles visibility; persist to localStorage | Med | T10, T9 | pending |
| T15 | **Frontend: CSV export** -- Wire Export button to `/api/export/{table}/csv` with current filter/sort params; trigger browser download | Med | T4, T10 | pending |
| T16 | **Frontend: Status bar** -- Row count, page info, prev/next pagination buttons | Med | T9 | pending |
| T17 | **Frontend: Setup wizard** -- Multi-step wizard (connection string -> test -> table select -> branding -> save); redirect to grid on completion | High | T7, T5 | pending |
| T18 | **Frontend: Glassmorphism theme** -- CSS custom properties for palette (#2F4858, #00A9A5, #F5F0EB); backdrop-filter on sidebar, toolbar, grid card | Med | T7 | pending |
| T19 | **Integration: rust-embed setup** -- Configure rust-embed to serve `frontend/dist/`; fallback to index.html for SPA routing | High | T7 | pending |
| T20 | **QA: End-to-end testing** -- Verify all acceptance scenarios against a test PostgreSQL database | High | T8-T18 | pending |

---

## Exit Criteria

- [ ] All Must-Have acceptance scenarios pass manually against a PostgreSQL database with 20+ tables and 500K+ rows
- [ ] First-run setup wizard successfully writes a valid seeki.toml and redirects to grid
- [ ] CSV export produces valid CSV with display name headers and filtered data
- [ ] Virtual scrolling renders smoothly for tables with 100K+ rows
- [ ] No SQL injection possible via table names, column names, filter values, or search terms
- [ ] Single binary build (cargo build --release) embeds all frontend assets and runs without Node.js
- [ ] Grid loads first page in under 2 seconds on a local PostgreSQL connection
- [ ] Glassmorphism renders correctly in Chrome, Firefox, and Safari (latest versions)

---

## References

- Competitive analysis: `docs/research/01-competitive-analysis.md`
- Tech stack research: `docs/research/02-tech-stack.md`
- UX patterns research: `docs/research/03-ux-patterns.md`
- Brainstorm mockups: `.brainstorm/326657-1775548709/content/` (visual-style-v4.html is latest approved direction)

---

*Authored by: Clault KiperS 4.6*
