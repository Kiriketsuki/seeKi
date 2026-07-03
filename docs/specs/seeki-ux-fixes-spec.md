# Feature: SeeKi UX Fixes Batch

## Overview

**User Story**: As a non-technical SeeKi user browsing the AutoConnect database, I want the grid to scroll horizontally, cleaner sidebar badges, human-friendly table names I can set myself, and enum column values to actually display, so that I can browse wide tables comfortably without touching config files or seeing bogus NULLs.

**Problem**: Wide tables are unusable (no horizontal scrolling), Postgres enum columns silently render as NULL (data loss in the eyes of the user), `_id`-suffix prettification produces duplicate column headers, and table names can only be renamed by editing `seeki.toml` on the server.

**Out of Scope**: Column renaming from the UI; per-user (non-shared) rename scopes; SQLite (target-DB) engine support; any write access to the target PostgreSQL database.

---

## Success Condition

> This feature is complete when a user at sg.aurrigo.com/seeki can horizontally scroll any wide table, sees flat borderless sidebar badges, can rename a table inline from the sidebar (persisted in seeki.db for all users), and the `locations` table shows real values in `type`/`encoded_type` with no duplicate "Original Table" headers.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | Exact mechanism blocking horizontal scroll (CSS clip vs missing scrollbar vs event capture) — confirm during repro | Claude | [ ] |

---

## Scope

### Must-Have
- Horizontal grid scrolling: trackpad swipe, shift+wheel, and a visible draggable horizontal scrollbar all pan columns on tables wider than the viewport.
- Enum/USER-DEFINED column decode: `type`, `encoded_type`, and any custom-typed column render their text values instead of NULL.
- Collision-safe column prettification: `_id` suffix is only dropped when the result does not duplicate another column's display name in the same table (`original_table_id` → "Original Table ID" when `original_table` exists).
- UI table rename: hover pencil / double-click on a sidebar table item opens inline edit; Enter saves, Esc cancels; empty value reverts to the automatic name; tooltip shows the real DB name; grid header title follows the rename.
- Rename persistence: stored in local `seeki.db` (`table_display_names`), shared by all users, survives restart. Precedence: UI rename > `seeki.toml [display]` > title-case heuristic.
- Flat sidebar badges: no border, `--sk-radius-sm`, subtle background tint only; consistent across `TableList` count chips and `Sidebar` badges.

### Should-Have
- Rename validation: trim whitespace, cap length (e.g. 64 chars), reject empty-after-trim as "revert to auto".

### Nice-to-Have
- "Reset name" affordance in the inline editor (explicit revert without blanking the field).

---

## Technical Plan

**Affected Components**:
- `frontend/src/components/DataGrid.svelte` — horizontal scroll fix
- `frontend/src/components/TableList.svelte`, `Sidebar.svelte` — chip restyle + inline rename UI
- `frontend/src/lib/api.ts`, `stores.ts`, `types.ts` — display-name API client + state
- `src/db/postgres.rs` — `pg_value_to_json` fallback → `try_get_unchecked::<String, _>`
- `src/config.rs` — collision-aware `display_name_column` (needs sibling column names in scope; callers in `src/db/postgres.rs:222–227, 313–318` already iterate full column lists)
- `src/store/` — new `store/display_names.rs` (CRUD, mirroring `store/views.rs` pattern)
- `src/api/mod.rs` — new routes

**Data Model Changes**:
- New SQLite table in seeki.db: `table_display_names (schema_name TEXT NOT NULL, table_name TEXT NOT NULL, display_name TEXT NOT NULL, updated_at TEXT, PRIMARY KEY (schema_name, table_name))`

**API Contracts**:
- `GET /api/tables/display-names` — returns `{ "schema.table": "Display Name", ... }`
- `PUT /api/tables/display-names/{schema}/{table}` — body `{ "display_name": "..." }`; empty/absent display_name deletes the override (revert). Returns the resolved name.
- `GET /api/tables` — response display_name now resolves store override first.

**Dependencies**: none external. sqlx `try_get_unchecked` (already available). No migration tooling — table created via `CREATE TABLE IF NOT EXISTS` in `Store::open()` per existing pattern.

**Risks**:
| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| `try_get_unchecked` returns garbage for exotic binary-format types | Low | Only used in the fallback arm where behaviour is currently NULL; worst case equals status quo |
| Horizontal scroll root cause differs from hypothesis | Medium | Repro first; fix is scoped to DataGrid.svelte CSS/RevoGrid config either way |
| Rename collides with issue-branch automation naming | Low | Feature is display-only; DB identifiers never change |

---

## Acceptance Scenarios

```gherkin
Feature: SeeKi UX fixes batch

  Background:
    Given SeeKi is connected to a PostgreSQL database

  Rule: Wide tables scroll horizontally

    Scenario: Panning a wide table
      Given a table with more columns than fit the viewport
      When the user shift+scrolls, swipes horizontally, or drags the horizontal scrollbar
      Then hidden columns scroll into view

  Rule: Enum columns display values

    Scenario: Enum column renders text
      Given a table with a Postgres enum column "type"
      When the rows are fetched
      Then each cell shows the enum's text value, not NULL

    Scenario: Truly NULL enum cell
      Given an enum column containing SQL NULL
      When the rows are fetched
      Then the cell renders the hatched NULL badge

  Rule: Column display names never collide

    Scenario: _id suffix kept on collision
      Given a table with columns "original_table" and "original_table_id"
      When column headers are generated
      Then headers are "Original Table" and "Original Table ID"

  Rule: Tables can be renamed from the UI

    Scenario: Rename a table inline
      Given the sidebar table list is visible
      When the user double-clicks a table name, types "Vehicle Telemetry", and presses Enter
      Then the sidebar and grid header show "Vehicle Telemetry"
      And the name persists after a server restart and for other users

    Scenario: Revert a rename
      Given a table with a UI rename
      When the user clears the name and presses Enter
      Then the table reverts to its automatic display name

    Scenario: Cancel editing
      When the user presses Esc while editing
      Then the previous name is kept unchanged
```

---

## Task Breakdown

| ID | Task | Priority | Dependencies | Status |
|:---|:-----|:---------|:-------------|:-------|
| T1 | Fix enum/USER-DEFINED decode in `pg_value_to_json` (try_get_unchecked fallback) + unit test | High | None | pending |
| T2 | Collision-aware `display_name_column` (table-scoped) + unit tests | High | None | pending |
| T3 | Repro + fix horizontal grid scrolling in DataGrid.svelte | High | None | pending |
| T4 | Flat sidebar badges (TableList + Sidebar chip restyle) | Med | None | pending |
| T5 | `table_display_names` store module + CRUD tests | High | None | pending |
| T5.1 | API routes GET/PUT display-names + resolution precedence in list_tables | High | T5 | pending |
| T5.2 | Inline rename UI in sidebar (edit, save, revert, tooltip) + header binding | High | T5.1 | pending |
| T6 | End-to-end verification against Aurrigo DB + deploy via release flow | Med | T1–T5.2 | pending |

---

## Exit Criteria

- [ ] All Must-Have scenarios pass in CI (`cargo test`, frontend vitest)
- [ ] No regressions on related features (saved views, sorting, stats bar)
- [ ] API contracts match implementation
- [ ] Verified against the live Aurrigo AutoConnect database (locations table types visible, wide vehicles_log scrolls)

---

## References

- Related specs: N/A
- Tickets: GitHub issues created from this spec (see repo issue tracker)

---
*Authored by: Clault KiperF 5.0*
