# Feature: Frontend Scaffold & Integration

## Overview

**User Story**: As a developer building SeeKi, I want a working Svelte 5 frontend project with a glassmorphism theme system and rust-embed integration so that the UI scaffold is ready for feature development and the release binary serves everything from a single file.

**Problem**: The `frontend/` directory is empty. There is no frontend build pipeline, no theme system, no component skeleton, and no mechanism to embed frontend assets into the Rust binary. All subsequent frontend epics (data grid, toolbar, setup wizard) are blocked until this scaffold exists.

**Out of Scope**:
- Interactive features (sorting, filtering, search, pagination) — Epic 3
- Table list in sidebar — Epic 3, Issue #16
- Global search, column hide/show, CSV export wiring — Epic 4
- Setup wizard UI — Epic 5, Issue #25
- Display formatting (timestamps, booleans, NULLs) — Epic 3, Issue #20
- RevoGrid theme override CSS — Epic 3 (needs functional grid first)
- Dark mode — Nice-to-Have in MVP spec
- SvelteKit, SSR, or file-based routing

---

## Success Condition

> This feature is complete when `just dev` starts both the Vite dev server and Rust backend with a single command, `just dev-mock` serves the UI with mock data and no backend, the glassmorphism layout shell renders in the browser matching the approved v4 mockup, and `just build` produces a single Rust binary that serves the embedded frontend at `/`.

---

## Open Questions

| # | Question | Raised By | Resolved |
|:--|:---------|:----------|:---------|
| 1 | RevoGrid Svelte 5 compatibility | MVP spec | [x] Confirmed: `@revolist/svelte-datagrid` v4.21+ supports Svelte 5 since v4.11.0 |
| 2 | `@aspect-ui/revogrid-svelte` package name | MVP spec | [x] Does not exist — correct package is `@revolist/svelte-datagrid` |
| 3 | backdrop-filter performance on grid card | MVP spec | [x] Keeping blur on all surfaces per approved mockup; revisit if profiling shows issues |

---

## Scope

### Must-Have

- **Svelte 5 + Vite project scaffold**: `frontend/` directory with Vite, Svelte 5, TypeScript, `@revolist/svelte-datagrid`, `lucide-svelte`. Acceptance: `npm run dev` starts Vite with HMR; `npm run build` produces `frontend/dist/`.
- **Vite dev proxy**: `/api/*` requests proxy to `http://127.0.0.1:3141`. Acceptance: API calls from the frontend reach the Rust backend in dev mode.
- **Justfile dev commands**: `just dev` (cargo run + vite dev, fail gracefully if no DB), `just dev-mock` (vite only with mock API), `just build` (npm build + cargo build --release). Acceptance: each command works as described with a single invocation.
- **Glassmorphism theme tokens**: `sk-`-prefixed CSS custom properties for palette (#2F4858, #00A9A5, #F5F0EB, #8a7a6c, #a0917f, #b0a090), glass surfaces (backdrop-filter blur 20px/24px, varying alpha), spacing, radius, shadows, typography (Inter + JetBrains Mono). Acceptance: all UI elements use CSS variables, no hardcoded colors.
- **Layout shell (App.svelte)**: Flexbox layout with sidebar + main area (toolbar + grid + status bar). Conditional `{#if isSetup}` for future wizard view. Acceptance: layout matches the approved v4 mockup structure.
- **Sidebar component**: Collapse/expand toggle persisted to localStorage. Branding header (title + subtitle via props). Attribution footer ("Powered by SeeKi"). Default `<slot>` for future table list. Width: 210px expanded, 48px collapsed. Acceptance: collapse toggle works and persists across page reloads.
- **Toolbar component**: Static placeholder with disabled search input, Columns button, Export button. Table name + row count display via props. Acceptance: renders matching mockup layout.
- **DataGrid component**: Mounts `<RevoGrid>` with basic config. Accepts columns + rows as props. Renders mock data in dev-mock mode. Acceptance: RevoGrid renders a grid with columns and rows.
- **StatusBar component**: Static "Showing X - Y of Z" text via props. Disabled Prev/Next buttons. Acceptance: renders matching mockup layout.
- **Mock API (mock.ts)**: 5 fake tables with realistic columns and 50 rows of plausible data per page. Acceptance: `just dev-mock` renders a functional-looking UI with no backend.
- **API fetch wrapper (api.ts)**: `VITE_MOCK=true` returns mock data immediately; otherwise tries fetch to backend, falls back to mock on network error with console.warn. Acceptance: UI works with and without a running backend.
- **TypeScript types (types.ts)**: Shared types for TableInfo, ColumnInfo, QueryResult matching the backend API contracts. Acceptance: all API calls are typed.
- **rust-embed asset serving (embed.rs)**: `#[derive(RustEmbed)]` over `frontend/dist/`. Axum fallback handler serving static files with SPA fallback to index.html. Cache: `/assets/*` immutable (1 year), `index.html` no-cache. Acceptance: `cargo build --release` binary serves the frontend at `/`.
- **SPA routing fallback**: Non-API, non-asset paths serve index.html. Acceptance: refreshing `/table/vehicles` serves the SPA, not a 404.

### Should-Have

- **Pagination controls in StatusBar**: prop-driven page number and prev/next callback wiring (even if handlers are no-ops in this epic) to avoid rework in Epic 3.

### Nice-to-Have

- **Animated sidebar collapse transition**: smooth width transition with CSS animation on collapse/expand.

---

## Technical Plan

**Affected Components**:

| Layer | Files | Changes |
|:------|:------|:--------|
| Frontend (new) | `frontend/package.json`, `frontend/vite.config.ts`, `frontend/tsconfig.json`, `frontend/index.html` | Scaffold Svelte 5 + Vite + TS project |
| Frontend (new) | `frontend/src/main.ts`, `frontend/src/App.svelte`, `frontend/src/app.css` | Entry point, layout shell, global styles |
| Frontend (new) | `frontend/src/theme/tokens.css` | CSS custom properties (palette, glass, spacing, typography) |
| Frontend (new) | `frontend/src/lib/api.ts`, `frontend/src/lib/mock.ts`, `frontend/src/lib/types.ts` | Fetch wrapper, mock data, shared types |
| Frontend (new) | `frontend/src/components/Sidebar.svelte`, `Toolbar.svelte`, `DataGrid.svelte`, `StatusBar.svelte` | Component shells |
| Backend (new) | `src/embed.rs` | rust-embed struct + axum fallback handler |
| Backend (mod) | `src/main.rs` | Add `mod embed;`, add `.fallback(embed::handler)` to Router |
| Build (new) | `Justfile` | `dev`, `dev-mock`, `build` recipes |

**Data Model Changes**: None — frontend-only epic plus a new Rust module for static file serving.

**API Contracts**: No new API endpoints. Frontend consumes existing endpoints:
- `GET /api/tables` — list tables
- `GET /api/tables/{table}/columns` — column metadata
- `GET /api/tables/{table}/rows?page=&page_size=` — paginated rows
- `GET /api/config/display` — branding and column display names

**Dependencies**:
- `@revolist/svelte-datagrid` ^4.21 — RevoGrid Svelte 5 wrapper
- `lucide-svelte` — SVG icon library
- `just` — command runner (must be installed on dev machine)
- `rust-embed` 8.x — already in Cargo.toml
- Inter font (Google Fonts CDN) — loaded in index.html
- JetBrains Mono font (Google Fonts CDN) — loaded in index.html

**Risks**:

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| RevoGrid Svelte 5 wrapper has undiscovered bugs | Low | Wrapper is actively maintained (v4.21+); fallback to raw `<revo-grid>` web component if needed |
| backdrop-filter performance on large grids | Low | Monitor during Epic 3 when real data flows in; can reduce grid card blur if needed |
| `just` not installed on dev machines | Low | Document in README; Justfile recipes are simple enough to run manually |
| rust-embed empty dist/ on first clone | Low | Backend compiles fine with empty embed; document `just build` in README |
| Google Fonts CDN dependency | Low | Fonts are aesthetic, not functional; system-ui fallback in font stack |

---

## Acceptance Scenarios

```gherkin
Feature: Frontend Scaffold & Integration
  As a developer building SeeKi
  I want a working frontend scaffold with theme and embedding
  So that subsequent epics can build features on a solid foundation

  Rule: Svelte 5 project scaffold

    Scenario: Vite dev server starts with HMR
      Given the frontend/ directory contains a Svelte 5 + Vite project
      When I run "npm run dev" in frontend/
      Then the Vite dev server starts on port 5173
      And changes to .svelte files trigger hot module replacement

    Scenario: Production build produces dist/
      Given the frontend/ project is set up
      When I run "npm run build" in frontend/
      Then frontend/dist/ is created with index.html and hashed assets

    Scenario: Dev proxy routes API calls to backend
      Given Vite dev server is running on :5173 and backend on :3141
      When the frontend fetches /api/tables
      Then the request proxies to http://127.0.0.1:3141/api/tables

  Rule: Justfile dev workflow

    Scenario: Single command starts full stack
      Given the project has a Justfile
      When I run "just dev"
      Then cargo run starts in the background
      And vite dev starts in the foreground
      And the terminal shows both processes' output

    Scenario: Mock mode runs without backend
      Given no Rust backend or database is running
      When I run "just dev-mock"
      Then only the Vite dev server starts
      And the UI renders with mock data from mock.ts

    Scenario: Release build produces single binary
      Given the frontend/ project builds successfully
      When I run "just build"
      Then "npm run build" runs first producing frontend/dist/
      And "cargo build --release" embeds dist/ into the binary

  Rule: Glassmorphism theme

    Scenario: Theme tokens are applied to all surfaces
      Given tokens.css defines sk-prefixed CSS custom properties
      When the app renders in a browser
      Then the sidebar uses backdrop-filter: blur(20px) with white @ 50% alpha
      And the toolbar uses backdrop-filter: blur(20px) with white @ 40% alpha
      And the grid card uses backdrop-filter: blur(24px) with white @ 55% alpha
      And no UI element uses hardcoded color values

    Scenario: Theme renders in modern browsers
      Given the app is running
      When opened in Chrome, Firefox, or Safari (latest)
      Then backdrop-filter effects render correctly
      And Inter and JetBrains Mono fonts load from Google Fonts

  Rule: Layout shell

    Scenario: Sidebar collapse persists
      Given the sidebar is expanded (210px)
      When the user clicks the collapse toggle
      Then the sidebar collapses to 48px
      And the preference is stored in localStorage
      When the user refreshes the page
      Then the sidebar remains collapsed

    Scenario: Layout matches v4 mockup structure
      Given the app is running
      When viewed in a desktop browser
      Then the layout shows: sidebar (left) + main area (toolbar top, grid center, status bar bottom)
      And the branding header shows title and subtitle
      And the attribution footer shows "Powered by SeeKi"

  Rule: Component shells render

    Scenario: DataGrid mounts RevoGrid
      Given the DataGrid component receives columns and rows props
      When the component renders
      Then a RevoGrid instance appears with the provided columns and data

    Scenario: Toolbar shows placeholder controls
      Given the Toolbar component renders
      Then a disabled search input, Columns button, and Export button are visible

    Scenario: StatusBar shows row info
      Given the StatusBar receives total=427229, start=1, end=50
      When the component renders
      Then it displays "Showing 1 - 50 of 427,229"

  Rule: Mock API

    Scenario: Mock mode provides realistic data
      Given VITE_MOCK=true is set
      When the app fetches /api/tables
      Then 5 tables are returned (vehicles, vehicles_log, events, faults, flights)
      And each table has a realistic row_count_estimate

    Scenario: Fallback to mock on network error
      Given VITE_MOCK is not set and no backend is running
      When the app fetches /api/tables
      Then the fetch fails with a network error
      And the app falls back to mock data
      And a warning is logged to the console

  Rule: rust-embed integration

    Scenario: Release binary serves frontend
      Given the release binary is built with "just build"
      When I run the binary and open http://127.0.0.1:3141/ in a browser
      Then the frontend app loads without a separate Node.js server

    Scenario: SPA fallback serves index.html
      Given the release binary is running
      When I navigate to http://127.0.0.1:3141/table/vehicles
      Then index.html is served (not a 404)
      And the frontend app handles the route client-side

    Scenario: Static assets are cached aggressively
      Given the release binary is running
      When the browser requests /assets/index-abc123.js
      Then the response includes "Cache-Control: public, max-age=31536000, immutable"
      When the browser requests /index.html
      Then the response includes "Cache-Control: no-cache"
```

---

## Task Breakdown

| ID | Task | Priority | Dependencies | Status |
|:---|:-----|:---------|:-------------|:-------|
| T1 | **Scaffold Svelte 5 + Vite + TS project** — `npm create vite@latest` in `frontend/`, add `@revolist/svelte-datagrid`, `lucide-svelte`; configure `vite.config.ts` with dev proxy to :3141 | High | None | pending |
| T2 | **Glassmorphism theme tokens** — Create `frontend/src/theme/tokens.css` with all `sk-` prefixed CSS custom properties; import in `app.css`; add Google Fonts to `index.html` | High | T1 | pending |
| T3 | **TypeScript types** — Create `frontend/src/lib/types.ts` with `TableInfo`, `ColumnInfo`, `QueryResult`, and API response types matching backend contracts | High | T1 | pending |
| T4 | **Mock API data** — Create `frontend/src/lib/mock.ts` with 5 tables, realistic columns, and 50 rows of plausible data per table | High | T3 | pending |
| T5 | **API fetch wrapper** — Create `frontend/src/lib/api.ts` with typed fetch functions; `VITE_MOCK` env var support; auto-fallback to mock on network error | High | T3, T4 | pending |
| T6 | **Layout shell (App.svelte)** — Flexbox layout with sidebar + main area; global styles in `app.css`; `{#if isSetup}` conditional placeholder | High | T2 | pending |
| T7 | **Sidebar component** — Collapse/expand with localStorage persistence; branding header; attribution footer; `<slot>` for table list | High | T6 | pending |
| T8 | **Toolbar component** — Static placeholder with disabled controls; table name + row count props | High | T6 | pending |
| T9 | **DataGrid component** — Mount `<RevoGrid>` with basic column/row props; render mock data | High | T5, T6 | pending |
| T10 | **StatusBar component** — Row count display; disabled prev/next buttons | Med | T6 | pending |
| T11 | **rust-embed module** — Create `src/embed.rs` with `RustEmbed` derive, fallback handler, cache headers; wire into `main.rs` Router | High | T1 | pending |
| T12 | **Justfile** — `dev` (cargo run + vite), `dev-mock` (vite only with VITE_MOCK=true), `build` (npm build + cargo build --release) recipes | High | T1, T11 | pending |

---

## Exit Criteria

- [ ] `just dev` starts both Vite and cargo from a single command
- [ ] `just dev-mock` renders the layout shell with mock data, no backend required
- [ ] `just build` produces a single binary that serves the frontend at `/`
- [ ] SPA fallback works (refreshing a non-root path serves index.html)
- [ ] Sidebar collapse/expand works and persists across page reloads
- [ ] All UI elements use CSS custom properties from tokens.css (no hardcoded colors)
- [ ] RevoGrid renders a basic grid with mock columns and rows
- [ ] Layout matches the approved v4 mockup structure (sidebar, toolbar, grid card, status bar)
- [ ] Theme renders correctly in Chrome, Firefox, and Safari (latest versions)
- [ ] TypeScript compiles with no errors; `npm run build` produces dist/ without warnings

---

## References

- MVP spec: `seeki-mvp-spec.md` (tasks T7, T18, T19)
- Approved visual direction: `.brainstorm/326657-1775548709/content/visual-style-v4.html`
- GitHub issues: #2 (epic), #13 (scaffold), #14 (theme), #15 (rust-embed)
- Open PR: #28 (epic: Frontend Scaffold & Integration)
- RevoGrid Svelte wrapper: `@revolist/svelte-datagrid` v4.21+ ([GitHub](https://github.com/revolist/svelte-datagrid))

---

*Authored by: Clault KiperS 4.6*
