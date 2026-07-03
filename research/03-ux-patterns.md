# UX Patterns: Spreadsheet-Like Database Viewer

Research conducted: 2026-04-07

## 1. Why phpMyAdmin/Adminer Are Intimidating (Anti-Patterns to Avoid)

| Anti-Pattern | Why It Fails | What To Do Instead |
|:---|:---|:---|
| SQL-first interface | Users don't know SQL | Data-first: show the grid immediately |
| Technical jargon | `VARCHAR(255)`, `INT(11)` | Human-readable: "Text", "Number", "Date" |
| Information overload | Structure, indexes, triggers all at once | Progressive disclosure -- data first, metadata on demand |
| Developer-centric nav | Raw table names with no context | Friendly table list with row counts and search |
| Dense toolbar | Browse, Structure, SQL, Search, Insert, Export... | Minimal: search, filter, sort, export |
| No visual type rendering | Everything is plain text | Type-aware cells (checkboxes, formatted dates, JSON) |
| Outdated design | Heavy borders, dense padding | Modern, clean, generous whitespace |

## 2. Intuitive Patterns from Airtable/Sheets

### Filtering

**Recommendation: Filter bar above table + per-column filter icons on headers**

- Filter bar (Airtable model): Visual condition builder `[Column] [Operator] [Value]`
- Active filters show as chips with X to remove, plus "Clear all"
- Per-column quick filters via funnel icon on headers
- Visual indicator when filters are active (badge count on filter button)
- Data-type-aware operators:
  - Text: contains, starts with, is exactly
  - Numbers: greater than, less than, between
  - Dates: before, after, in the last N days
  - Booleans: is true, is false

### Sorting

**Recommendation: Click column header to cycle sort**

- Single click: ascending. Click again: descending. Third click: remove sort
- Visual indicator: subtle chevron arrow in sorted column header
- Multi-column sort via sort panel for advanced users

### Search

**Recommendation: Global search bar + optional per-column search**

- Global search prominently placed top-right, debounced input (300ms)
- Highlights matching text within cells
- Server-side for >1000 rows, client-side for smaller tables
- Show result count: "Showing 23 of 1,456 rows matching 'sensor'"
- Per-column search hidden behind column header menu (power feature)

### Pagination vs Virtual Scrolling

**Recommendation: Virtual scrolling with pagination fallback**

- Virtual scrolling (render only visible rows) for default experience
- Pagination controls at bottom as secondary navigation
- 25 rows per page is optimal for most users
- Avoid infinite scroll -- causes disorientation
- Client-side for <1000 rows, server-side cursor pagination for larger datasets

### Column Interactions

| Feature | Implementation |
|:---|:---|
| Resize | Drag handle on column border (right edge of header) |
| Reorder | Drag-and-drop column headers |
| Hide/Show | Column visibility toggle in "Columns" dropdown |
| Freeze | First column (primary identifier) always sticky |
| Reset | "Reset to default" button |

Save column preferences to localStorage for persistence.

### Data Type Presentation

| Data Type | Rendering |
|:---|:---|
| Text (short) | Plain text, left-aligned |
| Text (long) | Truncated with ellipsis, full on hover/click |
| Numbers | Right-aligned, monospace, locale formatting (1,234.56) |
| Booleans | Read-only checkbox, NOT "true"/"false" text |
| Dates | Human-readable ("Apr 7, 2026"), tooltip shows full timestamp |
| JSON | Collapsed "{...}" badge, click to expand with syntax highlighting |
| URLs | Clickable links with external link icon |
| Email | Clickable mailto: link |
| NULL | Subtle gray italic "null" -- distinct from empty string |
| UUIDs/IDs | Monospace, truncated with copy button |

## 3. Multi-Table Navigation

**Recommendation: Tabs for <10 tables, searchable sidebar for 10+**

- Horizontal tab bar (like Google Sheets sheet tabs) for small table counts
- Collapsible left sidebar with search for larger databases
- Each entry shows: table name + row count badge
- Active table indicator with clear visual highlight
- Table name prominently displayed above the grid

## 4. Progressive Disclosure

| Layer | Content | Interaction |
|:---|:---|:---|
| 0 (always) | Data grid with column headers | Immediate on load |
| 1 (always) | Table name, row count in tabs | Visible in navigation |
| 2 (hover) | Column type tooltip ("Text", "Integer"), nullable, PK indicator | Hover on header |
| 3 (click) | Full column list, types, constraints, FK relationships | "Table info" button |
| 4 (power user) | Raw SQL schema, export schema as CSV/JSON | Explicit toggle |

**Key principle**: The data grid is layer 0 -- it should load immediately with no metadata overlay.

## 5. Mobile Responsiveness

**Recommendation: Card view on mobile, grid on desktop, with toggle**

- Breakpoint at ~768px: switch from grid to card layout
- Card layout: each row becomes a card, primary column is card title
- Frozen first column on tablets (where grid still works)
- Touch targets: minimum 44x44px for all interactive elements
- Simplified toolbar: collapse filter/sort/search into bottom sheet on mobile
- View toggle: let users switch between Grid and Card on any screen size

## 6. Implementation Priority (MVP to Full)

1. Type-aware cell rendering -- biggest UX win for de-intimidating DB data
2. Clean grid with sticky headers -- spreadsheet feel, minimal chrome
3. Global search + filter bar -- Airtable-style visual filter builder with chips
4. Column header sort -- click to sort, visual indicator
5. Table tabs/sidebar -- multi-table navigation that scales
6. Virtual scrolling with pagination -- smooth scroll + position reference
7. Progressive schema disclosure -- tooltips on headers, slide-out for details
8. Mobile card view -- responsive breakpoint
9. Column management -- resize, reorder, hide, freeze, persist
10. Row detail panel -- click a row to see all fields in side panel

## Sources

- Pencil & Paper: Enterprise Data Table UX Patterns
- Data Table UX: 5 Rules of Thumb (mannhowie.com)
- UX Patterns for Developers: Data Table (uxpatterns.dev)
- Eleken: 19+ Filter UI Examples for SaaS
- NN/g: Mobile Tables, Progressive Disclosure
- UX Movement: Best Mobile Layout for Complex Data Tables
- AG Grid: Cell Data Types documentation
- Smart Interface Design Patterns: Complex Filtering
