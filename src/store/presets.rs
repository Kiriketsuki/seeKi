use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;

const MAX_VALUE_BYTES: usize = 64 * 1024; // 64 KiB

/// Reject a serialized JSON value that exceeds the storage limit.
fn check_value_size(json: &str, context: &str) -> Result<()> {
    if json.len() > MAX_VALUE_BYTES {
        anyhow::bail!("preset value for '{context}' exceeds maximum size of 64 KiB");
    }
    Ok(())
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SortPreset {
    pub id: i64,
    pub name: String,
    pub columns: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterPreset {
    pub id: i64,
    pub name: String,
    pub filters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LastUsedState {
    pub sort_columns: Value,
    pub filters: Value,
    pub search_term: Option<String>,
}

// ── Last-used state ───────────────────────────────────────────────────────────

pub async fn get_last_used(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
) -> Result<Option<LastUsedState>> {
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT sort_columns, filters, search_term
         FROM table_last_used_state
         WHERE connection_id = ? AND schema_name = ? AND table_name = ?",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(None),
        Some((sort_json, filter_json, search)) => Ok(Some(LastUsedState {
            sort_columns: serde_json::from_str(&sort_json)?,
            filters: serde_json::from_str(&filter_json)?,
            search_term: search,
        })),
    }
}

pub async fn set_last_used(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
    state: &LastUsedState,
) -> Result<()> {
    let sort_json = serde_json::to_string(&state.sort_columns)?;
    check_value_size(&sort_json, "sort_columns")?;
    let filter_json = serde_json::to_string(&state.filters)?;
    check_value_size(&filter_json, "filters")?;
    if let Some(ref s) = state.search_term
        && s.len() > MAX_VALUE_BYTES
    {
        anyhow::bail!("preset value for 'search_term' exceeds maximum size of 64 KiB");
    }
    sqlx::query(
        "INSERT INTO table_last_used_state
             (connection_id, schema_name, table_name, sort_columns, filters, search_term)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(connection_id, schema_name, table_name) DO UPDATE SET
             sort_columns = excluded.sort_columns,
             filters      = excluded.filters,
             search_term  = excluded.search_term,
             updated_at   = datetime('now')",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .bind(&sort_json)
    .bind(&filter_json)
    .bind(&state.search_term)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete all browsing state (last-used, sort presets, filter presets) for one connection.
pub async fn clear_all(pool: &SqlitePool, conn_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM table_last_used_state WHERE connection_id = ?")
        .bind(conn_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM table_sort_presets WHERE connection_id = ?")
        .bind(conn_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM table_filter_presets WHERE connection_id = ?")
        .bind(conn_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Sort presets ──────────────────────────────────────────────────────────────

pub async fn list_sort_presets(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
) -> Result<Vec<SortPreset>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, name, columns FROM table_sort_presets
         WHERE connection_id = ? AND schema_name = ? AND table_name = ?
         ORDER BY created_at",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|(id, name, cols)| {
            serde_json::from_str(&cols).map(|columns| SortPreset { id, name, columns })
        })
        .collect::<Result<_, _>>()
        .map_err(anyhow::Error::from)
}

/// Save (or replace) a named sort preset. Returns the row id.
pub async fn save_sort_preset(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
    name: &str,
    columns: &Value,
) -> Result<i64> {
    let json = serde_json::to_string(columns)?;
    check_value_size(&json, "sort preset columns")?;
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO table_sort_presets
             (connection_id, schema_name, table_name, name, columns)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(connection_id, schema_name, table_name, name)
         DO UPDATE SET columns = excluded.columns
         RETURNING id",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .bind(name)
    .bind(&json)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Delete a named sort preset. Returns `true` if a row was deleted.
pub async fn delete_sort_preset(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
    name: &str,
) -> Result<bool> {
    let res = sqlx::query(
        "DELETE FROM table_sort_presets
         WHERE connection_id = ? AND schema_name = ? AND table_name = ? AND name = ?",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

// ── Filter presets ────────────────────────────────────────────────────────────

pub async fn list_filter_presets(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
) -> Result<Vec<FilterPreset>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, name, filters FROM table_filter_presets
         WHERE connection_id = ? AND schema_name = ? AND table_name = ?
         ORDER BY created_at",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|(id, name, f)| {
            serde_json::from_str(&f).map(|filters| FilterPreset { id, name, filters })
        })
        .collect::<Result<_, _>>()
        .map_err(anyhow::Error::from)
}

/// Save (or replace) a named filter preset. Returns the row id.
pub async fn save_filter_preset(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
    name: &str,
    filters: &Value,
) -> Result<i64> {
    let json = serde_json::to_string(filters)?;
    check_value_size(&json, "filter preset filters")?;
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO table_filter_presets
             (connection_id, schema_name, table_name, name, filters)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(connection_id, schema_name, table_name, name)
         DO UPDATE SET filters = excluded.filters
         RETURNING id",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .bind(name)
    .bind(&json)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Delete a named filter preset. Returns `true` if a row was deleted.
pub async fn delete_filter_preset(
    pool: &SqlitePool,
    conn_id: &str,
    schema: &str,
    table: &str,
    name: &str,
) -> Result<bool> {
    let res = sqlx::query(
        "DELETE FROM table_filter_presets
         WHERE connection_id = ? AND schema_name = ? AND table_name = ? AND name = ?",
    )
    .bind(conn_id)
    .bind(schema)
    .bind(table)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::testutil::ephemeral_store;

    const CONN: &str = "host:5432/mydb";
    const SCHEMA: &str = "public";
    const TABLE: &str = "vehicles";

    // ── Last-used ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn last_used_missing_returns_none() {
        let (store, _dir) = ephemeral_store().await;
        let result = get_last_used(store.pool(), CONN, SCHEMA, TABLE)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn last_used_round_trip() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let state = LastUsedState {
            sort_columns: serde_json::json!([{"col": "id", "dir": "asc"}]),
            filters: serde_json::json!({"status": "active"}),
            search_term: Some("foo".into()),
        };
        set_last_used(pool, CONN, SCHEMA, TABLE, &state)
            .await
            .unwrap();

        let got = get_last_used(pool, CONN, SCHEMA, TABLE)
            .await
            .unwrap()
            .expect("should exist");
        assert_eq!(got.sort_columns, state.sort_columns);
        assert_eq!(got.filters, state.filters);
        assert_eq!(got.search_term, state.search_term);
    }

    #[tokio::test]
    async fn last_used_upsert_replaces() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let s1 = LastUsedState {
            sort_columns: serde_json::json!([]),
            filters: serde_json::json!({}),
            search_term: None,
        };
        let s2 = LastUsedState {
            sort_columns: serde_json::json!([{"col": "name", "dir": "desc"}]),
            filters: serde_json::json!({}),
            search_term: Some("bar".into()),
        };
        set_last_used(pool, CONN, SCHEMA, TABLE, &s1).await.unwrap();
        set_last_used(pool, CONN, SCHEMA, TABLE, &s2).await.unwrap();

        let got = get_last_used(pool, CONN, SCHEMA, TABLE)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.sort_columns, s2.sort_columns);
        assert_eq!(got.search_term, s2.search_term);
    }

    #[tokio::test]
    async fn last_used_per_connection_isolation() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let state = LastUsedState {
            sort_columns: serde_json::json!([]),
            filters: serde_json::json!({}),
            search_term: None,
        };
        set_last_used(pool, "conn_a", SCHEMA, TABLE, &state)
            .await
            .unwrap();

        let result = get_last_used(pool, "conn_b", SCHEMA, TABLE).await.unwrap();
        assert!(result.is_none());
    }

    // ── Sort presets ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn sort_preset_round_trip() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let cols = serde_json::json!([{"col": "id", "dir": "asc"}]);
        let id = save_sort_preset(pool, CONN, SCHEMA, TABLE, "By ID", &cols)
            .await
            .unwrap();
        assert!(id > 0);

        let presets = list_sort_presets(pool, CONN, SCHEMA, TABLE).await.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "By ID");
        assert_eq!(presets[0].columns, cols);
    }

    #[tokio::test]
    async fn sort_preset_conflict_updates() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let cols1 = serde_json::json!([{"col": "id", "dir": "asc"}]);
        let cols2 = serde_json::json!([{"col": "name", "dir": "desc"}]);
        save_sort_preset(pool, CONN, SCHEMA, TABLE, "Mine", &cols1)
            .await
            .unwrap();
        save_sort_preset(pool, CONN, SCHEMA, TABLE, "Mine", &cols2)
            .await
            .unwrap();

        let presets = list_sort_presets(pool, CONN, SCHEMA, TABLE).await.unwrap();
        assert_eq!(presets.len(), 1, "upsert should not duplicate");
        assert_eq!(presets[0].columns, cols2);
    }

    #[tokio::test]
    async fn sort_preset_delete() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let cols = serde_json::json!([]);
        save_sort_preset(pool, CONN, SCHEMA, TABLE, "To Delete", &cols)
            .await
            .unwrap();

        let deleted = delete_sort_preset(pool, CONN, SCHEMA, TABLE, "To Delete")
            .await
            .unwrap();
        assert!(deleted);

        let presets = list_sort_presets(pool, CONN, SCHEMA, TABLE).await.unwrap();
        assert!(presets.is_empty());
    }

    #[tokio::test]
    async fn sort_preset_delete_nonexistent_returns_false() {
        let (store, _dir) = ephemeral_store().await;
        let deleted = delete_sort_preset(store.pool(), CONN, SCHEMA, TABLE, "ghost")
            .await
            .unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn sort_preset_per_connection_isolation() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let cols = serde_json::json!([]);
        save_sort_preset(pool, "conn_a", SCHEMA, TABLE, "Mine", &cols)
            .await
            .unwrap();

        let presets = list_sort_presets(pool, "conn_b", SCHEMA, TABLE)
            .await
            .unwrap();
        assert!(presets.is_empty());
    }

    // ── Size guards ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn set_last_used_rejects_oversized_sort_columns() {
        let (store, _dir) = ephemeral_store().await;
        // Build a JSON value whose serialized form exceeds 64 KiB
        let big = serde_json::Value::String("x".repeat(65 * 1024));
        let state = LastUsedState {
            sort_columns: big,
            filters: serde_json::json!({}),
            search_term: None,
        };
        let result = set_last_used(store.pool(), CONN, SCHEMA, TABLE, &state).await;
        assert!(result.is_err(), "oversized sort_columns must be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("sort_columns"), "error should name the field");
    }

    #[tokio::test]
    async fn set_last_used_rejects_oversized_filters() {
        let (store, _dir) = ephemeral_store().await;
        let big = serde_json::Value::String("x".repeat(65 * 1024));
        let state = LastUsedState {
            sort_columns: serde_json::json!([]),
            filters: big,
            search_term: None,
        };
        let result = set_last_used(store.pool(), CONN, SCHEMA, TABLE, &state).await;
        assert!(result.is_err(), "oversized filters must be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("filters"), "error should name the field");
    }

    #[tokio::test]
    async fn set_last_used_rejects_oversized_search_term() {
        let (store, _dir) = ephemeral_store().await;
        let state = LastUsedState {
            sort_columns: serde_json::json!([]),
            filters: serde_json::json!({}),
            search_term: Some("x".repeat(65 * 1024)),
        };
        let result = set_last_used(store.pool(), CONN, SCHEMA, TABLE, &state).await;
        assert!(result.is_err(), "oversized search_term must be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("search_term"), "error should name the field");
    }

    #[tokio::test]
    async fn save_sort_preset_rejects_oversized_columns() {
        let (store, _dir) = ephemeral_store().await;
        let big = serde_json::Value::String("x".repeat(65 * 1024));
        let result = save_sort_preset(store.pool(), CONN, SCHEMA, TABLE, "Big", &big).await;
        assert!(
            result.is_err(),
            "oversized sort preset columns must be rejected"
        );
    }

    #[tokio::test]
    async fn save_filter_preset_rejects_oversized_filters() {
        let (store, _dir) = ephemeral_store().await;
        let big = serde_json::Value::String("x".repeat(65 * 1024));
        let result = save_filter_preset(store.pool(), CONN, SCHEMA, TABLE, "Big", &big).await;
        assert!(
            result.is_err(),
            "oversized filter preset filters must be rejected"
        );
    }

    // ── Filter presets ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn filter_preset_round_trip() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let filters = serde_json::json!({"status": "active"});
        let id = save_filter_preset(pool, CONN, SCHEMA, TABLE, "Active only", &filters)
            .await
            .unwrap();
        assert!(id > 0);

        let presets = list_filter_presets(pool, CONN, SCHEMA, TABLE)
            .await
            .unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].filters, filters);
    }

    #[tokio::test]
    async fn filter_preset_conflict_updates() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let f1 = serde_json::json!({"status": "active"});
        let f2 = serde_json::json!({"status": "inactive"});
        save_filter_preset(pool, CONN, SCHEMA, TABLE, "Status", &f1)
            .await
            .unwrap();
        save_filter_preset(pool, CONN, SCHEMA, TABLE, "Status", &f2)
            .await
            .unwrap();

        let presets = list_filter_presets(pool, CONN, SCHEMA, TABLE)
            .await
            .unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].filters, f2);
    }
}
