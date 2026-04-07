# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is SeeKi?

A read-only, spreadsheet-like database viewer for casual (non-technical) users. Single Rust binary with embedded frontend assets. Currently supports PostgreSQL; SQLite planned for v0.2.

Key design principle: **no visible SQL anywhere** — users interact through a spreadsheet-like grid with click-to-sort, search, and filtering.

## Build & Run

```bash
# Build (debug)
cargo build

# Build (release — embeds frontend assets via rust-embed)
cargo build --release

# Run (requires seeki.toml in CWD or ~/.config/seeki/config.toml)
cargo run

# Check without building (faster iteration)
cargo check

# Run tests
cargo test

# Run a single test
cargo test <test_name>

# Clippy
cargo clippy
```

The server listens on `http://127.0.0.1:3141` by default (configurable in `seeki.toml`).

## Configuration

Config is loaded from `seeki.toml` (CWD) or `~/.config/seeki/config.toml`. See `seeki.toml.example` for the format. The config file is gitignored since it contains DB credentials.

## Architecture

```
src/
├── main.rs          # Entry point: config → DB pool → axum Router → serve
├── config.rs        # TOML config loading (AppConfig, DatabaseConfig, DatabaseKind enum)
├── api/mod.rs       # Axum routes under /api (tables, columns, rows)
├── db/mod.rs        # DatabasePool enum (dispatch layer) + shared types (TableInfo, ColumnInfo, QueryResult)
├── db/postgres.rs   # PostgreSQL-specific queries (schema introspection, paginated row fetching)
└── auth/mod.rs      # Placeholder — auth not yet implemented
```

**Data flow**: HTTP request → `api/` handler → `DatabasePool` dispatch → engine-specific module (e.g. `postgres.rs`) → JSON response.

**Adding a new database engine**: Add a `db/<engine>.rs` implementing the same functions as `postgres.rs` (`list_tables`, `get_columns`, `query_rows`), add a variant to `DatabasePool` and `DatabaseKind`, wire up the match arms.

**Frontend**: Will be Svelte 5 + RevoGrid in `frontend/`. Assets are embedded into the binary via `rust-embed` for single-binary deployment. Frontend directory is currently empty.

## SQL Injection Prevention

Table and column names are validated (alphanumeric + underscore only) before string interpolation into SQL. Actual values use parameterized queries via sqlx. Maintain this pattern — never interpolate user-supplied values into SQL strings.

## GitHub Automation

The `issue-branch-handler.yml` workflow auto-creates branches and draft PRs when issues are labeled (epic/feature/task/bug/hotfix). Branch naming follows `<prefix>/<issue-number>-<sanitized-title>`. Child issues branch from their parent issue's branch when one exists.

## Rust Edition

Uses Rust edition **2024** (`Cargo.toml`). This requires rustc 1.85+.
