# Feature: Auto-Update System + Multi-Platform CI Release Pipeline

## Purpose

Finish the update feature that already exists in pieces across the repo so that:

- GitHub Releases always contain installable binaries and matching `.sha256` sidecars
- running SeeKi instances discover new releases without a browser reload
- the current Settings workspace can apply and roll back updates without terminal access

This is a spec for implementation work only. It does not change the product intent that was already approved.

---

## Current Repo Reality

The spec needs to align with what is already in the repo today:

- `.github/workflows/release.yml` creates tags and GitHub Releases, but uploads no binaries or checksums.
- that workflow hardcodes `actions/checkout` to `ref: release`, so any future non-`release` trigger would currently build the wrong branch.
- `src/update/github.rs` only selects Linux assets today via `select_asset()`.
- `src/main.rs` performs a single background update check at startup and never schedules another one.
- `src/update/github.rs` uses a single shared release cache entry, so switching `pre_release_channel` can reuse stale data for up to 15 minutes.
- `src/api/update.rs` already exposes `/api/update/status`, `/api/update/check`, `/api/update/apply`, `/api/update/rollback`, `/api/update/wip`, and mutating endpoints are already protected by a bearer token.
- there is no frontend bootstrap path for that bearer token yet.
- the current in-app settings route renders [`frontend/src/components/settings/UpdatesPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/settings/UpdatesPanel.svelte), and that component is still a placeholder.
- a legacy modal [`frontend/src/components/SettingsPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/SettingsPanel.svelte) already contains much of the desired update UX, but it is not the primary settings surface and its mutating calls still lack auth headers.
- [`frontend/src/lib/api.ts`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/lib/api.ts) currently has duplicate update helpers (`checkForUpdates` and `checkForUpdate`) with conflicting response expectations.
- [`frontend/src/components/Sidebar.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/Sidebar.svelte) and [`frontend/src/components/SettingsNav.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/SettingsNav.svelte) already support update badges via props, but [`frontend/src/App.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/App.svelte) does not wire those props through yet.

---

## User Outcome

As a SeeKi operator, I can leave the app open, see that a newer release is available, open Settings > Updates, install it in one click, wait through the restart, and land back on the updated version. If the new binary is bad, I can roll back once in the same UI.

---

## Success Condition

This feature is complete when all of the following are true:

- pushing to `release` publishes a stable GitHub Release with:
  - `seeki-x86_64-linux-musl`
  - `seeki-x86_64-linux-musl.sha256`
  - `seeki-x86_64-darwin`
  - `seeki-x86_64-darwin.sha256`
  - `seeki-aarch64-darwin`
  - `seeki-aarch64-darwin.sha256`
- pushing to a supported work branch publishes a prerelease with the same six assets
- a running SeeKi instance notices a newer release without a browser refresh
- the Settings workspace, not just the legacy modal, can:
  - check for updates
  - show release notes
  - apply a release build
  - upload and apply a WIP build
  - roll back when `.prev` exists
- after apply or rollback, the browser reconnects automatically and reloads on the returned server

---

## Scope

### In Scope

- CI artifact builds and GitHub Release publishing
- Linux musl + macOS Intel + macOS Apple Silicon asset selection
- long-lived background polling with configurable interval
- frontend token bootstrap for mutating update endpoints
- current Settings workspace update UI
- update badges in the settings navigation surfaces
- docs and tests required to keep the feature maintainable

### Out of Scope

- Windows binaries
- signed releases, notarization, or provenance attestation
- delta updates or binary patching
- automatic restart without a service manager supervising the binary
- redesigning the legacy modal settings experience beyond keeping it on the shared API path
- macOS universal binaries

### Explicit v1 Constraints

- WIP upload remains an internal Linux-oriented path. The current WIP validator only accepts x86_64 ELF binaries, and broadening that validator is not part of this spec.
- unsigned macOS binaries are acceptable for v1; the user-facing docs must explain the quarantine workaround.

---

## Resolved Decisions

| Topic | Decision |
|:------|:---------|
| Canonical update surface | The primary implementation target is [`frontend/src/components/settings/UpdatesPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/settings/UpdatesPanel.svelte). The legacy modal may stay mounted, but it must consume the same updated API helpers while it exists. |
| Supported prerelease triggers | Use the repo's existing branch vocabulary: `task/*`, `feature/*`, `bug/*`, `hotfix/*`, plus `workflow_dispatch`. Do not introduce `feat/*` or `epic/*` just for this feature. |
| Prerelease versioning | Prerelease tags are derived in CI from the checked-out workspace version and must remain compatible with `SeekiVersion`. Use a suffix that starts with a letter and stays alphanumeric, for example `26.5.0.3n260416g1a2b3c4`. This is a build-time version only and is not committed back to the branch. |
| Poll interval values | `poll_interval_hours` accepts `0`, `1`, `6`, or `24`, where `0` means `Never`. Default is `6`. |
| Token bootstrap | Add `GET /api/update/token` for the browser UI only. Return the bearer token over same-origin requests and cache it in memory on the frontend. |
| Release notes source | Use the GitHub Release body already returned by the backend's release fetch path. Stable and prerelease workflows must publish a non-empty body so the UI has meaningful text to render. |

---

## Technical Plan

### 1. CI and Release Publishing

**Files**

- [`.github/workflows/release.yml`](/home/kiriketsuki/dev/Personal/seeKi/.github/workflows/release.yml)
- [`.github/CI-CD-Guide.md`](/home/kiriketsuki/dev/Personal/seeKi/.github/CI-CD-Guide.md)
- [`README.md`](/home/kiriketsuki/dev/Personal/seeKi/README.md)

**Required changes**

- Expand the workflow trigger set to:
  - `release`
  - `task/*`
  - `feature/*`
  - `bug/*`
  - `hotfix/*`
  - `workflow_dispatch`
- Stop hardcoding `actions/checkout` to `ref: release`; the workflow must check out the triggering ref.
- Keep the existing stable-only `release` branch behavior, including syncing `VERSION` from `main`, but gate that logic behind `if: github.ref_name == 'release'`.
- Add a metadata job that computes:
  - `build_version`
  - `tag_name`
  - `release_name`
  - `prerelease`
  - release body metadata such as branch name and commit SHA
- For prerelease branches:
  - derive a unique prerelease version in the workflow workspace
  - write it into the workspace `VERSION` file before `cargo build`
  - do not commit or push that rewritten version back to the source branch
- Build a three-job matrix:
  - `x86_64-unknown-linux-musl` on `ubuntu-latest`
  - `x86_64-apple-darwin` on `macos-13`
  - `aarch64-apple-darwin` on `macos-14`
- Each build job must:
  - install Node 20
  - run `npm ci` and `npm run build` in `frontend/`
  - build the Rust release binary for its target
  - rename the output binary to the exact asset name expected by `select_asset()`
  - generate a sidecar file named `<asset>.sha256`
  - upload both files as workflow artifacts
- The release job must download all artifacts and publish exactly six files to the GitHub Release.

**Implementation note**

The release asset name is part of the runtime contract. The backend currently derives the checksum URL by appending `.sha256` to the binary asset URL, so the workflow must preserve that exact naming convention.

### 2. Backend Update Runtime

**Files**

- [`src/update/github.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/github.rs)
- [`src/update/mod.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/mod.rs)
- [`src/main.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/main.rs)
- [`src/api/update.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/api/update.rs)
- [`src/api/mod.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/api/mod.rs)

**Required changes**

- Make release caching channel-aware.
  - Today there is one cache entry for both stable and prerelease queries.
  - After this change, a stable check must never satisfy a prerelease request, or vice versa.
  - A split cache keyed by `include_prerelease` is acceptable.
- Extend `UpdateSettings` with `poll_interval_hours`.
  - persist to `~/.seeki/update.json`
  - serde-default to `6`
  - preserve backwards compatibility with existing files that only contain `pre_release_channel` and `last_checked`
- Add a poller wake mechanism to `UpdateState` so settings changes can interrupt the sleep cycle.
  - `tokio::sync::Notify` is sufficient
- Replace the one-shot startup task in `main.rs` with a long-lived poller loop that:
  - runs one check shortly after startup
  - reads current settings at the start of each cycle
  - sleeps for the selected interval
  - waits on the settings-change notify when interval is `0`
  - forces a refresh when `pre_release_channel` changes so the cache does not mask the new channel
  - updates `last_checked` when a GitHub fetch completes successfully
- Expand `select_asset()` to cover:
  - `linux/x86_64` -> prefer `seeki-x86_64-linux-musl`, optionally fall back to `seeki-x86_64-linux-gnu`
  - `macos/x86_64` -> `seeki-x86_64-darwin`
  - `macos/aarch64` -> `seeki-aarch64-darwin`
  - unsupported platforms -> `None`

### 3. API Contract Cleanup

The current frontend update code is hard to wire correctly because the API surfaces are inconsistent. This spec standardizes the browser-facing contract enough to support the UI without requiring the frontend to reason about raw asset lists.

**Status response**

`GET /api/update/status` returns:

```json
{
  "current": "26.5.0.3a",
  "latest": "26.5.0.3n260416g1a2b3c4",
  "pre_release_channel": true,
  "poll_interval_hours": 6,
  "update_available": true,
  "previous_exists": false,
  "last_checked": "2026-04-16T09:00:00Z",
  "release_notes": "..."
}
```

Notes:

- `latest` and `release_notes` are `null` when no release is cached.
- `release_notes` should come from the cached GitHub Release body so the UI can render it without a forced manual check.

**Manual check**

`POST /api/update/check`:

- forces a GitHub refresh
- updates the release cache
- updates `last_checked`
- returns the same shape as `GET /api/update/status`

This removes the need for duplicate frontend types for "status" versus "check" responses.

**Settings patch**

`PATCH /api/update/settings` accepts:

```json
{
  "pre_release_channel": true,
  "poll_interval_hours": 6
}
```

Rules:

- both fields are optional
- `poll_interval_hours` must be one of `0`, `1`, `6`, `24`
- a successful write notifies the background poller so the new interval or channel takes effect immediately

**Token bootstrap**

`GET /api/update/token` returns:

```json
{
  "token": "<64-char-hex>"
}
```

Rules:

- no bearer token required
- same-origin only
- set `Cache-Control: no-store`
- reject mismatched `Origin`
- when `Origin` is absent, accept only when `Referer` resolves to the same scheme/host/port as the request host; otherwise reject with `403`

### 4. Frontend Integration

**Files**

- [`frontend/src/components/settings/UpdatesPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/settings/UpdatesPanel.svelte)
- [`frontend/src/components/SettingsPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/SettingsPanel.svelte)
- [`frontend/src/components/SettingsContent.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/SettingsContent.svelte)
- [`frontend/src/components/SettingsNav.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/SettingsNav.svelte)
- [`frontend/src/components/Sidebar.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/Sidebar.svelte)
- [`frontend/src/App.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/App.svelte)
- [`frontend/src/lib/api.ts`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/lib/api.ts)
- [`frontend/src/lib/types.ts`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/lib/types.ts)

**Required changes**

- Make `UpdatesPanel.svelte` the working implementation for Settings > Updates.
- Reuse the good parts of `SettingsPanel.svelte` instead of re-inventing them:
  - success and error banners
  - confirmation step before install / rollback / WIP apply
  - restart overlay and reconnect polling
  - WIP upload handling
- Consolidate update API helpers in `api.ts`.
  - remove or repurpose the duplicated `checkForUpdates` / `checkForUpdate` split
  - add one in-memory token cache
  - automatically attach `Authorization: Bearer <token>` to:
    - `POST /api/update/apply`
    - `POST /api/update/wip`
    - `POST /api/update/rollback`
- Normalize update types in `types.ts` so the settings UI has one shared response model.
- Extend the current settings route data flow instead of introducing a new global update store by default.
  - `App.svelte` already owns `updateStatus` and `updateAvailable`
  - wire that state down to `SettingsNav` and `UpdatesPanel`
  - only add a dedicated store if implementation complexity proves otherwise
- Wire both existing badge surfaces:
  - pass `showSettingsBadge={updateAvailable}` into `Sidebar`
  - pass `showUpdateBadge={updateAvailable}` into `SettingsNav`
  - keep the footer gear badge working as it does today
- Add settings controls for:
  - pre-release channel
  - poll interval (`Hourly`, `Every 6 hours`, `Daily`, `Never`)
- Release notes must render in the current settings panel when cached release data exists.

**Legacy modal handling**

[`frontend/src/components/SettingsPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/SettingsPanel.svelte) is not the primary UI target, but it is still mounted from `App.svelte`. The implementation must either:

- switch it to the same shared update helpers so it continues to work, or
- intentionally remove update actions from it in the same PR

What is not acceptable is leaving two different update surfaces where one works and the other still fails with `401`.

### 5. Docs and Verification

**Files**

- [`README.md`](/home/kiriketsuki/dev/Personal/seeKi/README.md)
- [`.github/CI-CD-Guide.md`](/home/kiriketsuki/dev/Personal/seeKi/.github/CI-CD-Guide.md)

**Required changes**

- document that release builds now publish download artifacts, not just tags
- document supported platforms for auto-update
- document the macOS quarantine workaround:
  - `xattr -d com.apple.quarantine /path/to/seeki`
- update CI docs so prerelease publishing matches the actual trigger set and branch vocabulary in the repo

---

## Acceptance Scenarios

```gherkin
Feature: Auto-update and CI release artifacts

  Background:
    Given SeeKi is running under a service manager
    And GitHub Releases contains binaries and .sha256 sidecars for the current platform

  Scenario: release branch push publishes a stable release with six assets
    Given a push to the release branch
    When the workflow completes
    Then GitHub Release v{VERSION} exists
    And prerelease is false
    And the release contains exactly six assets

  Scenario: feature branch push publishes a prerelease from the current branch
    Given a push to feature/89-auto-update-system-and-multi-platform-ci-release-pipeline
    When the workflow completes
    Then the workflow builds the checked-out feature branch, not the release branch
    And the GitHub Release is marked prerelease
    And the tag uses the CI-derived prerelease version
    And the release contains exactly six assets

  Scenario: background polling surfaces a new release without a browser refresh
    Given the app is open in the browser
    And poll_interval_hours is 6
    When the backend poller sees a newer release
    Then update_available becomes true in /api/update/status
    And the settings badge appears in the shell
    And the Updates panel shows the latest version and release notes

  Scenario: changing pre-release channel forces a fresh channel-aware lookup
    Given the stable release is cached
    When the user enables pre-release updates
    Then the backend does not reuse the stable cache entry for the pre-release lookup
    And the next status response reflects the prerelease channel

  Scenario: user installs a release update from the current Settings workspace
    Given update_available is true
    When the user clicks Install update in Settings > Updates
    Then the frontend fetches the update token
    And the backend verifies the binary against its .sha256 sidecar
    And the frontend shows a restarting overlay
    And the browser reconnects and reloads onto the new version
    And previous_exists becomes true

  Scenario: rollback returns to the prior binary
    Given a release update was applied successfully
    When the user clicks Rollback
    Then the frontend shows the same restarting overlay
    And the app reloads on the prior version
    And previous_exists becomes false

  Scenario: cross-origin token request is rejected
    Given a page from another origin calls GET /api/update/token
    When the Origin or Referer does not match the request host
    Then the response is 403
    And no token is returned
```

---

## Test Plan

### Backend tests

- add unit tests for `select_asset()` covering:
  - `linux/x86_64`
  - `macos/x86_64`
  - `macos/aarch64`
  - unsupported platforms
- add unit tests for `UpdateSettings` serde defaults and validation of `poll_interval_hours`
- add API tests for:
  - `GET /api/update/token` same-origin accept path
  - `GET /api/update/token` cross-origin reject path
  - `PATCH /api/update/settings` with valid and invalid intervals
- add cache tests proving stable and prerelease lookups do not share the same cache entry

### Frontend tests

- add tests for token caching and auth-header attachment in `frontend/src/lib/api.ts`
- add tests that the update status/check helpers now agree on one response shape
- add UI coverage for:
  - badge visibility when `update_available` is true
  - poll interval selector wiring
  - release notes rendering
  - restart overlay behavior after apply / rollback

### Manual verification

- push a stable build through `release` and confirm six published assets
- push a prerelease build through a `feature/*` branch and confirm six published assets
- on Linux musl:
  - detect update
  - apply update
  - reconnect
  - roll back
- on macOS:
  - detect update
  - apply update
  - reconnect
  - verify the quarantine workaround is documented if needed

---

## Task Breakdown

| ID | Task | Files | Priority | Depends On |
|:---|:-----|:------|:---------|:-----------|
| CI-1 | Refactor release workflow to build the triggering ref, derive stable/prerelease metadata, and publish six assets | `release.yml` | High | None |
| CI-2 | Update CI/release docs to reflect the new workflow behavior and branch triggers | `.github/CI-CD-Guide.md`, `README.md` | Med | CI-1 |
| BE-1 | Make release caching channel-aware and extend platform asset selection | `src/update/github.rs` | High | None |
| BE-2 | Add `poll_interval_hours`, settings-change wakeups, and long-lived background polling | `src/update/mod.rs`, `src/main.rs`, `src/api/update.rs` | High | BE-1 |
| BE-3 | Standardize update API contracts and add `GET /api/update/token` | `src/api/update.rs`, `src/api/mod.rs` | High | BE-2 |
| FE-1 | Consolidate update API helpers and types, including in-memory token caching | `frontend/src/lib/api.ts`, `frontend/src/lib/types.ts` | High | BE-3 |
| FE-2 | Implement the working Settings > Updates experience using the shared update API path | `frontend/src/components/settings/UpdatesPanel.svelte`, `frontend/src/components/SettingsContent.svelte` | High | FE-1 |
| FE-3 | Wire update badges through the current app shell | `frontend/src/App.svelte`, `frontend/src/components/Sidebar.svelte`, `frontend/src/components/SettingsNav.svelte` | High | FE-2 |
| FE-4 | Keep the legacy modal on the same auth-capable update API path, or remove conflicting update actions from it | `frontend/src/components/SettingsPanel.svelte` | Med | FE-1 |
| TEST-1 | Add backend/frontend automated coverage and run manual apply/rollback verification | backend + frontend test files | High | CI-1, BE-3, FE-4 |

---

## Risks and Mitigations

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| Prerelease workflow builds the wrong code because checkout stays pinned to `release` | High | Make `checkout` follow the triggering ref and gate stable-only steps behind `github.ref_name == 'release'` |
| Stable and prerelease channels leak into each other through the shared cache | High | Key the cache by channel and force refresh on channel change |
| Two frontend settings surfaces drift apart | High | Treat `UpdatesPanel.svelte` as canonical and require the legacy modal to use the same update API path |
| Token endpoint is too permissive | Medium | Same-origin checks only, `Cache-Control: no-store`, no CORS expansion |
| Token endpoint is too strict for the browser UI | Medium | Allow either matching `Origin` or matching `Referer`; test both allowed and rejected paths |
| Unsigned macOS binaries alarm users | High | Document the quarantine workaround now; defer notarization |
| Release notes panel is empty on prereleases | Medium | Ensure CI writes a non-empty release body for automated prereleases |

---

## Exit Criteria

- [ ] `release.yml` publishes six assets for stable releases and prereleases
- [ ] prerelease publishing uses `task/*`, `feature/*`, `bug/*`, or `hotfix/*` triggers and builds the actual triggering ref
- [ ] `GET /api/update/status` exposes `poll_interval_hours` and cached `release_notes`
- [ ] changing `pre_release_channel` does not reuse a stale stable-cache entry
- [ ] the current Settings workspace can check, apply, roll back, and upload WIP builds
- [ ] mutating update calls from the browser succeed without manual token pasting
- [ ] both the shell badge and the settings-nav badge reflect `update_available`
- [ ] `SettingsPanel.svelte` is either on the shared auth-capable path or no longer exposes broken update actions
- [ ] backend and frontend tests covering the new contracts are green
- [ ] README and CI docs match the shipped workflow

---

## References

- [`src/update/mod.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/mod.rs)
- [`src/update/github.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/github.rs)
- [`src/update/swap.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/swap.rs)
- [`src/update/wip.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/wip.rs)
- [`src/update/auth.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/auth.rs)
- [`src/update/version.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/update/version.rs)
- [`src/api/update.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/api/update.rs)
- [`src/main.rs`](/home/kiriketsuki/dev/Personal/seeKi/src/main.rs)
- [`frontend/src/components/settings/UpdatesPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/settings/UpdatesPanel.svelte)
- [`frontend/src/components/SettingsPanel.svelte`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/components/SettingsPanel.svelte)
- [`frontend/src/lib/api.ts`](/home/kiriketsuki/dev/Personal/seeKi/frontend/src/lib/api.ts)
- [`.github/workflows/release.yml`](/home/kiriketsuki/dev/Personal/seeKi/.github/workflows/release.yml)
