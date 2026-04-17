use std::collections::HashMap;

use anyhow::Result;
use sqlx::SqlitePool;

use crate::db::{SavedViewDefinition, SavedViewSummary, ViewColumn};

const MAX_JSON_BYTES: usize = 64 * 1024; // 64 KiB
const DEFINITION_VERSION: i64 = 1;

type SavedViewRow = (
    i64,
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
    String,
);

fn check_json_size(json: &str, context: &str) -> Result<()> {
    if json.len() > MAX_JSON_BYTES {
        anyhow::bail!("{context} exceeds maximum size of 64 KiB");
    }
    Ok(())
}

pub async fn list_views(pool: &SqlitePool, conn_id: &str) -> Result<Vec<SavedViewSummary>> {
    let rows: Vec<(i64, String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT id, name, base_schema, base_table, definition_version, created_at, updated_at
         FROM saved_views
         WHERE connection_id = ?
         ORDER BY LOWER(name), id",
    )
    .bind(conn_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, name, base_schema, base_table, definition_version, created_at, updated_at)| {
                SavedViewSummary {
                    id,
                    name,
                    base_schema,
                    base_table,
                    definition_version,
                    created_at,
                    updated_at,
                }
            },
        )
        .collect())
}

pub async fn get_view(
    pool: &SqlitePool,
    conn_id: &str,
    id: i64,
) -> Result<Option<SavedViewDefinition>> {
    let row: Option<SavedViewRow> = sqlx::query_as(
        "SELECT id, name, base_schema, base_table, definition_version, columns, filters, created_at, updated_at
         FROM saved_views
         WHERE connection_id = ? AND id = ?",
    )
    .bind(conn_id)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(
        |(
            id,
            name,
            base_schema,
            base_table,
            definition_version,
            columns,
            filters,
            created_at,
            updated_at,
        )| {
            Ok(SavedViewDefinition {
                id,
                name,
                base_schema,
                base_table,
                definition_version,
                columns: serde_json::from_str(&columns)?,
                filters: serde_json::from_str(&filters)?,
                created_at,
                updated_at,
            })
        },
    )
    .transpose()
}

pub async fn create_view(
    pool: &SqlitePool,
    conn_id: &str,
    name: &str,
    base_schema: &str,
    base_table: &str,
    columns: &[ViewColumn],
    filters: &HashMap<String, String>,
) -> Result<SavedViewDefinition> {
    let columns_json = serde_json::to_string(columns)?;
    check_json_size(&columns_json, "saved view columns")?;
    let filters_json = serde_json::to_string(filters)?;
    check_json_size(&filters_json, "saved view filters")?;

    let result = sqlx::query(
        "INSERT INTO saved_views
            (connection_id, name, base_schema, base_table, definition_version, columns, filters)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(conn_id)
    .bind(name)
    .bind(base_schema)
    .bind(base_table)
    .bind(DEFINITION_VERSION)
    .bind(columns_json)
    .bind(filters_json)
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();
    get_view(pool, conn_id, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Saved view was created but could not be reloaded"))
}

pub async fn rename_view(pool: &SqlitePool, conn_id: &str, id: i64, name: &str) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE saved_views
         SET name = ?, updated_at = datetime('now')
         WHERE connection_id = ? AND id = ?",
    )
    .bind(name)
    .bind(conn_id)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_view(pool: &SqlitePool, conn_id: &str, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM saved_views WHERE connection_id = ? AND id = ?")
        .bind(conn_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::testutil::ephemeral_store;

    fn sample_columns() -> Vec<ViewColumn> {
        vec![
            ViewColumn {
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "id".into(),
                alias: None,
                aggregate: None,
            },
            ViewColumn {
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "total".into(),
                alias: Some("order_total".into()),
                aggregate: Some(crate::db::ViewAggregate::Sum),
            },
        ]
    }

    #[tokio::test]
    async fn create_and_get_view_round_trip() {
        let (store, _dir) = ephemeral_store().await;
        let filters = HashMap::from([("id".to_string(), "42".to_string())]);
        let created = create_view(
            store.pool(),
            "conn",
            "Orders",
            "public",
            "orders",
            &sample_columns(),
            &filters,
        )
        .await
        .unwrap();

        assert_eq!(created.name, "Orders");
        assert_eq!(created.base_schema, "public");
        assert_eq!(created.base_table, "orders");
        assert_eq!(created.columns.len(), 2);
        assert_eq!(created.filters, filters);

        let loaded = get_view(store.pool(), "conn", created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "Orders");
        assert_eq!(loaded.columns[1].alias.as_deref(), Some("order_total"));
    }

    #[tokio::test]
    async fn list_views_returns_sorted_summaries() {
        let (store, _dir) = ephemeral_store().await;
        create_view(
            store.pool(),
            "conn",
            "Zoo",
            "public",
            "orders",
            &sample_columns(),
            &HashMap::new(),
        )
        .await
        .unwrap();
        create_view(
            store.pool(),
            "conn",
            "Alpha",
            "public",
            "orders",
            &sample_columns(),
            &HashMap::new(),
        )
        .await
        .unwrap();

        let items = list_views(store.pool(), "conn").await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "Alpha");
        assert_eq!(items[1].name, "Zoo");
    }

    #[tokio::test]
    async fn rename_and_delete_view_work() {
        let (store, _dir) = ephemeral_store().await;
        let created = create_view(
            store.pool(),
            "conn",
            "Orders",
            "public",
            "orders",
            &sample_columns(),
            &HashMap::new(),
        )
        .await
        .unwrap();

        assert!(
            rename_view(store.pool(), "conn", created.id, "Renamed")
                .await
                .unwrap()
        );
        let renamed = get_view(store.pool(), "conn", created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(renamed.name, "Renamed");

        assert!(delete_view(store.pool(), "conn", created.id).await.unwrap());
        assert!(
            get_view(store.pool(), "conn", created.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn views_are_isolated_by_connection() {
        let (store, _dir) = ephemeral_store().await;
        let created = create_view(
            store.pool(),
            "conn-a",
            "Orders",
            "public",
            "orders",
            &sample_columns(),
            &HashMap::new(),
        )
        .await
        .unwrap();

        assert!(
            get_view(store.pool(), "conn-b", created.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(list_views(store.pool(), "conn-b").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rename_missing_view_returns_false() {
        let (store, _dir) = ephemeral_store().await;
        assert!(
            !rename_view(store.pool(), "conn", 999, "Renamed")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn create_view_rejects_oversized_columns_json() {
        let (store, _dir) = ephemeral_store().await;
        let huge_alias = "x".repeat(70 * 1024);
        let columns = vec![ViewColumn {
            source_schema: "public".into(),
            source_table: "orders".into(),
            column_name: "id".into(),
            alias: Some(huge_alias),
            aggregate: None,
        }];

        let error = create_view(
            store.pool(),
            "conn",
            "Too Big",
            "public",
            "orders",
            &columns,
            &HashMap::new(),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("saved view columns"));
    }

    #[tokio::test]
    async fn create_view_rejects_oversized_filters_json() {
        let (store, _dir) = ephemeral_store().await;
        let filters = HashMap::from([("name".to_string(), "x".repeat(70 * 1024))]);

        let error = create_view(
            store.pool(),
            "conn",
            "Too Big",
            "public",
            "orders",
            &sample_columns(),
            &filters,
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("saved view filters"));
    }

    #[tokio::test]
    async fn create_view_enforces_name_uniqueness_per_connection() {
        let (store, _dir) = ephemeral_store().await;
        create_view(
            store.pool(),
            "conn",
            "Orders",
            "public",
            "orders",
            &sample_columns(),
            &HashMap::new(),
        )
        .await
        .unwrap();

        let error = create_view(
            store.pool(),
            "conn",
            "Orders",
            "public",
            "orders",
            &sample_columns(),
            &HashMap::new(),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().to_lowercase().contains("unique"));
    }
}
