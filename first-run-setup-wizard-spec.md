# Feature: First-Run Setup Wizard

## Overview

**User Story**: As a non-technical user receiving a SeeKi binary, I want a browser-based first-run setup wizard so that I can configure a database connection (including SSH tunnel) and start browsing data without editing TOML files or restarting the app.

**Problem**: When no `seeki.toml` exists, the backend boots into setup mode with only API endpoints — the frontend renders a placeholder div. Users cannot configure SeeKi without hand-editing TOML. The backend also currently requires a restart after save ("Configure your database connection, then restart the app"). Additionally, the origin use case (Aurrigo AutoConnect DB) lives behind an SSH bastion and the current backend has no SSH tunneling support.

**Out of Scope**:
- SQLite support (remains planned for v0.2)
- In-app settings screen for editing config after initial setup
- Multi-profile / database switching from the UI
- Advanced SSH options (ProxyJump chaining, SOCKS proxies, custom `known_hosts`)
- SSH host key verification UI (will auto-accept new host keys with a log warning)
- Column / table display name overrides in the wizard (wizard sets title + subtitle only)
- Connection pool tuning in UI (uses default `max_connections = 5`)

---

## Success Condition

> This feature is complete when a non-technical user running the SeeKi binary for the first time can, entirely within the browser, enter database connection details (including SSH tunnel with private key + passphrase), verify the connection, pick which tables to expose, set an app title, save the config, and be taken directly into the grid view — all without restarting the binary.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | None identified | — | [x] |

---

## Scope

### Must-Have

- **Wizard shell (`SetupWizard.svelte`)**: Owns step state (`currentStep`, `wizardData`), renders a centered glass card on a dark background, 4-dot progress indicator, uses existing `--sk-*` tokens. Emits no events upward; on successful save triggers `window.location.reload()` after polling detects `mode: "normal"`.
- **Step 1 — Connection + Test**: URL field by default; a "Fill in fields instead" toggle reveals host/port/database/user/password with a live URL preview. A "Connect via SSH Tunnel" toggle reveals an SSH section with host/port/username and an auth method selector (private key file w/ optional passphrase, password, SSH agent). An inline "Test Connection" button calls the backend; success shows table count preview and enables Next; failure shows a human-readable error distinguished by `error_source` (`ssh` vs `db`).
- **Step 2 — Tables**: Smart-filtered checklist with two tabs — "Your tables" (non-system, pre-checked) and "System" (system tables like `pg_*`, `information_schema.*`, pre-unchecked, hidden on first render). Search box filters the visible tab. Selection count is shown. Next is disabled if zero tables are selected.
- **Step 3 — Branding**: Title field (required) and subtitle field (optional) with a live sidebar preview card showing how they will appear in the grid view.
- **Step 4 — Confirm**: Readonly summary of all wizard state (database kind, host, SSH state, table count, title). Back button returns to step 3 with state preserved. "Save & Open" triggers the save flow.
- **Backend config schema**: `AppConfig` gains `ssh: Option<SshConfig>`. `SshConfig` has `host`, `port` (default 22), `username`, `auth_method` ("key" | "password" | "agent"), `key_path` (required for key method). **Secrets never land in `seeki.toml`** — passphrases and passwords go into a sibling `.seeki.secrets` file with mode 0600.
- **Backend SSH tunnel** (`src/ssh/mod.rs`): Wraps the `openssh` crate to establish a local port forward before `sqlx` connects. Tunnel lifetime is tied to the app — dropped on hot reload rollback. `StrictHostKeyChecking=accept-new` is passed to ssh.
- **Backend hot reload** (`src/app_mode.rs`): Introduce `AppMode = Setup | Normal(Arc<RealAppState>)` wrapped in `Arc<RwLock<AppMode>>`. `main.rs` builds a single router with this state. Routes dispatch based on the current mode. Save handler rebuilds `DatabasePool` (with optional tunnel), then takes the write lock and swaps `AppMode::Setup → AppMode::Normal`. In-flight requests finish on the old mode; new requests see the new mode.
- **`/api/status`**: Always returns `{ mode: "setup" | "normal" }` regardless of current mode — available in both setup and normal routers so the frontend can poll it.
- **`POST /api/setup/test-connection`** (extended): Request gains optional `ssh` block. Response gains `tables: [{ name, estimated_rows, is_system }]` and `error_source: "ssh" | "db"`.
- **`POST /api/setup/save`** (extended): Request gains `ssh`, `tables.include`, and `branding` blocks. Handler validates connection (including SSH), writes `seeki.toml` (no secrets) and `.seeki.secrets` (mode 0600) atomically, swaps `AppMode`, returns success. On any failure, rolls back cleanly — no stale files.
- **Frontend polling + reload**: After save success, poll `GET /api/status` every 250ms up to 20 attempts. When `mode === "normal"`, call `window.location.reload()` which re-bootstraps the SPA against the normal router.
- **Error handling**: All errors from both endpoints surface in the UI with copy a non-technical user can act on. SSH errors are visually distinct from DB errors.
- **Backwards compatibility**: Existing `seeki.toml` files without `[ssh]` continue to load unchanged.

### Should-Have

- **Keyboard navigation**: Enter advances to next step when the current step is valid; Esc on step 1 clears the input; Back/Next buttons are keyboard accessible.
- **Field preservation on Back**: Step 1 values (except password/passphrase) are preserved when the user navigates Back.
- **URL parsing**: When the user toggles from URL mode to fields mode, an existing URL is parsed into the individual fields.
- **Progress indicator**: 4-dot stepper at the top of the card showing current step, completed steps, and locked future steps.

### Nice-to-Have

- **SSH agent auto-detect**: If `SSH_AUTH_SOCK` is set, default the auth method to "SSH agent".
- **Localhost defaults**: Pre-fill `localhost:5432` / `postgres` when fields mode is first opened.
- **"Where do I find this?" helper**: Expandable tip on step 1 showing where to find the connection string in pgAdmin / DBeaver.

---

## Technical Plan

**Affected Components**:

| File | Action | Description |
|:-----|:-------|:------------|
| `src/main.rs` | Modify | Replace boot-time mode switch with single router backed by `Arc<RwLock<AppMode>>` |
| `src/app_mode.rs` | New | `AppMode` enum + hot-reload state wrapper |
| `src/api/mod.rs` | Modify | Router dispatches to setup or normal handlers based on `AppMode`; `/api/status` always works |
| `src/api/setup.rs` | Modify | Extend `test-connection` + `save` with `ssh`, `tables.include`, `branding` params; save swaps `AppMode` |
| `src/config.rs` | Modify | Add `SshConfig` struct; add `ssh: Option<SshConfig>` to `AppConfig`; add `.seeki.secrets` loader |
| `src/ssh/mod.rs` | New | SSH tunnel wrapper around `openssh` crate; spawn, local port forwarding, drop cleanup |
| `src/db/mod.rs` | Modify | `DatabasePool::connect` accepts optional `SshConfig`; opens tunnel before sqlx connect |
| `src/db/postgres.rs` | Modify | `test_connection` accepts optional `SshConfig`, connects via the forwarded port |
| `Cargo.toml` | Modify | Add `openssh` dependency |
| `.gitignore` | Modify | Add `.seeki.secrets` |
| `frontend/src/components/SetupWizard.svelte` | New | Container: step state, transitions, `wizardData` |
| `frontend/src/components/SetupStep1Connection.svelte` | New | URL/fields toggle, SSH section with 3 auth methods, test-connection call |
| `frontend/src/components/SetupStep2Tables.svelte` | New | Two-tab smart checklist (Your tables / System), search, selection count |
| `frontend/src/components/SetupStep3Branding.svelte` | New | Title + subtitle with live sidebar preview |
| `frontend/src/components/SetupStep4Confirm.svelte` | New | Summary, save, polling, redirect |
| `frontend/src/lib/api.ts` | Modify | Add `setupTestConnection()`, `setupSaveConfig()`, `getStatus()` |
| `frontend/src/lib/types.ts` | Modify | Add `WizardData`, `SetupStatus`, `TestConnectionResult`, `SshWizardConfig` types |
| `frontend/src/App.svelte` | Modify | Replace placeholder `{#if isSetup}` block with `<SetupWizard />` |

**Data Model Changes**:

- **`seeki.toml`**: new optional `[ssh]` section (no secrets):
  ```toml
  [ssh]
  host = "bastion.example.com"
  port = 22
  username = "ubuntu"
  auth_method = "key"                   # "key" | "password" | "agent"
  key_path = "/home/user/.ssh/id_rsa"   # required when auth_method = "key"
  ```
- **`.seeki.secrets`** (new sibling, mode 0600, gitignored):
  ```toml
  [ssh]
  key_passphrase = "..."
  # or:
  password = "..."
  ```
- **Rationale**: `seeki.toml` may end up in version control or shared environments; secrets must not.

**API Contracts**:

- `GET /api/status` → `{ "mode": "setup" | "normal" }` — available in both modes
- `POST /api/setup/test-connection`:
  ```json
  {
    "kind": "postgres",
    "url": "postgresql://user:pass@host:5432/db",
    "ssh": {
      "host": "bastion.example.com",
      "port": 22,
      "username": "ubuntu",
      "auth_method": "key",
      "key_path": "/home/user/.ssh/id_rsa",
      "key_passphrase": "..."
    }
  }
  ```
  → `{ "success": true, "tables": [{"name":"users","estimated_rows":4200,"is_system":false}, ...] }`
  → or `{ "success": false, "error": "...", "error_source": "ssh" | "db" }`
- `POST /api/setup/save`:
  ```json
  {
    "server": { "host": "127.0.0.1", "port": 3141 },
    "database": { "kind": "postgres", "url": "postgresql://...", "max_connections": 5 },
    "ssh": { ... },
    "tables": { "include": ["users", "orders", "products"] },
    "branding": { "title": "Fleet Telemetry", "subtitle": "Route history and live tracking" }
  }
  ```
  → `{ "success": true }` — backend swaps `AppMode` before responding
  → or `{ "success": false, "error": "..." }` — no files written

**Dependencies**:

- `openssh` crate (Rust wrapper for system `ssh` binary) — covers key file, password, and agent via existing ssh tooling and respects `ssh-agent`.
- No new frontend dependencies.
- Runtime prerequisite: system `ssh` binary on `PATH` (documented in README).

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| `openssh` crate requires system `ssh` binary on host | High | Document prerequisite in README; error clearly if binary missing |
| Password-based SSH auth is awkward via system `ssh` | Medium | Accept limitation; key file is primary path per origin use case |
| Hot-reload race: request in flight during mode swap | Medium | Hold `RwLock` write lock during swap; new requests wait microseconds; in-flight requests complete on old state |
| SSH tunnel process leaks on save rollback | Medium | Explicit cleanup in save error path; `Drop` impl on tunnel wrapper terminates child |
| `.seeki.secrets` world-readable on Windows | Medium | Document Windows caveat in README; set NTFS ACLs where possible |
| SSH host key verification fails on first connect | Medium | Pass `-o StrictHostKeyChecking=accept-new`; log warning when new host key accepted |
| Large schemas (100+ tables) slow the selection list | Low | Search box handles filter; tab split already reduces visible count |
| User closes browser mid-save → config writes but no mode swap happens | Low | Next request sees written config and reloads `AppMode` via file check fallback |

---

## Acceptance Scenarios

```gherkin
Feature: First-Run Setup Wizard
  As a non-technical user
  I want a browser-based setup wizard on first launch
  So that I can configure my database without editing TOML or restarting the binary

  Background:
    Given no seeki.toml exists in the current directory
    And no seeki.toml exists in ~/.config/seeki/
    And the SeeKi binary is started
    And the user opens http://127.0.0.1:3141 in a browser

  Rule: Wizard renders when no config exists

    Scenario: Fresh install shows the wizard
      Then the wizard appears as a centered card on a dark background
      And the card shows "Step 1 of 4 — Connect your database"
      And a 4-dot progress indicator is visible
      And step 1 is marked active

  Rule: Step 1 validates a direct Postgres URL connection

    Scenario: User pastes a valid URL and tests successfully
      Given the user is on step 1 with URL mode selected
      When the user pastes "postgresql://user:pass@localhost:5432/mydb"
      And the user clicks "Test Connection"
      Then the button shows a loading spinner
      And on success the button turns green with a table count message
      And the "Next" button becomes enabled

    Scenario: User sees a helpful error on connection failure
      Given the user is on step 1
      When the user pastes an unreachable URL and clicks "Test Connection"
      Then the UI shows an error message prefixed "Database:"
      And the "Next" button remains disabled

  Rule: Step 1 supports individual field input

    Scenario: User toggles to individual fields
      Given the user is on step 1 with URL mode
      When the user clicks "Fill in fields instead"
      Then fields appear for host, port, database, user, password
      And a URL preview updates live below the fields
      When the user clicks "Test Connection"
      Then the backend receives the assembled URL

  Rule: Step 1 supports SSH tunneling with private key + passphrase

    Scenario: User enables SSH tunnel with key file
      Given the user is on step 1
      When the user toggles "Connect via SSH Tunnel"
      Then an SSH section appears with host, port, username, and auth method selector
      When the user selects "Private Key File" as auth method
      Then a file path field and optional passphrase field appear
      When the user enters valid SSH details and clicks "Test Connection"
      Then the backend establishes the tunnel
      And tests the DB through the tunnel
      And the UI reports success with the table list

    Scenario: SSH connection fails with distinct error source
      Given the user has enabled SSH tunnel with an invalid key path
      When the user clicks "Test Connection"
      Then the UI shows an error prefixed "SSH:"
      And the "Next" button remains disabled

  Rule: Step 2 shows a smart-filtered table list

    Scenario: User sees non-system tables pre-checked by default
      Given step 1 returned 12 user tables and 8 system tables
      When the user advances to step 2
      Then the "Your tables (12)" tab is active
      And shows 12 tables, all pre-checked
      And the "System (8)" tab is inactive
      And "Next" is enabled

    Scenario: User deselects a non-system table
      Given the user is on step 2 with all 12 tables checked
      When the user unchecks "audit_log"
      Then the checkbox is cleared
      And the selection count reads "11 of 12 selected"

    Scenario: User cannot advance with zero selections
      Given the user is on step 2
      When the user clicks "Deselect All"
      Then all checkboxes are cleared
      And "Next" is disabled
      And a hint says "Select at least one table"

    Scenario: User opts to expose a system table
      Given the user is on step 2
      When the user clicks the "System" tab
      And checks "pg_stat_activity"
      Then the selection count increases by 1

  Rule: Step 3 captures branding with live preview

    Scenario: User sets title and subtitle
      Given the user is on step 3
      When the user types "Fleet Telemetry" into the title field
      And "Route history and live tracking" into the subtitle field
      Then a live sidebar preview shows both values
      And "Next" becomes enabled (title is non-empty)

    Scenario: Empty title blocks advance
      Given the user is on step 3
      When the title field is empty
      Then "Next" is disabled

  Rule: Step 4 shows summary and saves, with in-process hot reload

    Scenario: User confirms and saves successfully
      Given the user is on step 4
      Then the summary shows database kind, host, SSH state, selected table count, and title
      When the user clicks "Save & Open"
      Then the button shows "Saving..."
      And the backend writes seeki.toml (no secrets)
      And the backend writes .seeki.secrets with mode 0600 (if SSH passphrase or password provided)
      And the backend swaps AppMode from Setup to Normal
      And the frontend polls /api/status
      When /api/status returns "mode": "normal"
      Then the page reloads
      And the user lands in the grid view
      And the sidebar shows the configured title and subtitle
      And the first selected table is auto-loaded

    Scenario: Save fails because the DB became unreachable
      Given the user is on step 4
      And the database has become unreachable since step 1
      When the user clicks "Save & Open"
      Then the backend responds success=false with a human-readable error
      And seeki.toml is not written
      And .seeki.secrets is not written
      And the user remains on step 4 with an error banner
      And the user can navigate Back to step 1 with preserved non-secret fields to retry

    Scenario: Save fails because seeki.toml cannot be written
      Given the user is on step 4
      And the CWD is read-only
      When the user clicks "Save & Open"
      Then the backend responds with an error mentioning the write failure
      And no partial files exist on disk
      And AppMode remains Setup
```

---

## Task Breakdown

| ID  | Task | Priority | Dependencies | Status |
|:----|:-----|:---------|:-------------|:-------|
| T1  | Add `SshConfig` struct and `ssh: Option<SshConfig>` to `AppConfig` in `src/config.rs`; add `.seeki.secrets` loader; unit tests for parsing with and without SSH | High | None | pending |
| T2  | Add `src/ssh/mod.rs` — SSH tunnel wrapper using the `openssh` crate (spawn, local port forward, `Drop` cleanup); unit test against a local ssh daemon if available, otherwise integration-only | High | T1 | pending |
| T3  | Extend `DatabasePool::connect` in `src/db/mod.rs` to accept optional `SshConfig`; open tunnel before sqlx; tunnel is held for pool lifetime | High | T2 | pending |
| T4  | Extend `postgres::test_connection` in `src/db/postgres.rs` to accept optional `SshConfig`; return structured result with table rows estimate and system flag | High | T3 | pending |
| T5  | Add `src/app_mode.rs` with `AppMode` enum (`Setup`, `Normal(Arc<AppState>)`) and helper types; tests for mode transitions | High | None | pending |
| T6  | Refactor `src/main.rs` — single router backed by `Arc<RwLock<AppMode>>`; `/api/status` always works; routes dispatch from mode; both initial-config-present and initial-config-absent boot paths | High | T5 | pending |
| T7  | Extend `POST /api/setup/test-connection` in `src/api/setup.rs` — accept optional `ssh` param; richer response with `tables: [{name, estimated_rows, is_system}]` and `error_source` | High | T4 | pending |
| T8  | Extend `POST /api/setup/save` in `src/api/setup.rs` — accept `ssh`, `tables.include`, `branding`; write `seeki.toml` + `.seeki.secrets`; swap `AppMode`; rollback on any failure | High | T6, T7 | pending |
| T9  | Backend unit tests — SSH config round-trip, secrets file mode 0600, `AppMode` swap correctness, save rollback on DB failure, save rollback on fs failure | High | T8 | pending |
| T10 | Add `openssh` to `Cargo.toml`; document system `ssh` binary prerequisite in README | High | T2 | pending |
| T11 | Add `.seeki.secrets` to `.gitignore` | Med | T8 | pending |
| T12 | Create `frontend/src/components/SetupWizard.svelte` — step state, transitions, `wizardData` shape, progress indicator, centered glass card | High | None | pending |
| T13 | Create `frontend/src/components/SetupStep1Connection.svelte` — URL/fields toggle, SSH section with 3 auth methods, test-connection button, error display with `error_source` | High | T12 | pending |
| T14 | Create `frontend/src/components/SetupStep2Tables.svelte` — two-tab smart checklist, search, selection count, validation | High | T12 | pending |
| T15 | Create `frontend/src/components/SetupStep3Branding.svelte` — title + subtitle with live sidebar preview | High | T12 | pending |
| T16 | Create `frontend/src/components/SetupStep4Confirm.svelte` — summary, save, `/api/status` polling, reload on success, error banner on failure | High | T12, T7, T8 | pending |
| T17 | Add `setupTestConnection()`, `setupSaveConfig()`, `getStatus()` to `frontend/src/lib/api.ts` | High | T7, T8 | pending |
| T18 | Add `WizardData`, `SetupStatus`, `TestConnectionResult`, `SshWizardConfig` types to `frontend/src/lib/types.ts` | High | None | pending |
| T19 | Wire `<SetupWizard />` into the `{#if isSetup}` block in `frontend/src/App.svelte` | High | T12, T13, T14, T15, T16 | pending |
| T20 | Manual QA: fresh install → wizard → direct URL → save → grid loads without restart | High | T19, T8 | pending |
| T21 | Manual QA: fresh install → wizard → SSH tunnel with key file + passphrase → save → grid loads (against origin use case) | High | T19, T8 | pending |
| T22 | Manual QA: error paths — invalid URL, invalid SSH key path, DB unreachable at save, read-only CWD | High | T19 | pending |

---

## Exit Criteria

- [ ] All Must-Have acceptance scenarios pass manual verification against a real Postgres DB behind an SSH bastion
- [ ] Cold start with no config → wizard → completed setup → grid view, without restarting the binary
- [ ] Private key + passphrase SSH auth works end-to-end against the origin use case (Aurrigo AutoConnect DB)
- [ ] `seeki.toml` never contains passwords or passphrases
- [ ] `.seeki.secrets` is created with mode 0600 on POSIX and listed in `.gitignore`
- [ ] All existing Epic 1-4 backend and frontend tests still pass
- [ ] New unit tests added for SSH config parsing, test-connection with SSH, and `AppMode` hot-reload transitions
- [ ] `cargo clippy` clean
- [ ] Frontend `vitest run` clean
- [ ] README documents the `ssh` binary runtime prerequisite and the secrets file behaviour

---

## References

- Related specs: [seeki-mvp-spec.md](./seeki-mvp-spec.md), [config-extensions-spec.md](./config-extensions-spec.md)
- Epic issue: #5
- PR: #31
- Sub-issue: #25 (setup wizard UI)
- Backend touchpoints: `src/api/setup.rs`, `src/main.rs`, `src/config.rs`, `src/db/mod.rs`, `src/db/postgres.rs`
- Frontend touchpoints: `frontend/src/App.svelte` (placeholder line ~432), `frontend/src/components/`
- Existing test patterns: `src/api/setup.rs` tests, `frontend/src/lib/*.test.ts`

---
*Authored by: Clault KiperS 4.6*
