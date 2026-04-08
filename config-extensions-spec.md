# Feature: Config Extensions

## Overview

**User Story**: As a SeeKi administrator, I want to configure which tables are exposed, how columns and tables are displayed, and set branding for the app so that the interface is tailored to my team's needs without code changes.

**Problem**: The current `AppConfig` only supports `[server]` and `[database]` sections. There is no way to control which tables appear, override column/table display names, or set app branding -- all of which are required by downstream features (display config endpoint, setup wizard, table filtering).

**Out of Scope**:
- API routes (T3 display config endpoint)
- Database query changes (T2 per-column filters, T6 table filtering logic)
- Two-mode server (T5 setup wizard)
- Error handling refactor (T0)

---

## Success Condition

> This feature is complete when `AppConfig` parses `[tables]`, `[display]`, and `[branding]` sections from TOML with sensible defaults, two utility functions convert raw names to display names using config overrides and heuristics, and existing configs (with only `[server]` + `[database]`) continue to parse without error.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | Column name heuristic edge cases (e.g. `posn_lat` -> "Posn Lat" vs something better) -- tune during implementation | Brainstorm | [ ] |

---

## Scope

### Must-Have

- **TablesConfig struct**: `include: Option<Vec<String>>`, `exclude: Option<Vec<String>>`. Both allowed simultaneously -- include applied first, then exclude subtracted. `#[serde(default)]` on the `AppConfig` field. Acceptance: both fields parse correctly; missing section defaults to no filtering.

- **DisplayConfig struct**: `tables: HashMap<String, String>` for table display name overrides, `columns: HashMap<String, HashMap<String, String>>` for column overrides (nested TOML: `[display.columns.table_name]`). `#[serde(default)]`. Acceptance: nested TOML structure parses into correct HashMap nesting; empty section defaults to empty maps.

- **BrandingConfig struct**: `title: Option<String>`, `subtitle: Option<String>`. `#[serde(default)]`. Acceptance: missing section defaults to `None` for both fields.

- **display_name_table() utility**: `fn display_name_table(table: &str, config: &DisplayConfig) -> String`. Check `config.tables` override first; if none, apply heuristic (snake_case to Title Case). Acceptance: `vehicles_log` -> "Vehicles Log" by default; config override takes precedence.

- **display_name_column() utility**: `fn display_name_column(table: &str, column: &str, config: &DisplayConfig) -> String`. Check `config.columns[table][column]` override first; if none, apply heuristic (snake_case to Title Case, drop `_id` suffix). Acceptance: `supervisor_id` -> "Supervisor" by default; config override takes precedence.

- **Updated seeki.toml.example**: Document all new sections with inline comments and example values. Acceptance: example file contains `[tables]`, `[display.tables]`, `[display.columns.example_table]`, and `[branding]` sections.

- **Backward compatibility**: Existing configs with only `[server]` and `[database]` must continue to parse without error. Acceptance: minimal config loads successfully with all new fields at defaults.

### Should-Have

- None -- this is a focused foundational task.

### Nice-to-Have

- None.

---

## Technical Plan

**Affected Components**:

| File | Changes |
|:-----|:--------|
| `src/config.rs` | Add `TablesConfig`, `DisplayConfig`, `BrandingConfig` structs; add `tables`, `display`, `branding` fields to `AppConfig`; add `display_name_table()` and `display_name_column()` functions; add `casualify()` helper |
| `seeki.toml.example` | Add all new config sections with documented examples |

**Data Model Changes**: None -- config-only changes, no database interaction.

**API Contracts**: None -- this task adds no routes.

**Dependencies**: None -- uses only `serde`, `toml`, and `std::collections::HashMap` (all already in scope).

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| TOML nested table syntax confuses users editing config | Low | Clear comments in `seeki.toml.example`; setup wizard (T5) will generate the file |
| Heuristic produces poor display names for unusual column names | Medium | Heuristic is intentionally simple; config overrides are the escape hatch |

---

## Acceptance Scenarios

```gherkin
Feature: Config Extensions
  As a SeeKi administrator
  I want to configure table exposure, display names, and branding
  So that the interface is tailored to my team

  Rule: Config backward compatibility

    Scenario: Minimal config parses with defaults
      Given a seeki.toml containing only [server] and [database] sections
      When AppConfig::load() is called
      Then the config loads successfully
      And tables.include is None
      And tables.exclude is None
      And display.tables is an empty HashMap
      And display.columns is an empty HashMap
      And branding.title is None
      And branding.subtitle is None

    Scenario: Full config with all sections parses correctly
      Given a seeki.toml containing all sections including [tables], [display], and [branding]
      When AppConfig::load() is called
      Then all fields are populated with the values from the file

  Rule: Tables include and exclude

    Scenario: Both include and exclude set
      Given a TablesConfig with include = ["a", "b", "c"] and exclude = ["c"]
      When the effective table list is computed
      Then the result is ["a", "b"]

    Scenario: Only include set
      Given a TablesConfig with include = ["a", "b"] and exclude = None
      When the effective table list is computed from tables ["a", "b", "c", "d"]
      Then the result is ["a", "b"]

    Scenario: Only exclude set
      Given a TablesConfig with include = None and exclude = ["c"]
      When the effective table list is computed from tables ["a", "b", "c", "d"]
      Then the result is ["a", "b", "d"]

    Scenario: Neither set
      Given a TablesConfig with include = None and exclude = None
      When the effective table list is computed from tables ["a", "b", "c"]
      Then all tables are returned: ["a", "b", "c"]

  Rule: Display name heuristic for columns

    Scenario: Snake case to Title Case
      Given no config override for "my_table.some_column"
      When display_name_column("my_table", "some_column", &config) is called
      Then it returns "Some Column"

    Scenario: Drop _id suffix
      Given no config override for "vehicles_log.supervisor_id"
      When display_name_column("vehicles_log", "supervisor_id", &config) is called
      Then it returns "Supervisor"

    Scenario: Config override takes precedence for columns
      Given config has display.columns.vehicles_log.posn_lat = "Latitude"
      When display_name_column("vehicles_log", "posn_lat", &config) is called
      Then it returns "Latitude"

  Rule: Display name heuristic for tables

    Scenario: Table snake case to Title Case
      Given no config override for "vehicles_log"
      When display_name_table("vehicles_log", &config) is called
      Then it returns "Vehicles Log"

    Scenario: Config override takes precedence for tables
      Given config has display.tables.vehicles_log = "Fleet Log"
      When display_name_table("vehicles_log", &config) is called
      Then it returns "Fleet Log"

  Rule: Branding defaults

    Scenario: No branding section
      Given a seeki.toml with no [branding] section
      When AppConfig::load() is called
      Then branding.title is None and branding.subtitle is None

    Scenario: Custom branding
      Given a seeki.toml with [branding] title = "AutoConnect" and subtitle = "Fleet Telemetry"
      When AppConfig::load() is called
      Then branding.title is Some("AutoConnect") and branding.subtitle is Some("Fleet Telemetry")
```

---

## Task Breakdown

| ID | Task | Priority | Dependencies | Status |
|:---|:-----|:---------|:-------------|:-------|
| T1.1 | Add `TablesConfig`, `DisplayConfig`, `BrandingConfig` structs to `config.rs` | High | None | pending |
| T1.2 | Add new fields to `AppConfig` with `#[serde(default)]` | High | T1.1 | pending |
| T1.3 | Implement `casualify()` helper (snake_case to Title Case, optional `_id` drop) | High | None | pending |
| T1.4 | Implement `display_name_table()` and `display_name_column()` | High | T1.1, T1.3 | pending |
| T1.5 | Update `seeki.toml.example` with all new sections | High | T1.1 | pending |
| T1.6 | Write unit tests for config parsing and display name functions | High | T1.1-T1.4 | pending |

---

## Exit Criteria

- [ ] `cargo test` passes with all new unit tests
- [ ] `cargo clippy` clean with no warnings
- [ ] Existing minimal config (only `[server]` + `[database]`) parses without error
- [ ] Full config with all new sections parses correctly
- [ ] `display_name_column` applies heuristic and respects overrides
- [ ] `display_name_table` applies heuristic and respects overrides
- [ ] `seeki.toml.example` documents all new sections with examples

---

## References

- Epic spec: `backend-api-config-spec.md`
- Epic issue: #1
- This issue: #7
- PR: #33 (`feature/7-config-extensions-tables-display-branding-sections`)
- Parent epic PR: #27 (`epic/1-backend-api-config`)

---

*Authored by: Clault KiperS 4.6*
