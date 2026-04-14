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

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **seeKi** (650 symbols, 1239 relationships, 49 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## When Debugging

1. `gitnexus_query({query: "<error or symptom>"})` — find execution flows related to the issue
2. `gitnexus_context({name: "<suspect function>"})` — see all callers, callees, and process participation
3. `READ gitnexus://repo/seeKi/process/{processName}` — trace the full execution flow step by step
4. For regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})` — see what your branch changed

## When Refactoring

- **Renaming**: MUST use `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` first. Review the preview — graph edits are safe, text_search edits need manual review. Then run with `dry_run: false`.
- **Extracting/Splitting**: MUST run `gitnexus_context({name: "target"})` to see all incoming/outgoing refs, then `gitnexus_impact({target: "target", direction: "upstream"})` to find all external callers before moving code.
- After any refactor: run `gitnexus_detect_changes({scope: "all"})` to verify only expected files changed.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Tools Quick Reference

| Tool | When to use | Command |
|------|-------------|---------|
| `query` | Find code by concept | `gitnexus_query({query: "auth validation"})` |
| `context` | 360-degree view of one symbol | `gitnexus_context({name: "validateUser"})` |
| `impact` | Blast radius before editing | `gitnexus_impact({target: "X", direction: "upstream"})` |
| `detect_changes` | Pre-commit scope check | `gitnexus_detect_changes({scope: "staged"})` |
| `rename` | Safe multi-file rename | `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` |
| `cypher` | Custom graph queries | `gitnexus_cypher({query: "MATCH ..."})` |

## Impact Risk Levels

| Depth | Meaning | Action |
|-------|---------|--------|
| d=1 | WILL BREAK — direct callers/importers | MUST update these |
| d=2 | LIKELY AFFECTED — indirect deps | Should test |
| d=3 | MAY NEED TESTING — transitive | Test if critical path |

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/seeKi/context` | Codebase overview, check index freshness |
| `gitnexus://repo/seeKi/clusters` | All functional areas |
| `gitnexus://repo/seeKi/processes` | All execution flows |
| `gitnexus://repo/seeKi/process/{name}` | Step-by-step execution trace |

## Self-Check Before Finishing

Before completing any code modification task, verify:
1. `gitnexus_impact` was run for all modified symbols
2. No HIGH/CRITICAL risk warnings were ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated

## Keeping the Index Fresh

After committing code changes, the GitNexus index becomes stale. Re-run analyze to update it:

```bash
npx gitnexus analyze
```

If the index previously included embeddings, preserve them by adding `--embeddings`:

```bash
npx gitnexus analyze --embeddings
```

To check whether embeddings exist, inspect `.gitnexus/meta.json` — the `stats.embeddings` field shows the count (0 means no embeddings). **Running analyze without `--embeddings` will delete any previously generated embeddings.**

> Claude Code users: A PostToolUse hook handles this automatically after `git commit` and `git merge`.

## CLI

- Re-index: `npx gitnexus analyze`
- Check freshness: `npx gitnexus status`
- Generate docs: `npx gitnexus wiki`

<!-- gitnexus:end -->
