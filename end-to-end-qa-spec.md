# Feature: End-to-End QA

## Overview

**User Story**: As a SeeKi developer, I want comprehensive E2E tests and a manual QA checklist so that I can verify all MVP features work correctly against a real database and catch regressions before release.

**Problem**: SeeKi is feature-complete across 5 epics but has only unit-level test coverage (~20 backend config tests, ~100 frontend cell-formatting tests). There is no integration or E2E testing against a real database, no verification that the setup wizard, data grid, toolbar, and navigation work end-to-end through the browser.

**Out of Scope**:
- CI/GitHub Actions integration (local-only for now)
- Performance benchmarking / load testing
- Cross-browser testing matrix (Chrome only for Playwright)
- Backend unit test expansion (existing coverage is sufficient)
- Mock API tests (all E2E tests run against real Postgres)

---

## Success Condition

> This feature is complete when `just test-e2e` runs a Playwright suite against the real MEC-Miki PostgreSQL database covering setup wizard, data grid, toolbar, navigation, and error states, all tests pass, and a manual QA checklist document covers visual fidelity, accessibility, and exploratory edge cases.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | None identified | -- | [x] |

---

## Scope

### Must-Have

- **Playwright infrastructure**: `playwright.config.ts`, `globalSetup` (copy `seeki.toml.test` -> `seeki.toml`, start Rust binary), `globalTeardown` (kill server, clean up config), `.env.test` for credentials (gitignored), `seeki.toml.test` checked in without credentials: [passes when `just test-e2e` boots the app and runs all specs]
- **Setup wizard E2E** (`setup-wizard.spec.ts`): connection test success/failure, SSH toggle reveals fields, step navigation (forward/back), save flow triggers hot-reload, polling transitions to normal mode: [passes when wizard completes end-to-end and grid loads after save]
- **Data grid E2E** (`data-grid.spec.ts`): initial load shows vehicle_logs rows, sort cycling (asc/desc/unsorted) with chevron indicators, per-column filter with debounce, multi-filter AND logic, global search, pagination (page forward/back, boundary states), cell formatting (timestamps as "MMM D, H:MM AM/PM", booleans as Yes/No badges, numbers right-aligned, NULLs with distinct styling): [passes when all grid interactions produce correct DOM state against MEC-Miki vehicle_logs and SOC tables]
- **Toolbar E2E** (`toolbar.spec.ts`): search toggle via icon click and Ctrl+K, column visibility popover open/close, column hide/show reflected in grid, localStorage persistence survives page reload, CSV export triggers download with correct Content-Disposition: [passes when all toolbar actions produce expected results]
- **Navigation E2E** (`navigation.spec.ts`): default table loads on app start, sidebar table switching loads new data, sidebar search filters table list, sidebar collapse/expand: [passes when navigation between tables works correctly]
- **Error states E2E** (`error-states.spec.ts`): invalid table name returns 404 page or message, SQL injection attempt in filter/search params is rejected (400 or sanitized): [passes when error scenarios produce safe, user-friendly responses]
- **Manual QA checklist** (`QA-CHECKLIST.md`): structured document covering visual fidelity (glassmorphism, CSS tokens, color badges, hatched NULLs), accessibility (keyboard nav, ARIA labels, screen reader basics), SSH tunnel scenarios, exploratory edge cases (long values, unicode, special characters in column names), UX polish (animations, transitions): [passes when checklist is written and actionable]
- **Justfile targets**: `just test-e2e` (build + run Playwright) and `just test-e2e-ui` (Playwright UI mode): [passes when both commands work]

### Should-Have
- **Test data assertions against known MEC-Miki state**: Assertions reference specific vehicle_logs and SOC table structures (column names, expected data types) rather than generic checks

### Nice-to-Have
- **Playwright screenshot comparison**: Visual regression snapshots for key states (grid loaded, wizard step 1, column dropdown open)

---

## Technical Plan

**Affected Components**:

| File | Action | Description |
|:-----|:-------|:------------|
| `frontend/playwright.config.ts` | New | Playwright config: baseURL `http://127.0.0.1:3141`, two projects (setup / normal), timeout settings |
| `frontend/e2e/global-setup.ts` | New | Copy `seeki.toml.test` -> CWD `seeki.toml`, start `cargo run`, wait for `/api/status` healthy |
| `frontend/e2e/global-teardown.ts` | New | Kill server process, remove copied `seeki.toml` |
| `frontend/e2e/setup-wizard.spec.ts` | New | Setup wizard flow tests (connection, steps, save, hot-reload) — runs in "setup" Playwright project with no seeki.toml |
| `frontend/e2e/data-grid.spec.ts` | New | Grid tests (load, sort, filter, search, pagination, formatting) against vehicle_logs and SOC |
| `frontend/e2e/toolbar.spec.ts` | New | Toolbar tests (search, column visibility, CSV export) |
| `frontend/e2e/navigation.spec.ts` | New | Sidebar and table switching tests |
| `frontend/e2e/error-states.spec.ts` | New | Error handling and security tests |
| `frontend/e2e/fixtures.ts` | New | Shared test fixtures (page setup, common selectors, test helpers) |
| `seeki.toml.test` | New | Test config pointing at MEC-Miki Postgres (credentials via env vars from `.env.test`) |
| `.env.test.example` | New | Example env file showing required variables (DB password etc.) |
| `QA-CHECKLIST.md` | New | Manual QA checklist for visual/accessibility/exploratory testing |
| `Justfile` | Modify | Add `test-e2e` and `test-e2e-ui` targets |
| `.gitignore` | Modify | Add `.env.test`, `test-results/`, `playwright-report/` |

**Test Project Isolation**:

Two Playwright projects handle the config state split:
- `setup` project: its `setup` hook removes `seeki.toml` before running `setup-wizard.spec.ts` — tests start from setup mode
- `normal` project: its `setup` hook copies `seeki.toml.test` to `seeki.toml` before all other specs — tests start from normal mode

**Data Model Changes**: None.

**API Contracts**: No new endpoints. Tests exercise existing endpoints:
- `GET /api/status` -- mode check (setup vs normal)
- `GET /api/tables` -- table list
- `GET /api/tables/{table}/columns` -- column metadata
- `GET /api/tables/{table}/rows?page=&sort_column=&sort_direction=&search=&filter.*=` -- paginated rows
- `GET /api/export/{table}/csv?...` -- CSV download
- `POST /api/setup/test-connection` -- connection validation
- `POST /api/setup/save` -- config save + hot-reload

**Dependencies**:
- `@playwright/test` (npm dev dependency)
- MEC-Miki PostgreSQL running locally (vehicle_logs and SOC tables as primary fixtures)
- Rust toolchain (cargo build/run)

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| MEC-Miki data changes between test runs causing flaky assertions | Medium | Assert on structure (column names, types, row count > 0) not specific row values; use relative assertions ("contains", "greater than") |
| globalSetup cargo build slow on first run | Low | `just test-e2e` runs `cargo build` first; globalSetup only starts the pre-built binary |
| Port 3141 already in use from a dev session | Low | globalSetup checks port availability; fails fast with clear message |

---

## Acceptance Scenarios

```gherkin
Feature: End-to-End QA
  As a SeeKi developer
  I want automated E2E tests and a manual QA checklist
  So that I can verify MVP features work against a real database

  Background:
    Given the MEC-Miki PostgreSQL database is running locally
    And the database contains vehicle_logs and SOC tables with data

  Rule: Playwright infrastructure boots the app and runs tests

    Scenario: Test suite starts and connects to real database
      Given seeki.toml.test is configured for MEC-Miki
      When I run "just test-e2e"
      Then the Rust binary starts on port 3141
      And Playwright tests execute against the running server
      And the server is stopped after tests complete

  Rule: Setup wizard completes end-to-end

    Scenario: Successful connection and save
      Given no seeki.toml exists (setup mode)
      When I enter the MEC-Miki connection string in Step 1
      And click "Test Connection"
      Then a success message appears with the table list
      When I select vehicle_logs and SOC tables in Step 2
      And set a title in Step 3
      And click "Save" in Step 4
      Then the app transitions to normal mode with the grid visible

    Scenario: Failed connection shows descriptive error
      Given no seeki.toml exists (setup mode)
      When I enter an invalid connection string
      And click "Test Connection"
      Then an error message appears identifying the failure source
      And the wizard remains on Step 1

    Scenario: SSH toggle reveals SSH config fields
      Given no seeki.toml exists (setup mode)
      And I am on Step 1 of the wizard
      When I toggle the SSH tunnel switch on
      Then SSH host, port, username, and auth method fields appear
      When I toggle it off
      Then the SSH fields are hidden

  Rule: Data grid displays and interacts correctly

    Scenario: Grid loads with vehicle_logs data
      Given the app is in normal mode with vehicle_logs selected
      Then the grid displays rows with correct column headers
      And the status bar shows "Showing 1-50 of N"

    Scenario: Sort cycling works
      Given the grid is loaded with vehicle_logs
      When I click a column header once
      Then rows are sorted ascending with an up chevron visible
      When I click the same header again
      Then rows are sorted descending with a down chevron visible
      When I click the same header a third time
      Then sort is cleared and no chevron is visible

    Scenario: Per-column filter narrows results
      Given the grid is loaded
      When I toggle filters visible
      And type a value in a column filter input
      Then the grid updates after 300ms debounce
      And the status bar shows a reduced row count

    Scenario: Multiple column filters AND together
      Given the grid is loaded with filters visible
      When I type a value in column filter A
      And type a value in column filter B
      Then only rows matching both filters are shown

    Scenario: Global search filters rows across text columns
      Given the grid is loaded
      When I activate global search and type a term
      Then only rows containing that term in any text column are shown

    Scenario: Cell formatting matches display rules
      Given the grid is loaded with vehicle_logs data
      Then timestamp columns display as "MMM D, H:MM AM/PM" format
      And boolean columns display as "Yes" or "No" badges
      And numeric columns are right-aligned
      And NULL cells show italic "NULL" with distinct hatched styling

    Scenario: Pagination navigates between pages
      Given the grid is loaded with more than 50 rows
      When I click the next page button
      Then the status bar updates to "Showing 51-100 of N"
      When I click the previous page button
      Then the status bar returns to "Showing 1-50 of N"

  Rule: Toolbar controls work correctly

    Scenario: Global search toggle via icon and keyboard shortcut
      Given the grid is loaded
      When I click the search icon in the toolbar
      Then a search bar appears below the table header
      When I press Escape
      Then the search bar collapses
      When I press Ctrl+K
      Then the search bar appears again

    Scenario: Column visibility toggle persists across reload
      Given the grid is loaded
      When I click the columns icon
      Then a popover appears with checkboxes for each column
      When I uncheck a column
      Then that column disappears from the grid
      When I reload the page
      Then the column remains hidden (localStorage persistence)

    Scenario: CSV export reflects active search
      Given the grid is loaded with a search term active
      When I click the export icon
      Then a file download is triggered with Content-Disposition: attachment
      And the filename ends in .csv

  Rule: Navigation between tables works

    Scenario: Switching tables loads new data
      Given the grid is showing vehicle_logs
      When I click SOC in the sidebar
      Then the grid reloads with SOC columns and rows
      And the table header updates to show the SOC display name

    Scenario: Sidebar search filters table list
      Given the sidebar shows multiple tables
      When I type "veh" in the sidebar search
      Then only tables whose names contain "veh" are visible

    Scenario: Sidebar collapse and expand
      Given the sidebar is expanded
      When I click the collapse toggle
      Then the sidebar collapses and the grid expands
      When I click the expand toggle
      Then the sidebar returns to its original width

  Rule: Error states are handled safely

    Scenario: SQL injection attempt in filter is rejected safely
      Given the grid is loaded
      When I type "'; DROP TABLE vehicle_logs--" in a filter input
      Then the app does not crash
      And no SQL error message is exposed to the user
      And the grid either shows zero results or an empty state

    Scenario: Request for non-existent table returns safe error
      Given the app is running
      When a request is made directly to /api/tables/nonexistent_table_xyz/rows
      Then a 404 response is returned
      And the response body does not contain a database stack trace
```

---

## Task Breakdown

| ID | Task | Priority | Dependencies | Status |
|:---|:-----|:---------|:-------------|:-------|
| T1 | Playwright infrastructure: install `@playwright/test`, create `playwright.config.ts` with two projects (setup/normal), `global-setup.ts`, `global-teardown.ts`, `fixtures.ts` | High | None | pending |
| T1.1 | Create `seeki.toml.test` and `.env.test.example` with MEC-Miki connection config (credentials via env vars) | High | None | pending |
| T1.2 | Add `test-e2e` and `test-e2e-ui` targets to Justfile | High | T1 | pending |
| T1.3 | Update `.gitignore` with `.env.test`, `test-results/`, `playwright-report/` | High | T1 | pending |
| T2 | Setup wizard E2E (`setup-wizard.spec.ts`): connection success/failure, SSH toggle, step navigation, save + hot-reload polling | High | T1 | pending |
| T3 | Data grid E2E (`data-grid.spec.ts`): load, sort cycling, per-column filter, multi-filter AND, search, pagination, cell formatting against vehicle_logs and SOC | High | T1 | pending |
| T4 | Toolbar E2E (`toolbar.spec.ts`): search toggle + Ctrl+K, column visibility popover, hide/show + localStorage, CSV export download | High | T1 | pending |
| T5 | Navigation E2E (`navigation.spec.ts`): default table load, sidebar table switching, sidebar search, collapse/expand | Med | T1 | pending |
| T6 | Error states E2E (`error-states.spec.ts`): SQL injection attempt, 404 for missing table | Med | T1 | pending |
| T7 | Manual QA checklist (`QA-CHECKLIST.md`): visual fidelity, accessibility, SSH tunnel scenarios, exploratory edge cases, UX polish | Med | None | pending |
| T8 | Full suite green run: `just test-e2e` passes + manual checklist walked through | High | T2-T7 | pending |

---

## Exit Criteria

- [ ] `just test-e2e` runs all Playwright specs to green against MEC-Miki
- [ ] Setup wizard and data grid have comprehensive automated coverage (highest-risk areas)
- [ ] No regressions on existing unit tests (`cargo test` and `npm test` still pass)
- [ ] SQL injection test confirms safe handling of malicious input
- [ ] Manual QA checklist written and actionable
- [ ] Manual QA checklist walked through at least once with findings documented

---

## References

- Related specs: `seeki-mvp-spec.md`, `first-run-setup-wizard-spec.md`, `data-grid-table-navigation-spec.md`, `toolbar-column-export-spec.md`
- PR: #32 (epic: End-to-End QA)
- Sub-issues: #26 (E2E acceptance test suite)

---
*Authored by: Clault KiperS 4.6*
