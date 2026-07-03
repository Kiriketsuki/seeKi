# Competitive Analysis: Read-Only Database Viewers

Research conducted: 2026-04-07

## Comparison Table

| Tool | Read-Only | UX Simplicity | Supported DBs | Deployment | License | Stars |
|:---|:---|:---|:---|:---|:---|:---|
| **Datasette** | Native (by design) | Good -- table browse + faceted search, but SQL box visible | SQLite only (ETL for others) | Python pip, Docker, WASM | Apache 2.0 | 10.9k |
| **NocoDB** | Configurable (viewer role) | Excellent -- true spreadsheet UI | MySQL, PG, SQLite, MariaDB, MSSQL | Docker, npm | Sustainable Use License | 60.7k |
| **Baserow** | Configurable (viewer role) | Excellent -- spreadsheet-native | Own internal DB only | Docker Compose (heavy) | MIT core + proprietary | 3.7k |
| **Directus** | Configurable (CRUDS roles) | Good -- modern admin panel, more CMS | PG, MySQL, SQLite, MSSQL, Oracle, MariaDB, CockroachDB | Docker, npm | BSL 1.1 | 29k+ |
| **Metabase** | Configurable (data permissions) | Moderate -- BI/dashboard focused | PG, MySQL, SQLite, MongoDB, BigQuery, many more | Docker, JAR | AGPL 3.0 | 41k+ |
| **Adminer** | No native read-only | Poor for non-tech -- DBA-oriented | MySQL, PG, SQLite, MSSQL, Oracle, MongoDB | Single PHP file | Apache 2.0 / GPL 2 | ~6k |
| **CloudBeaver** | Configurable (connection-level) | Poor for non-tech -- SQL IDE | PG, MySQL, SQLite, Oracle, MSSQL, many via JDBC | Docker | Apache 2.0 | ~4k |
| **Redash** | Configurable (viewer role) | Moderate -- query-first | 40+ data sources | Docker Compose (complex) | BSD 2-Clause | ~27k |
| **Evidence** | Read-only by design | Poor for browsing -- code-first | Any SQL DB | Node.js / Docker | Apache 2.0 | ~4.5k |
| **Outerbase Studio** | No native read-only | Good -- spreadsheet-style editor | PG, MySQL, SQLite, LibSQL | Browser, Electron, Docker | AGPL 3.0 | ~2k |

## Deep Dive: Datasette (Closest Competitor)

**Strengths:**
- Read-only by default -- the core design philosophy is "publish data for exploration"
- Rich plugin ecosystem (500+ plugins) -- faceted search, export formats, charting
- Datasette Lite runs entirely in WASM in the browser (zero deployment)
- One-command cloud deployment (`datasette publish cloudrun`)
- Built for non-technical users (journalists, researchers, archivists)
- Apache 2.0 -- true open source

**Critical Limitations:**
- SQLite only -- cannot connect to PostgreSQL or MySQL directly. Must ETL via `db-to-sqlite`
- Not truly spreadsheet-like -- tables with pagination and a SQL query box
- Pre-1.0 -- API still breaking between alpha releases
- Python dependency -- not a single binary

## Deep Dive: NocoDB (Strongest Spreadsheet UX)

**Strengths:**
- True spreadsheet interface on top of existing SQL databases
- Connects directly to MySQL, PostgreSQL, etc.
- Viewer role for read-only access
- 60k+ GitHub stars, very active community
- Multiple view types: grid, gallery, kanban, form, calendar

**Critical Limitations:**
- License changed to Sustainable Use License (not OSS for managed service)
- Heavy deployment -- requires its own metadata database
- Feature bloat -- full Airtable alternative (forms, automations, API builder)
- Sync lag -- metadata layer can get out of sync with external DB

## Gap Analysis

1. **No tool does "point at a DB connection string, get a spreadsheet view" simply.** NocoDB is closest but carries massive overhead. Datasette requires SQLite conversion.

2. **Read-only is always an afterthought.** Every tool (except Datasette and Evidence) is built for read-write and restricts via permissions. UI still shows write-oriented affordances.

3. **No single-binary solution exists** for web-based read-only DB browsing.

4. **Spreadsheet UX + database connectivity is an unsolved pairing.** Tools are either:
   - Spreadsheet-like but don't connect to external DBs (Baserow)
   - Connect to external DBs but aren't spreadsheet-like (Metabase, CloudBeaver, Adminer)
   - Both, but massively overbuilt (NocoDB)

5. **Missing features non-technical users want:**
   - Simple column filtering (like Excel AutoFilter)
   - Click-to-sort without writing SQL
   - Export selected rows to CSV/Excel
   - Saved views per user
   - Natural language search across all columns
   - No visible SQL anywhere

## Conclusion

The market gap is clear: a lightweight, read-only, spreadsheet-like database viewer that connects directly to PostgreSQL/MySQL/SQLite with zero configuration beyond a connection string. This is SeeKi's niche.
