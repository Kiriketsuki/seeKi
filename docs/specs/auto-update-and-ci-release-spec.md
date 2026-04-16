# Feature: Auto-Update System + Multi-Platform CI Release Pipeline

## Context

SeeKi already ships a well-designed update module (`src/update/`) with secure
SHA256-verified downloads, atomic binary swap, rollback, bearer-token auth, and
ELF validation. All mutating API endpoints (`/apply`, `/rollback`, `/wip`) and
the status/check endpoints are already wired in `src/api/update.rs` and
`src/api/mod.rs`.

However, three critical gaps prevent the update flow from working end-to-end:

1. **CI builds no binaries.** `.github/workflows/release.yml` creates GitHub
   Releases but uploads zero artifacts. `select_asset()` finds nothing to
   download, so the auto-update path is inert.
2. **`select_asset()` is Linux-only.** Currently hardcoded to
   `seeki-x86_64-linux-musl` / `-gnu`. SeeKi will ship to macOS too.
3. **Frontend can't drive the patcher.** `UpdatesPanel.svelte` is a placeholder
   with disabled buttons and a literal "patcher backend unavailable" message,
   even though the backend is live. Mutating endpoints require a bearer token
   that the frontend has no way to fetch.

Additionally, the one-shot background check in `main.rs` runs only at startup
and never re-polls, so a browser left open overnight will never notice a new
release.

This spec delivers the CI pipeline + the frontend wiring + the small Rust
deltas needed to close these gaps.

---

## Overview

**User Story**: As a SeeKi user (sg-server operator today, external customer
tomorrow), I want my running instance to check for, download, and apply new
releases from one click in Settings, so I don't need terminal access or
SSH-into-the-box knowledge to stay current.

**Problem**: The update subsystem is ~80% built but never reaches the user: CI
ships no binaries for the downloader to find, the frontend has disabled
buttons, and the poller is one-shot. Deploying a new build today requires
manual scp, kill, and pray-systemd-restarts — exactly what SeeKi is meant to
abolish for its non-technical users.

**Out of Scope**:
- Signed releases (GPG/cosign signatures). SHA256 sidecar is the trust anchor.
- Delta updates / binary patching. Full-binary replacement is acceptable at
  ~15 MB.
- Automatic self-restart when no service manager runs the binary. If the user
  started seeki manually via `./seeki`, apply succeeds but the restart is
  their responsibility.
- Windows target. macOS + Linux musl only for v1.
- Customer-facing update UI copy polish beyond "Update available" /
  "Restarting" / "Rollback".
- Signed CI provenance attestation (SLSA, etc.).

---

## Success Condition

> This feature is complete when a user running an installed seeki can see
> a new release appear in Settings > Updates without a browser reload, click
> Apply, watch the server restart, and land on the new version — with a
> Rollback button that reverts the same way. This works for Linux musl,
> macOS Intel, and macOS Apple Silicon binaries served from GitHub Releases
> built by CI.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | Nightly version scheme. **Resolved**: CI rewrites VERSION to `{base}N{YYMMDD}` for nightly builds (e.g. `26.5.0.3aN260416`). Fits existing `SeekiVersion` parser and lexicographic suffix ordering. | planner | [x] |
| 2 | macOS code signing. **Resolved**: Ship unsigned for v1; README documents `xattr -d com.apple.quarantine /path/to/seeki` as the one-time bypass. Defer Developer ID signing + notarization to a later spec. | planner | [x] |
| 3 | Token delivery. **Resolved**: `GET /api/update/token` with same-origin guard. Frontend caches in memory for the session. | planner | [x] |
| 4 | Poller interval. **Resolved**: Configurable in Settings > Updates via a new `poll_interval_hours` field on `UpdateSettings`. Options: Hourly / 6-hourly / Daily / Never. Default: 6-hourly. | planner | [x] |

---

## Scope

### Must-Have

- **CI matrix build**: the `release` workflow builds `x86_64-unknown-linux-musl`,
  `x86_64-apple-darwin`, and `aarch64-apple-darwin` binaries and attaches each
  with its `.sha256` sidecar to the GitHub Release.
  _Acceptance_: Running the workflow on a push to `release` produces a Release
  with 6 assets (3 binaries + 3 sha256 files) with matching sums.

- **Nightly / pre-release builds**: the same workflow runs on push to
  `epic/*`, `feat/*`, `hotfix/*` and on `workflow_dispatch`, creating a
  Release with `prerelease: true`.
  _Acceptance_: Pushing to the current `epic/61-epic-ux-tuning` branch
  triggers a build and creates a pre-release tag with artifacts.

- **Cross-platform `select_asset()`**: the Rust code detects OS + arch at
  runtime and selects the matching asset.
  _Acceptance_: On Linux musl, selects `seeki-x86_64-linux-musl`. On macOS
  M1, selects `seeki-aarch64-darwin`. On macOS Intel, selects
  `seeki-x86_64-darwin`. Unit test covers all three cases.

- **Periodic background poller with configurable interval**: replaces the
  one-shot startup check with a tokio task that re-reads the interval
  from `UpdateSettings` each cycle, so a Settings change takes effect on
  the next tick without a restart. Options: Hourly / 6-hourly / Daily /
  Never. Default: 6-hourly.
  _Acceptance_: `last_checked` in `UpdateSettings` advances over time
  without user interaction; `update_available` flips to `true` when a
  new release is published, visible via `GET /api/update/status` without
  any frontend action. Switching the interval in Settings changes the
  next wake time; switching to Never stops the poller without a restart.

- **Token delivery endpoint**: a way for the frontend to retrieve the
  bearer token needed for mutating calls, guarded by same-origin /
  localhost checks.
  _Acceptance_: The frontend can call `/api/update/apply` with a valid
  `Authorization: Bearer <token>` header without the user pasting the
  token from `~/.config/seeki/update-token`.

- **Updates panel apply flow**: the existing disabled buttons in
  `UpdatesPanel.svelte` are wired to the real API, with a "Restarting..."
  overlay that polls `/api/update/status` until the server responds, then
  reloads the page.
  _Acceptance_: User clicks "Install update" → sees overlay → lands on new
  version in the same browser tab without a manual refresh.

- **Rollback flow**: visible only when `previous_exists: true`; same
  disconnect/reconnect UX as apply.
  _Acceptance_: After a successful apply, clicking "Rollback" returns the
  binary to its prior version; after rollback completes, the button
  disappears (no second rollback available — matches existing swap.rs
  behaviour).

- **Update-available badge**: a visual indicator in the Settings sidebar
  (or app shell) when `update_available: true`, so the user doesn't need
  to open the Updates panel to notice.
  _Acceptance_: Badge appears when the poller finds a new release;
  disappears after apply succeeds or after the user dismisses (one-shot).

### Should-Have

- **WIP upload UI**: wire the "Upload WIP build" button to the existing
  `/api/update/wip` endpoint for internal use (ops pushing a local musl
  build without waiting for CI). Helpful for sg-server deployment today.

- **Release notes preview**: show the `body` field from the GitHub
  release in the Updates panel so the user knows what they're installing.

- **Manual "Check now" latency indicator**: spinner + timestamp showing
  when the last check succeeded.

### Nice-to-Have

- **CI caching**: sccache / cargo registry cache to bring matrix build
  time under 5 minutes.

- **macOS universal binary**: a single `seeki-universal-darwin` produced
  by `lipo` combining both arch slices.

- **Changelog generation**: derive release notes from git log between the
  prior tag and HEAD, append to the existing release body.

---

## Technical Plan

### Affected Components

| Area | File | Change |
|:-----|:-----|:-------|
| CI | `.github/workflows/release.yml` | Add matrix build jobs before release job; attach artifacts |
| Rust | `src/update/github.rs` | `select_asset()` uses `std::env::consts::{OS, ARCH}` |
| Rust | `src/main.rs` | Replace one-shot `tokio::spawn` with periodic 6h loop |
| Rust | `src/api/update.rs` | Add `GET /api/update/token` handler |
| Rust | `src/api/mod.rs` | Route the new token endpoint |
| Frontend | `frontend/src/components/settings/UpdatesPanel.svelte` | Wire apply/rollback/upload buttons, reconnect overlay, release notes |
| Frontend | `frontend/src/lib/api.ts` | Attach `Authorization` header to mutating calls using fetched token |
| Frontend | `frontend/src/components/SettingsNav.svelte` (or sidebar) | Badge when `update_available` |
| Frontend | `frontend/src/lib/stores.ts` | Store for update status shared across components |
| Build | `VERSION` handling in CI | Nightly version rewrite for non-`release` branches |

### Data Model Changes

- **Add `poll_interval_hours: u32`** to `UpdateSettings` (persisted to
  `~/.seeki/update.json`). Value `0` means "Never". Default: `6`. Valid
  values accepted by the settings endpoint: `0, 1, 6, 24`.
- **Optional (Should-Have)**: add `dismissed_version: Option<String>` so
  the badge can be dismissed without re-appearing for the same version.

`UpdateState` already has `cache.latest()`, which serves the "available"
read path.

### API Contracts

**New**: `GET /api/update/token`
- Auth: same-origin check (inspect `Origin` header against request `Host`;
  reject if mismatch or if `Origin` is absent and the request is not from
  a localhost source). No bearer token required.
- Response: `{ "token": "<64-char-hex>" }`
- Purpose: lets the frontend cache the token in memory for mutating calls.

**Extended**: `PATCH /api/update/settings`
- Accepts `poll_interval_hours` in addition to the existing
  `pre_release_channel` field. Validated: must be one of `0, 1, 6, 24`.
  Persists via `UpdateSettings::save()`.

**Unchanged**: `/api/update/status`, `/api/update/check`,
`/api/update/apply`, `/api/update/rollback`, `/api/update/wip`.

### Build / Release Pipeline

CI job graph:

```
trigger (push to release | epic/* | feat/* | hotfix/* | dispatch)
  │
  ├── compute-version (ubuntu, sets VERSION + prerelease flag)
  │
  ├── build-linux-musl (ubuntu-latest, target: x86_64-unknown-linux-musl)
  ├── build-macos-x86  (macos-13, target: x86_64-apple-darwin)
  └── build-macos-arm  (macos-14, target: aarch64-apple-darwin)
       │
       └── release (needs: all builds; creates tag, uploads 6 artifacts)
```

Per-build steps: checkout → install Rust + target → install Node 20 →
`npm ci && npm run build` in `frontend/` → `cargo build --release --target
<triple>` → compute SHA256 → upload as job artifact.

Release job: download all artifacts → create tag → create GitHub Release
with `prerelease` flag set from the compute-version job output → attach
all 6 files.

### Dependencies

- GitHub Actions: `actions/checkout@v6`, `actions/cache` (cargo),
  `actions/upload-artifact@v4`, `actions/download-artifact@v4`,
  `softprops/action-gh-release@v2`.
- Rust toolchain: stable + `x86_64-unknown-linux-musl`,
  `x86_64-apple-darwin`, `aarch64-apple-darwin` targets.
- Node 20 for frontend build.
- `musl-tools` apt package on Linux runner for linking.

### Risks

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| macOS Gatekeeper blocks unsigned binary on first launch | High | Document `xattr -d com.apple.quarantine` workaround; future: add signing + notarization |
| Nightly VERSION string breaks `SeekiVersion::from_str()` parser | Medium | Resolve via Open Question 1 before CI build step lands |
| Frontend disconnect detection is unreliable across browsers | Medium | Poll `/api/update/status` with a short timeout; treat any 5 consecutive failures as "restarting" |
| Background poller wakes sleeping laptops / wastes battery | Low | 6h interval is gentle; `tokio::time::sleep` parks; no wakeups |
| GitHub rate limits on `/releases/latest` from many customer instances | Low | 15-min cache + 6h poller = ~4 requests/day/instance — within unauthenticated limits (60/hr/IP) |
| Token leak via browser devtools network panel | Accepted | Local-network tool; token rotates if user deletes `~/.config/seeki/update-token`; mutating endpoints already log calls |
| macOS ARM/Intel runner exhaust on GH Actions free tier | Low | Matrix is 3 jobs; acceptable for this repo's release cadence |

---

## Acceptance Scenarios

```gherkin
Feature: Auto-Update System

  Background:
    Given seeki is running as a systemd-managed service on the user's machine
    And GitHub Releases contains a release newer than the running binary
    And the release has assets matching the current platform

  Rule: Passive discovery — user doesn't need to hunt for updates

    Scenario: Badge appears when a new release is published
      Given the user has Settings open (or the app loaded) but is not on the Updates tab
      When the background poller checks GitHub and finds a newer release
      Then an "update available" badge appears on the Settings icon or Updates tab
      And opening the Updates panel shows the new version number and release notes

    Scenario: No badge on the current version
      Given the user has a fresh install matching the latest published release
      When the poller runs
      Then no badge appears
      And the Updates panel shows "You're up to date"

  Rule: One-click apply with visible restart

    Scenario: Apply installs the new release and returns the user to the new version
      Given an update is available
      And the user has opened the Updates panel
      When the user clicks "Install update"
      Then the frontend shows a "Restarting..." overlay
      And the backend verifies SHA256, writes the new binary, and exits
      And systemd restarts the process
      And the frontend polls /api/update/status every 2 seconds
      And within 30 seconds the page reloads on the new version
      And the Rollback button is now visible

    Scenario: SHA256 mismatch aborts the apply cleanly
      Given the release asset's SHA256 sidecar does not match the downloaded bytes
      When the user clicks "Install update"
      Then the backend rejects with a clear error
      And the running binary is untouched
      And the frontend shows the error to the user without reloading

    Scenario: Missing SHA256 sidecar aborts before download
      Given a release is published without a .sha256 file
      When the user clicks "Install update"
      Then the backend responds with "Release is missing a SHA256 sidecar"
      And no download is attempted

  Rule: Rollback is a safety net, not a regression vector

    Scenario: Rollback reverts to the prior binary
      Given the user has applied an update
      And the Rollback button is visible
      When the user clicks "Rollback"
      Then the frontend shows the same "Restarting..." overlay
      And the prior binary is restored
      And after restart, the Rollback button is no longer visible
      And the running version matches the pre-apply version

    Scenario: No rollback after fresh install
      Given the user has never applied an update since install
      When the user opens the Updates panel
      Then the Rollback button is not rendered

  Rule: Multi-platform asset selection

    Scenario Outline: select_asset picks the correct binary for the current platform
      Given the release contains <available_assets>
      And the host platform is <os>/<arch>
      When select_asset is invoked
      Then the chosen asset name is <expected>

      Examples:
        | available_assets                                                | os    | arch    | expected                      |
        | musl,darwin-x86,darwin-arm                                      | linux | x86_64  | seeki-x86_64-linux-musl       |
        | musl,darwin-x86,darwin-arm                                      | macos | x86_64  | seeki-x86_64-darwin           |
        | musl,darwin-x86,darwin-arm                                      | macos | aarch64 | seeki-aarch64-darwin          |
        | musl only                                                       | macos | x86_64  | (none — returns None)         |

  Rule: CI produces verifiable artifacts

    Scenario: release branch push creates a stable release with 6 assets
      Given a push to the release branch
      When the workflow completes
      Then a GitHub Release is created with prerelease: false
      And the release has exactly 6 assets: 3 binaries + 3 sha256 files
      And each binary's SHA256 matches its sidecar

    Scenario: epic branch push creates a pre-release
      Given a push to an epic/* branch
      When the workflow completes
      Then a GitHub Release is created with prerelease: true
      And the tag includes a nightly marker matching the agreed scheme
      And users with pre_release_channel: false do not see it in their poller

  Rule: Token delivery is scoped to same-origin

    Scenario: Frontend fetches the token to call mutating endpoints
      Given the frontend is served by the seeki backend on 127.0.0.1:3141
      When the Updates panel mounts
      Then GET /api/update/token returns the bearer token as JSON
      And the token is cached in memory for the session

    Scenario: Cross-origin request to /api/update/token is rejected
      Given an external page at http://evil.example makes a same-origin-less fetch
      When the browser sends the request without a matching Origin header
      Then the endpoint responds with 403 Forbidden
      And no token is leaked
```

---

## Task Breakdown

| ID   | Task | Priority | Dependencies | Status |
|:-----|:-----|:---------|:-------------|:-------|
| T1   | Resolve Open Question 1 (nightly version scheme) | High | None | pending |
| T2   | Resolve Open Question 3 (token delivery pattern) | High | None | pending |
| T3   | Extend `.github/workflows/release.yml` with matrix build (musl + darwin-x86 + darwin-arm) | High | T1 | pending |
| T3.1 | Add compute-version job that rewrites VERSION for non-release branches per T1 | High | T1 | pending |
| T3.2 | Add build matrix jobs with frontend + cargo build + SHA256 computation | High | T3 | pending |
| T3.3 | Refactor release job to consume uploaded artifacts and attach all 6 files | High | T3.2 | pending |
| T3.4 | Add triggers for `epic/*`, `feat/*`, `hotfix/*` branches with `prerelease: true` | High | T3 | pending |
| T4   | Update `select_asset()` in `src/update/github.rs` for multi-platform detection | High | None | pending |
| T4.1 | Unit tests for `select_asset()` covering linux/musl, macos/x86, macos/arm, unsupported platforms | High | T4 | pending |
| T5   | Replace one-shot poller in `main.rs` with a loop that re-reads `UpdateSettings.poll_interval_hours` each cycle; `0` parks indefinitely on a settings-change notify | High | T5.0 | pending |
| T5.0 | Add `poll_interval_hours: u32` to `UpdateSettings` with serde default of 6; extend `PATCH /api/update/settings` to accept and validate it | High | None | pending |
| T6   | Add `GET /api/update/token` handler and route it | High | T2 | pending |
| T6.1 | Same-origin / localhost guard on the token endpoint | High | T6 | pending |
| T6.2 | Integration test: valid same-origin request returns token; cross-origin rejected | Med | T6.1 | pending |
| T7   | Wire `UpdatesPanel.svelte` to real apply / rollback / upload endpoints | High | T6 | pending |
| T7.1 | Implement "Restarting..." overlay with disconnect detection + polling | High | T7 | pending |
| T7.2 | Page reload after successful reconnect | High | T7.1 | pending |
| T7.3 | Hide Rollback button when `previous_exists: false` | High | T7 | pending |
| T7.4 | Release notes preview from `body` field | Med | T7 | pending |
| T7.5 | Poller interval selector in Updates panel (Hourly / 6-hourly / Daily / Never) wired to `PATCH /api/update/settings` | High | T5.0, T7 | pending |
| T8   | Update-available badge in `SettingsNav.svelte` / sidebar | High | T7 | pending |
| T8.1 | Shared store for update status (`frontend/src/lib/stores.ts`) | High | T8 | pending |
| T9   | Attach bearer token to mutating frontend calls in `api.ts` | High | T6 | pending |
| T10  | WIP upload UI wiring for sg-server ops use | Med | T7 | pending |
| T11  | End-to-end manual test on sg-server: apply, rollback, verify badge | High | T3, T4, T5, T6, T7 | pending |
| T12  | Document macOS Gatekeeper workaround in README | Med | T3 | pending |

---

## Exit Criteria

- [ ] All Must-Have scenarios pass manual testing on sg-server (Ubuntu 20
      musl) and at least one macOS machine
- [ ] `cargo test` green on the `select_asset()` platform detection suite
- [ ] CI workflow green on both a push to `release` (stable) and a push to
      an `epic/*` branch (pre-release), with 6 assets per release
- [ ] No regressions on the existing `/api/update/*` endpoints
- [ ] `UpdatesPanel.svelte` no longer contains the "patcher backend
      unavailable" placeholder copy
- [ ] `cargo clippy` green
- [ ] Manual rollback test: apply → rollback → verify version reverts and
      Rollback button hides
- [ ] All four Open Questions remain resolved (see table above)
- [ ] Setting `poll_interval_hours` to `0` (Never) in Settings stops the
      poller within the current cycle without requiring a restart

---

## References

- Existing update module: `src/update/{mod,github,swap,wip,auth,version}.rs`
- Existing update API: `src/api/update.rs`
- Existing workflow: `.github/workflows/release.yml`
- Brainstorm context: user approved design 2026-04-16 (targets Ubuntu
  20/22/24/26 + Arch + macOS Intel + macOS ARM; UX = notify + one-click +
  rollback; repo already public)
- Related: today's hotfix commit `bd5f6f7` (resolves partial #71 merge so
  the current branch compiles)

---

## Verification

**Local verification (sg-server + dev machine)**:

1. Build a musl binary locally: `cargo build --release --target
   x86_64-unknown-linux-musl`.
2. Tag a test release manually: `gh release create v26.5.0.3aN260416
   --prerelease --notes "Test" target/.../seeki#seeki-x86_64-linux-musl
   <sha256-file>` (once CI is in place, this is automatic).
3. Restart seeki on sg-server with the current binary; wait up to 6h (or
   call `POST /api/update/check` manually) and verify the Updates panel
   shows the new version.
4. Click Install update → verify overlay → verify page reload on new
   version → verify Rollback button visible.
5. Click Rollback → verify reversion → verify Rollback button hides.

**CI verification**:

- Push to `epic/61-epic-ux-tuning` → confirm pre-release created with 6
  assets, SHA256 sums match.
- Push to `release` → confirm stable release with 6 assets.

**Unit test**: `cargo test --package seeki select_asset` covers the four
platform cases including the unsupported-platform path.

---

*Authored by: Clault KiperS 4.6*
