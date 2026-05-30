CREATE TABLE saved_views (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id      TEXT    NOT NULL,
    name               TEXT    NOT NULL,
    base_schema        TEXT    NOT NULL,
    base_table         TEXT    NOT NULL,
    definition_version INTEGER NOT NULL DEFAULT 1,
    columns            TEXT    NOT NULL,
    filters            TEXT    NOT NULL DEFAULT '{}',
    created_at         TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at         TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(connection_id, name)
);
