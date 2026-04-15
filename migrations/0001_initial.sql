-- Singleton key/value settings (values are JSON-encoded).
-- Used for: theme, page_size, update_channel, and any future app-level prefs.
CREATE TABLE app_settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Last-used sort/filter state per (connection, schema, table) — auto-saved on change.
-- Restored automatically when the user switches back to the same table.
CREATE TABLE table_last_used_state (
    connection_id TEXT NOT NULL,
    schema_name   TEXT NOT NULL,
    table_name    TEXT NOT NULL,
    sort_columns  TEXT NOT NULL DEFAULT '[]',  -- JSON: [{"col":"x","dir":"asc"}, ...]
    filters       TEXT NOT NULL DEFAULT '{}',  -- JSON: {"col": "value"}
    search_term   TEXT,
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (connection_id, schema_name, table_name)
);

-- Named sort presets (user-saved, per connection+table).
CREATE TABLE table_sort_presets (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id TEXT    NOT NULL,
    schema_name   TEXT    NOT NULL,
    table_name    TEXT    NOT NULL,
    name          TEXT    NOT NULL,
    columns       TEXT    NOT NULL,  -- JSON: [{"col":"x","dir":"asc"}, ...]
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(connection_id, schema_name, table_name, name)
);

-- Named filter presets (user-saved, per connection+table).
CREATE TABLE table_filter_presets (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id TEXT    NOT NULL,
    schema_name   TEXT    NOT NULL,
    table_name    TEXT    NOT NULL,
    name          TEXT    NOT NULL,
    filters       TEXT    NOT NULL,  -- JSON: {"col": "value"}
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(connection_id, schema_name, table_name, name)
);

-- Generic key/value UI state per connection (schema only; frontend migration deferred).
-- Example keys: "sidebar_collapsed", "column_visibility.public.vehicles"
CREATE TABLE ui_state (
    connection_id TEXT NOT NULL,
    key           TEXT NOT NULL,
    value         TEXT NOT NULL,  -- JSON
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (connection_id, key)
);

-- Custom view stubs — schema only, populated by issue #77.
CREATE TABLE custom_views (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL UNIQUE,
    definition TEXT NOT NULL,  -- JSON
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
