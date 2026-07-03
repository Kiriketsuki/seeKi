# Tech Stack Research

Research conducted: 2026-04-07

## Chosen Stack

| Layer | Choice | Rationale |
|:---|:---|:---|
| Backend | Rust (axum + tokio) | Single binary, type safety, sqlx multi-DB |
| Frontend | Svelte 5 | Smallest bundle (~5-10KB), reactive, compiles away |
| Data Grid | RevoGrid (MIT) | Web component, virtual scroll for 100K+ rows, 50KB |
| PG driver | sqlx (postgres) | Compile-time checked queries, async |
| SQLite driver | sqlx (sqlite) | Same interface, zero config |
| Config | TOML | Simple, human-readable |
| Auth | Argon2 + session cookies | Basic login for internal use |
| Deploy | Single binary (rust-embed) + Docker | ~20MB binary, FROM scratch image |

## Backend Language Evaluation

| Language | Deployment | Multi-DB Support | Build Complexity |
|:---|:---|:---|:---|
| **Rust** | Single binary (cross-compile) | sqlx with multiple backends | Medium (learning curve, compile times) |
| Go | Single binary (cross-compile) | `database/sql` + pgx, modernc.org/sqlite | Low |
| Node.js | Needs runtime + node_modules | knex.js or drizzle | Low dev, high deploy |
| Python | Needs runtime + pip/venv | SQLAlchemy | Low dev, medium deploy |

### Why Rust over Go

- Stronger type safety catches more bugs at compile time
- sqlx provides compile-time query checking
- axum + tokio is a well-established async web stack
- Same single-binary deployment story as Go
- Trade-off: slower compile times, steeper learning curve

### Prior Art

- **pgweb** (Go, 8.6k stars) -- single-binary PostgreSQL viewer, validates the single-binary architecture
- **Datasette** (Python, 10.9k stars) -- read-only viewer philosophy, validates the market

## Frontend Framework Evaluation

| Framework | Bundle Size | Grid Ecosystem | Data-Heavy DX |
|:---|:---|:---|:---|
| React | ~45KB gzip | Best (AG Grid, TanStack, Glide) | Good but verbose |
| Vue 3 | ~33KB gzip | Good (AG Grid, RevoGrid) | Great, Composition API |
| **Svelte 5** | ~5-10KB gzip | Growing (SVAR, RevoGrid, TanStack) | Excellent reactivity |
| HTMX | ~14KB | No grid libraries | Unusable for 100k+ row virtual scroll |

### Why Svelte 5

For a tool that is 90% data grid, the framework should get out of the way. Svelte compiles away, leaving minimal runtime overhead. RevoGrid (web component) works natively with Svelte.

Fallback: React + AG Grid Community if Svelte integration proves insufficient.

## Data Grid Library Evaluation

| Library | License | Bundle Size | Virtual Scroll | Spreadsheet Feel |
|:---|:---|:---|:---|:---|
| **RevoGrid** | MIT (Pro available) | ~50KB | Yes (millions) | Excellent |
| AG Grid Community | MIT | ~200KB | Yes (100k+) | Excellent |
| TanStack Table | MIT | ~15KB (headless) | DIY | You build the UI |
| Handsontable | **Non-commercial only** | ~300KB | Yes | Best -- but disqualified |
| Glide Data Grid | MIT | ~40KB | Yes | Good -- React only |
| SVAR DataGrid | MIT | ~moderate | Yes | Good -- Svelte native |

### Why RevoGrid

- MIT licensed, web component (framework-agnostic)
- Virtual scrolling handles millions of cells
- Built-in copy/paste from Excel/Google Sheets
- Column resizing, drag-and-drop, cell templates
- ~50KB bundle -- 4x lighter than AG Grid
- WAI-ARIA accessibility built in

## Multi-Database Connection Pattern

```
sqlx (unified interface)
    |
    +-- postgres feature (PostgreSQL)
    +-- sqlite feature (SQLite)
    +-- mysql feature (MySQL, future)
```

Schema introspection adapter per engine (~200 LOC each):

| Operation | PostgreSQL | SQLite |
|:---|:---|:---|
| List tables | `pg_class` + `pg_namespace` | `sqlite_master` |
| List columns | `information_schema.columns` | `PRAGMA table_info(x)` |
| Row count | `pg_class.reltuples` (estimate) | `SELECT COUNT(*)` |

No ORM needed. Raw `sqlx` with parameterized queries is simpler for read-only use.

## Deployment Comparison

| Tool | Deployment | Weight |
|:---|:---|:---|
| **SeeKi (target)** | Single Rust binary + Docker FROM scratch | ~20MB |
| pgweb | Single Go binary | ~15MB |
| Datasette | pip install | Python + deps |
| NocoDB | Docker (Node.js) | ~500MB |
| Baserow | Docker Compose | ~1GB+ |

## Risk Mitigation

| Risk | Mitigation |
|:---|:---|
| RevoGrid missing a feature | Fall back to AG Grid Community (MIT) |
| Svelte ecosystem too small | RevoGrid is a web component -- framework-agnostic |
| sqlx compile times | Use `cargo check` during dev, only full build for release |
| Schema introspection edge cases | Add engines incrementally; PostgreSQL first |
