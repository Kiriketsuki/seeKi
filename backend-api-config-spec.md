# Feature: Backend API & Config

## Overview

**User Story**: As a non-technical team member (simulation analyst, data analyst, or engineer), I want SeeKi's backend to support configurable table exposure, display name overrides, per-column filtering, CSV export, and a first-run setup wizard so that the frontend can deliver a complete spreadsheet-like browsing experience without any SQL knowledge.

**Problem**: The current backend scaffold provides only basic table listing, column metadata, and paginated row fetching. It lacks the configuration, filtering, export, and setup capabilities required by the MVP spec.

**Out of Scope**:
- Frontend implementation (covered by Epics 2-5)
- Authentication/authorization (deferred -- MVP targets trusted internal network)
- SQLite or MySQL support (PostgreSQL only for MVP)
- Write operations (SeeKi is read-only by design)
- Real-time data streaming / live updates
- Saved views or bookmarks

---

## Success Condition

> This feature is complete when all six API contracts (config display, per-column filters, CSV export, setup test-connection, setup save, table filtering) are implemented, return correct JSON/CSV shapes, maintain SQL injection prevention, and the server boots in setup-only mode when no config file exists.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | Column name auto-heuristic edge cases (e.g. `posn_lat` -> what?) -- tune during implementation | Brainstorm | [ ] |

---

## Scope

### Must-Have

- **Config extensions** (#7/T1): Add `[tables]`, `[display.columns]`, and `[branding]` sections to `AppConfig`. All sections optional with `#[serde(default)]`. `TablesConfig` has `include: Option<Vec<String>>` and `exclude: Option<Vec<String>>` -- both allowed simultaneously (include first, then subtract exclude). Update `seeki.toml.example`. Acceptance: missing sections parse without error; both include+exclude coexist correctly; config round-trips.

- **Per-column filter support** (#8/T2): Accept `filter.{column_name}=value` query params on `GET /api/tables/{table}/rows`. Parse via `HashMap<String, String>` extraction with `filter.` prefix. Validate column names against table schema. Generate parameterized `WHERE col ILIKE $N` clauses combined with AND. Acceptance: single filter returns matching rows; multiple filters AND together; invalid column names return 400; filter values are parameterized (no SQL injection).

- **Display config endpoint** (#9/T3): Add `GET /api/config/display` returning column display name overrides, branding info. Standalone `display_name(table, column, config)` utility function in `config.rs` -- DB layer untouched. Heuristic: snake_case to Title Case, drop `_id` suffix. Config overrides take precedence. Acceptance: endpoint returns correct JSON shape; `supervisor_id` casualifies to "Supervisor"; config override `"vehicles_log.posn_lat" = "Latitude"` takes precedence.

- **CSV export endpoint** (#10/T4): Add `GET /api/export/{table}/csv` accepting same filter/sort params as the rows endpoint. Use SQLx cursor stream (`fetch()`) mapped to CSV row bytes via `csv` crate, streamed through `Body::from_stream`. Display names as CSV headers. `Content-Disposition: attachment; filename="{table}.csv"`. Acceptance: downloads valid CSV with display name headers; respects active filters and sort; streams 500K+ rows without OOM.

- **Setup wizard backend endpoints** (#11/T5): Two-mode server architecture. `main()` checks for config file: no config boots setup-only server (`/api/setup/*` + static assets); config exists boots normal server. `POST /api/setup/test-connection` accepts `{ url }`, returns `{ success, error?, tables? }`. `POST /api/setup/save` accepts config payload, writes template-based `seeki.toml` to CWD with human-readable comments. Frontend shows "Configuration saved -- please refresh." Acceptance: valid connection returns success + table list; invalid connection returns descriptive error; save writes parseable `seeki.toml`; setup-only server boots when no config; normal server boots when config exists.

- **Table include/exclude filtering** (#12/T6): Filter `list_tables` result through `TablesConfig`. Logic: start with all tables from DB, if `include` is set keep only those, if `exclude` is set remove those. Add `display_name` field derivation at API layer (not in DB layer). Acceptance: include-only shows listed tables; exclude-only hides listed; both together applies include then exclude.

- **Error handling** (cross-cutting): Replace `AppError` with `thiserror`-based enum: `BadRequest(String)` -> 400, `NotFound(String)` -> 404, `Internal(anyhow::Error)` -> 500. Existing `From<anyhow::Error>` preserved. Acceptance: bad requests return 400; not-found returns 404; internal errors return 500 with no sensitive details leaked.

### Should-Have

- Pagination controls metadata in row query response (total pages, has_next, has_prev)

### Nice-to-Have

- Typed filter operators (>, <, between, before/after) beyond ILIKE -- first post-MVP feature
- Connection pool health check endpoint

---

## Technical Plan

**Affected Components**:

| Layer | Files | Changes |
|:------|:------|:--------|
| Config | `src/config.rs` | Add `TablesConfig`, `DisplayConfig`, `BrandingConfig` structs; add `display_name()` utility function |
| API | `src/api/mod.rs` | Add routes for display config, CSV export, setup endpoints; refine `AppError` with `thiserror`; add per-column filter parsing via HashMap |
| DB | `src/db/mod.rs` | Add `test_connection()` to `DatabasePool`; add `stream_rows()` method for cursor-based streaming |
| DB/PG | `src/db/postgres.rs` | Implement `test_connection()`; add per-column filter support to `query_rows`; add `stream_rows()` for CSV export |
| Main | `src/main.rs` | Two-mode server branch: detect config presence, boot setup-only or normal server |
| Example | `seeki.toml.example` | Add all new config sections with inline documentation |
| Deps | `Cargo.toml` | Add `csv`, `thiserror` crates |

**Data Model Changes**: None -- SeeKi is read-only and introspects existing schemas.

**API Contracts**:

| Method | Path | Description | Request | Response |
|:-------|:-----|:------------|:--------|:---------|
| `GET` | `/api/tables` | List exposed tables (filtered by config) | -- | `{ tables: [{ name, row_count_estimate }] }` |
| `GET` | `/api/tables/{table}/columns` | Column metadata | -- | `{ columns: [{ name, data_type, display_type, is_nullable, is_primary_key }] }` |
| `GET` | `/api/tables/{table}/rows` | Paginated rows with filters | `?page&page_size&sort_column&sort_direction&search&filter.{col}=val` | `{ columns, rows, total_rows, page, page_size }` |
| `GET` | `/api/export/{table}/csv` | Stream CSV export | Same query params as rows | `text/csv` attachment |
| `GET` | `/api/config/display` | Display config for frontend | -- | `{ columns: { "table.col": "Name" }, branding: { title, subtitle } }` |
| `POST` | `/api/setup/test-connection` | Test DB connection | `{ url: string }` | `{ success: bool, error?: string, tables?: string[] }` |
| `POST` | `/api/setup/save` | Write config to disk | `{ database: {...}, tables: {...}, branding: {...} }` | `{ success: bool }` |

**Dependencies**:
- `csv` crate -- CSV streaming writer
- `thiserror` crate -- typed error enum

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| SQLx cursor holds connection for entire CSV download | Low | SeeKi targets 1-2 concurrent users; pool of 5 connections is sufficient |
| Column name heuristic produces bad guesses | Medium | Keep heuristic simple (snake_case -> Title Case, drop `_id`); config overrides are the escape hatch |
| Setup wizard writes config while another process reads | Low | Setup runs only on first launch when no other instance exists |
| HashMap query param extraction misses edge cases | Low | Validate extracted column names against schema before building SQL |

---

## Acceptance Scenarios

```gherkin
Feature: Backend API & Config
  As a non-technical team member
  I want SeeKi's backend to support configuration, filtering, export, and setup
  So that the frontend can deliver a complete browsing experience

  Background:
    Given SeeKi is running with a valid seeki.toml pointing to a PostgreSQL database
    And the database contains tables "vehicles", "vehicles_log" (427K rows), and "events"

  Rule: Config extensions parse correctly

    Scenario: Missing optional sections use defaults
      Given seeki.toml contains only [server] and [database] sections
      When the application starts
      Then it boots successfully with default config (all tables exposed, no display overrides, no branding)

    Scenario: Include and exclude coexist
      Given seeki.toml contains [tables] include = ["vehicles", "vehicles_log", "events"] and exclude = ["events"]
      When the application starts
      Then the effective table list is ["vehicles", "vehicles_log"]

  Rule: Per-column filter support

    Scenario: Single column filter
      Given the user sends GET /api/tables/vehicles_log/rows?filter.vehicle_id=ADT3
      When the server processes the request
      Then only rows where vehicle_id contains "ADT3" are returned
      And the response total_rows reflects the filtered count

    Scenario: Multiple column filters combine with AND
      Given the user sends GET /api/tables/vehicles_log/rows?filter.vehicle_id=ADT3&filter.supervisor=Local
      When the server processes the request
      Then only rows matching both filters are returned

    Scenario: Invalid column name returns 400
      Given the user sends GET /api/tables/vehicles_log/rows?filter.nonexistent_col=foo
      When the server processes the request
      Then the response status is 400
      And the body contains an error message about invalid column name

    Scenario: Filter values are parameterized
      Given the user sends GET /api/tables/vehicles_log/rows?filter.vehicle_id='; DROP TABLE vehicles;--
      When the server processes the request
      Then the filter value is treated as a literal string parameter
      And no SQL injection occurs

  Rule: Display config endpoint

    Scenario: Returns correct JSON shape
      Given seeki.toml contains [display.columns] with "vehicles_log.posn_lat" = "Latitude"
      And [branding] with title = "AutoConnect" and subtitle = "Fleet Telemetry"
      When the user sends GET /api/config/display
      Then the response contains columns with "vehicles_log.posn_lat" mapped to "Latitude"
      And branding with title "AutoConnect" and subtitle "Fleet Telemetry"

    Scenario: Auto-heuristic casualifies column names
      Given no config override exists for "vehicles_log.supervisor_id"
      When the user sends GET /api/config/display
      Then "vehicles_log.supervisor_id" maps to "Supervisor"

    Scenario: Config override takes precedence over heuristic
      Given seeki.toml contains [display.columns] with "vehicles_log.supervisor_id" = "Team Lead"
      When the user sends GET /api/config/display
      Then "vehicles_log.supervisor_id" maps to "Team Lead" (not "Supervisor")

  Rule: CSV export streams correctly

    Scenario: Export with display name headers
      Given the user sends GET /api/export/vehicles/csv
      When the server processes the request
      Then the response Content-Type is "text/csv"
      And the Content-Disposition header contains 'attachment; filename="vehicles.csv"'
      And the first row contains display names (e.g. "Vehicle" not "vehicle_id")

    Scenario: Export respects filters and sort
      Given the user sends GET /api/export/vehicles_log/csv?filter.vehicle_id=ADT3&sort_column=vehicle_id&sort_direction=asc
      When the server streams the CSV
      Then only rows matching the filter are included
      And rows are sorted by vehicle_id ascending

    Scenario: Large table export does not exhaust memory
      Given vehicles_log has 427K rows
      When the user sends GET /api/export/vehicles_log/csv
      Then the full CSV streams to completion
      And server memory usage remains bounded (cursor-based streaming)

  Rule: Setup wizard backend

    Scenario: Test valid connection
      Given no seeki.toml exists and the setup-only server is running
      When the user sends POST /api/setup/test-connection with a valid PostgreSQL URL
      Then the response contains success: true and tables: ["vehicles", "vehicles_log", "events"]

    Scenario: Test invalid connection
      Given the setup-only server is running
      When the user sends POST /api/setup/test-connection with an invalid URL
      Then the response contains success: false and a descriptive error message

    Scenario: Save writes valid config
      Given the setup-only server is running
      When the user sends POST /api/setup/save with a complete config payload
      Then seeki.toml is written to CWD with correct values and human-readable comments
      And the response contains success: true

    Scenario: Setup-only mode when no config
      Given no seeki.toml exists in CWD or ~/.config/seeki/
      When the SeeKi binary starts
      Then only /api/setup/* endpoints are available
      And data endpoints return 404 or are not mounted

    Scenario: Normal mode when config exists
      Given a valid seeki.toml exists
      When the SeeKi binary starts
      Then all data endpoints are available
      And /api/setup/* endpoints are not mounted

  Rule: Table include/exclude filtering

    Scenario: Include-only filtering
      Given seeki.toml contains [tables] include = ["vehicles", "vehicles_log"]
      When the user sends GET /api/tables
      Then only "vehicles" and "vehicles_log" are returned
      And "events" is not in the response

    Scenario: Exclude-only filtering
      Given seeki.toml contains [tables] exclude = ["events"]
      When the user sends GET /api/tables
      Then "vehicles" and "vehicles_log" are returned
      And "events" is not in the response

    Scenario: Both include and exclude
      Given seeki.toml contains [tables] include = ["vehicles", "vehicles_log", "events"] and exclude = ["events"]
      When the user sends GET /api/tables
      Then only "vehicles" and "vehicles_log" are returned

  Rule: Error handling returns appropriate status codes

    Scenario: Bad request returns 400
      Given the user sends a request with an invalid column name in a filter
      When the server processes the request
      Then the response status is 400
      And the body contains a descriptive error message

    Scenario: Table not found returns 404
      Given the user sends GET /api/export/nonexistent_table/csv
      When the server processes the request
      Then the response status is 404

    Scenario: Internal error returns 500 without sensitive details
      Given the database connection drops unexpectedly
      When the user sends any data request
      Then the response status is 500
      And the error message does not contain connection strings or credentials
```

---

## Task Breakdown

| ID | Task | Issue | Priority | Dependencies | Status |
|:---|:-----|:------|:---------|:-------------|:-------|
| T0 | **Cross-cutting: Error handling** -- Add `thiserror`, replace `AppError` with status-code-aware enum | -- | High | None | pending |
| T1 | **Config extensions** -- Add `TablesConfig`, `DisplayConfig`, `BrandingConfig` to `AppConfig`; update `seeki.toml.example` | #7 | High | None | pending |
| T2 | **Per-column filter support** -- Add `filter.{column}` param parsing and parameterized WHERE clauses | #8 | High | T0 | pending |
| T3 | **Display config endpoint** -- Add `GET /api/config/display` with `display_name()` utility | #9 | High | T1 | pending |
| T4 | **CSV export endpoint** -- Add `GET /api/export/{table}/csv` with cursor streaming | #10 | High | T0, T2, T3 | pending |
| T5 | **Setup wizard endpoints** -- Two-mode server, `test-connection`, `save` endpoints | #11 | High | T1 | pending |
| T6 | **Table include/exclude filtering** -- Filter `list_tables` by config; `display_name` at API layer | #12 | High | T1 | pending |

**Dependency graph**: T0 and T1 are independent roots. T2 depends on T0. T3 and T5 and T6 depend on T1. T4 depends on T0, T2, and T3.

**Suggested implementation order**: T0 -> T1 -> T2 + T3 + T5 + T6 (parallel) -> T4

---

## Exit Criteria

- [ ] All Must-Have acceptance scenarios pass against a test PostgreSQL database
- [ ] SQL injection prevention maintained: table/column names validated, filter values parameterized
- [ ] Config changes are backward-compatible (missing sections use sensible defaults)
- [ ] All API contracts return correct JSON/CSV shapes matching the spec
- [ ] CSV export streams 500K+ rows without OOM
- [ ] Two-mode server correctly detects config presence and boots appropriate routes
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes

---

## References

- MVP spec: `seeki-mvp-spec.md` (tasks T1-T6)
- Epic issue: #1
- PR: #27 (`epic/1-backend-api-config`)
- Sub-issues: #7 (config), #8 (filters), #9 (display), #10 (CSV), #11 (setup), #12 (table filtering)
- Brainstorm: approved in this conversation session

---

*Authored by: Clault KiperS 4.6*
