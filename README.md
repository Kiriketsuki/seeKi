<p align="left">
  <img src="frontend/public/logo-wordmark.svg" alt="SeeKi" height="48" />
</p>

# SeeKi

![version-26.5.0.2-blue](https://img.shields.io/badge/version-26.5.0.1-blue)
[![CI](https://github.com/Kiriketsuki/seeKi/actions/workflows/ci.yml/badge.svg)](https://github.com/Kiriketsuki/seeKi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
[![Docs](https://img.shields.io/badge/docs-GitHub_Pages-blue?logo=github)](https://kiriketsuki.github.io/seeKi/)

**A read-only, spreadsheet-like database viewer for people who don't speak SQL.**

SeeKi connects to your PostgreSQL database and presents it as a clean, searchable, filterable grid -- no SQL knowledge required. Point it at a connection string, pick the tables you want to expose, and share the URL with your team.

Built as a single Rust binary with an embedded frontend. No runtime dependencies, no Docker required, no Node.js in production.

<!-- TODO: Add screenshot once frontend is implemented -->

## Why SeeKi?

Every team has people who need to look at database data but shouldn't need to learn SQL or wrestle with DBeaver. Analysts generating reports. Engineers checking vehicle telemetry. Ops staff looking up customer records.

Existing tools are either too complex (DBeaver, Adminer), too heavy (NocoDB, Metabase), or don't connect to your database (Datasette requires SQLite conversion). SeeKi fills the gap: **lightweight, read-only, spreadsheet-like, zero-config**.

| | SeeKi | DBeaver | NocoDB | Datasette |
|:--|:------|:--------|:-------|:----------|
| Read-only by design | Yes | No | Configurable | Yes |
| Connects to PostgreSQL | Yes | Yes | Yes | No (SQLite only) |
| Non-technical friendly | Yes | No | Yes | Moderate |
| Single binary | Yes | No | No | No |
| Setup time | < 2 min | 10+ min | 30+ min | 5+ min |

## Features

- **Spreadsheet-like grid** -- virtual scrolling handles 500K+ rows smoothly
- **No visible SQL** -- search, filter, and sort through the UI, not a query editor
- **Collapsible sidebar** -- browse tables with search and row counts at a glance
- **Click-to-sort** -- click any column header to sort ascending, descending, or reset
- **Global search** -- search across all text columns in the current table
- **Per-column filters** -- filter individual columns directly from the header
- **Column visibility** -- hide columns you don't need; preferences persist across sessions
- **CSV export** -- export the current filtered/sorted view with friendly column names
- **Friendly formatting** -- timestamps become "Apr 7, 3:58 PM", booleans become checkboxes, NULLs are visually distinct
- **Smart column names** -- `supervisor_id` becomes "Supervisor" automatically; override in config
- **Setup wizard** -- first-run browser wizard: paste connection string, pick tables, set branding
- **Custom branding** -- your app title, your subtitle, your tool (with "Powered by SeeKi" attribution)

## Quick Start

### From Binary

```bash
# Download the latest release for your platform
# https://github.com/kiriketsuki/seeKi/releases

# Run it -- the setup wizard opens in your browser
./seeki

# Or, if you already have a config file:
./seeki --config path/to/seeki.toml
```

### From Source

```bash
# Prerequisites: Rust 1.85+ (edition 2024), Node.js 20+

# Clone
git clone https://github.com/kiriketsuki/seeKi.git
cd seeKi

# Build frontend
cd frontend && npm install && npm run build && cd ..

# Build and run
cargo build --release
./target/release/seeki
```

Open `http://127.0.0.1:3141` in your browser. If no config file exists, the setup wizard will guide you through connecting to your database.

## Configuration

SeeKi looks for config in this order:
1. `seeki.toml` in the current directory
2. `~/.config/seeki/config.toml`

### Example `seeki.toml`

```toml
[server]
host = "127.0.0.1"
port = 3141

[database]
kind = "postgres"
url = "postgres://user:password@localhost:5432/mydb"
max_connections = 5
# Which schemas to browse. Omit to default to ["public"].
# System schemas (pg_catalog, information_schema, pg_*) are always excluded.
schemas = ["public", "reporting"]

[branding]
title = "AutoConnect"
subtitle = "Fleet Telemetry Database"

[tables]
# Include specific tables (if omitted, all tables are exposed).
# Bare names match any selected schema; use "schema.table" to target one schema.
include = ["vehicles", "vehicles_log", "reporting.orders"]
# Or exclude specific tables:
# exclude = ["internal_logs", "migrations", "audit.sessions"]

[display.columns]
# Override auto-generated column display names
# Format: "table_name.column_name" = "Display Name"
"vehicles_log.supervisor_id" = "Supervisor"
"vehicles_log.posn_lat" = "Latitude"
"vehicles_log.posn_lon" = "Longitude"
"vehicles_log.vehicle_id" = "Vehicle"
"vehicles_log.entry_id" = "#"
```

### Column Name Auto-Heuristic

Without any configuration, SeeKi automatically converts column names to friendly display names:

| Raw Column | Auto Display Name | Rule |
|:-----------|:-----------------|:-----|
| `vehicle_id` | Vehicle | Drop `_id` suffix, Title Case |
| `supervisor_id` | Supervisor | Drop `_id` suffix, Title Case |
| `created_at` | Created At | Title Case |
| `is_active` | Is Active | Title Case |
| `posn_lat` | Posn Lat | Title Case (override recommended) |

Use `[display.columns]` overrides for names the heuristic gets wrong.

### Multiple Schemas

PostgreSQL databases often organise tables across schemas (`public`, `reporting`, `audit`, …). SeeKi auto-discovers every non-system schema the DB user can access; during the setup wizard, tick the schemas you want exposed. The selection is saved to `seeki.toml` as `database.schemas`.

- Display: tables in `public` are shown unqualified (`orders`); tables in other schemas are prefixed (`reporting.orders`). When the same bare name exists in multiple selected schemas, both are always shown qualified.
- `tables.include` / `tables.exclude` accept either bare names (match any selected schema) or qualified `schema.table` pairs.
- An empty `schemas = []` is rejected at startup.

## Architecture

```
src/
├── main.rs          # Entry: config -> DB pool -> axum router -> serve
├── config.rs        # TOML config (AppConfig, DatabaseConfig, tables, display, branding)
├── api/mod.rs       # REST endpoints: tables, columns, rows, export, setup
├── db/mod.rs        # DatabasePool enum (dispatch layer) + shared types
├── db/postgres.rs   # PostgreSQL: schema introspection, paginated queries, CSV streaming
└── auth/mod.rs      # Placeholder (post-MVP)

frontend/            # Svelte 5 + Vite (embedded via rust-embed in release builds)
├── src/
│   ├── App.svelte          # Root layout: sidebar + main content
│   ├── lib/
│   │   ├── Sidebar.svelte  # Table list, search, branding
│   │   ├── Grid.svelte     # RevoGrid wrapper with virtual scrolling
│   │   ├── Toolbar.svelte  # Search, columns dropdown, export button
│   │   ├── StatusBar.svelte # Row count, pagination
│   │   └── SetupWizard.svelte # First-run configuration flow
│   └── stores/             # Svelte stores for table state, filters, config
└── vite.config.ts          # Dev proxy to axum on :3141
```

**Data flow**: Browser -> Svelte frontend -> REST API -> `DatabasePool` dispatch -> PostgreSQL -> JSON response -> RevoGrid render.

**Single binary**: In release builds, `cargo build --release` embeds all frontend assets (HTML, JS, CSS) into the binary via `rust-embed`. The binary serves both the API and the frontend. No Node.js, no separate static file server.

**Development**: During development, run Vite's dev server (`npm run dev` in `frontend/`) which proxies API calls to axum running on `:3141`. Hot module replacement works normally.

## Development

```bash
# Prerequisites
rustup install stable    # Rust 1.85+
node --version           # Node.js 20+

# Backend (runs on :3141)
cargo run

# Frontend dev server (runs on :5173, proxies API to :3141)
cd frontend && npm run dev

# Check without building (faster iteration)
cargo check

# Run tests
cargo test

# Lint
cargo clippy
```

## Roadmap

### v0.1 -- MVP (current)

- [x] PostgreSQL connection and schema introspection
- [x] REST API: tables, columns, paginated rows
- [x] SQL injection prevention (parameterized queries + identifier validation)
- [ ] Svelte 5 + RevoGrid frontend with glassmorphism theme
- [ ] Collapsible sidebar with table search and row counts
- [ ] Click-to-sort, global search, per-column text filters
- [ ] Column hide/show with localStorage persistence
- [ ] CSV export with display name headers
- [ ] Auto-friendly display formatting (timestamps, booleans, NULLs)
- [ ] Column name casualification (auto-heuristic + config overrides)
- [ ] First-run setup wizard
- [ ] Table include/exclude configuration
- [ ] App branding customization
- [ ] rust-embed single binary build

### v0.2 -- Enhanced Filtering + SQLite

- [ ] Airtable-style filter bar: `[Column] [Operator] [Value]` chips
- [ ] Type-aware filter operators (text: contains/equals, numbers: >/</between, dates: before/after)
- [ ] SQLite database support
- [ ] Column resize and reorder (drag handles, persist to localStorage)

### v0.3 -- Auth + Polish

- [ ] Username/password authentication (Argon2 + session cookies)
- [ ] Dark mode with theme toggle
- [ ] Row detail side panel (click row to see all fields)
- [ ] JSON cell expand with syntax highlighting

### v0.4 -- Power Features

- [ ] Saved views (per-user column/filter/sort configurations)
- [ ] Multi-column sort panel
- [ ] Mobile card view (responsive breakpoint at 768px)
- [ ] Export filtered/selected rows
- [ ] MySQL support

## Security

SeeKi is designed for internal/trusted networks. It is **read-only by design** -- no write operations are possible through the application.

- Table and column names are validated (alphanumeric + underscore only) before any SQL interpolation
- All user-supplied values use parameterized queries via sqlx
- No SQL is ever visible to or writable by the end user
- Authentication is planned for v0.3; until then, restrict access at the network level

## Contributing

Contributions are welcome. Please open an issue to discuss before submitting large changes.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit with conventional commits (`feat:`, `fix:`, `docs:`, etc.)
4. Open a pull request

## License

MIT License. See [LICENSE](LICENSE) for details.

## Attribution

SeeKi is built with:
- [Rust](https://www.rust-lang.org/) + [axum](https://github.com/tokio-rs/axum) + [sqlx](https://github.com/launchbadge/sqlx)
- [Svelte 5](https://svelte.dev/) + [Vite](https://vitejs.dev/)
- [RevoGrid](https://revolist.github.io/revogrid/) for virtual scrolling data grids
- [Lucide](https://lucide.dev/) for SVG icons

---

*data-seeKing.*
