use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db::{
    SavedViewColumn, SavedViewDefinition, SavedViewSummary, ViewDefinitionFilters,
    ViewDefinitionShape, ViewGrouping, ViewRanking, ViewSourceRef, ViewTemplateId,
};

const MAX_JSON_BYTES: usize = 64 * 1024; // 64 KiB
const DEFINITION_VERSION: i64 = 2;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredViewColumnsEnvelope {
    columns: Vec<SavedViewColumn>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sources: Vec<ViewSourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grouping: Option<ViewGrouping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ranking: Option<ViewRanking>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template: Option<ViewTemplateId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum StoredViewColumns {
    Legacy(Vec<SavedViewColumn>),
    V2(StoredViewColumnsEnvelope),
}

impl StoredViewColumns {
    fn from_shape(shape: &ViewDefinitionShape) -> Self {
        Self::V2(StoredViewColumnsEnvelope {
            columns: shape.columns.clone(),
            sources: shape.sources.clone(),
            grouping: shape.grouping.clone(),
            ranking: shape.ranking.clone(),
            template: shape.template,
        })
    }

    fn into_shape(self, filters: ViewDefinitionFilters) -> ViewDefinitionShape {
        match self {
            Self::Legacy(columns) => ViewDefinitionShape {
                columns,
                filters,
                sources: Vec::new(),
                grouping: None,
                ranking: None,
                template: None,
            },
            Self::V2(columns) => ViewDefinitionShape {
                columns: columns.columns,
                filters,
                sources: columns.sources,
                grouping: columns.grouping,
                ranking: columns.ranking,
                template: columns.template,
            },
        }
    }
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
            let filters: ViewDefinitionFilters = serde_json::from_str(&filters)?;
            let stored_columns: StoredViewColumns = serde_json::from_str(&columns)?;
            Ok(SavedViewDefinition {
                id,
                name,
                base_schema,
                base_table,
                definition_version,
                shape: stored_columns.into_shape(filters),
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
    shape: &ViewDefinitionShape,
) -> Result<SavedViewDefinition> {
    let columns_json = serde_json::to_string(&StoredViewColumns::from_shape(shape))?;
    check_json_size(&columns_json, "saved view columns")?;
    let filters_json = serde_json::to_string(&shape.filters)?;
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
    use std::collections::HashMap;

    use crate::db::{ViewFilterValue, ViewSourceKind};
    use crate::store::testutil::ephemeral_store;

    fn sample_shape() -> ViewDefinitionShape {
        ViewDefinitionShape {
            columns: vec![
                SavedViewColumn {
                    kind: None,
                    source_id: None,
                    source_schema: "public".into(),
                    source_table: "orders".into(),
                    column_name: "id".into(),
                    alias: None,
                    aggregate: None,
                    derived: None,
                },
                SavedViewColumn {
                    kind: None,
                    source_id: None,
                    source_schema: "public".into(),
                    source_table: "orders".into(),
                    column_name: "total".into(),
                    alias: Some("order_total".into()),
                    aggregate: Some(crate::db::SavedViewAggregate::Sum),
                    derived: None,
                },
            ],
            filters: HashMap::from([("id".to_string(), ViewFilterValue::Legacy("42".into()))]),
            sources: vec![ViewSourceRef {
                id: "orders".into(),
                kind: ViewSourceKind::Fk,
                schema: "public".into(),
                table: "orders".into(),
                label: Some("Orders".into()),
                r#match: None,
                self_join: None,
            }],
            grouping: None,
            ranking: None,
            template: Some(ViewTemplateId::Scratch),
        }
    }

    #[tokio::test]
    async fn create_and_get_view_round_trip() {
        let (store, _dir) = ephemeral_store().await;
        let shape = sample_shape();
        let created = create_view(store.pool(), "conn", "Orders", "public", "orders", &shape)
            .await
            .unwrap();

        assert_eq!(created.name, "Orders");
        assert_eq!(created.base_schema, "public");
        assert_eq!(created.base_table, "orders");
        assert_eq!(created.shape.columns.len(), 2);
        assert_eq!(created.shape.filters, shape.filters);
        assert_eq!(created.definition_version, 2);

        let loaded = get_view(store.pool(), "conn", created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "Orders");
        assert_eq!(
            loaded.shape.columns[1].alias.as_deref(),
            Some("order_total")
        );
        assert_eq!(loaded.shape.template, Some(ViewTemplateId::Scratch));
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
            &ViewDefinitionShape {
                filters: HashMap::new(),
                ..sample_shape()
            },
        )
        .await
        .unwrap();
        create_view(
            store.pool(),
            "conn",
            "Alpha",
            "public",
            "orders",
            &ViewDefinitionShape {
                filters: HashMap::new(),
                ..sample_shape()
            },
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
            &ViewDefinitionShape {
                filters: HashMap::new(),
                ..sample_shape()
            },
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
            &ViewDefinitionShape {
                filters: HashMap::new(),
                ..sample_shape()
            },
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
        let shape = ViewDefinitionShape {
            columns: vec![SavedViewColumn {
                kind: None,
                source_id: None,
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "id".into(),
                alias: Some(huge_alias),
                aggregate: None,
                derived: None,
            }],
            filters: HashMap::new(),
            sources: Vec::new(),
            grouping: None,
            ranking: None,
            template: None,
        };

        let error = create_view(store.pool(), "conn", "Too Big", "public", "orders", &shape)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("saved view columns"));
    }

    #[tokio::test]
    async fn create_view_rejects_oversized_filters_json() {
        let (store, _dir) = ephemeral_store().await;
        let shape = ViewDefinitionShape {
            filters: HashMap::from([(
                "name".to_string(),
                ViewFilterValue::Legacy("x".repeat(70 * 1024)),
            )]),
            ..sample_shape()
        };

        let error = create_view(store.pool(), "conn", "Too Big", "public", "orders", &shape)
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
            &ViewDefinitionShape {
                filters: HashMap::new(),
                ..sample_shape()
            },
        )
        .await
        .unwrap();

        let error = create_view(
            store.pool(),
            "conn",
            "Orders",
            "public",
            "orders",
            &ViewDefinitionShape {
                filters: HashMap::new(),
                ..sample_shape()
            },
        )
        .await
        .unwrap_err();

        assert!(error.to_string().to_lowercase().contains("unique"));
    }

    #[tokio::test]
    async fn get_view_reads_v1_rows_with_defaults() {
        let (store, _dir) = ephemeral_store().await;
        sqlx::query(
            "INSERT INTO saved_views
                (connection_id, name, base_schema, base_table, definition_version, columns, filters)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("conn")
        .bind("Legacy")
        .bind("public")
        .bind("orders")
        .bind(1_i64)
        .bind(
            r#"[{"source_schema":"public","source_table":"orders","column_name":"id","alias":"legacy_id"}]"#,
        )
        .bind(r#"{"id":"42"}"#)
        .execute(store.pool())
        .await
        .unwrap();

        let loaded = get_view(store.pool(), "conn", 1).await.unwrap().unwrap();
        assert_eq!(loaded.definition_version, 1);
        assert_eq!(loaded.shape.columns.len(), 1);
        assert_eq!(loaded.shape.columns[0].alias.as_deref(), Some("legacy_id"));
        assert_eq!(
            loaded.shape.filters.get("id"),
            Some(&ViewFilterValue::Legacy("42".into()))
        );
        assert!(loaded.shape.sources.is_empty());
        assert!(loaded.shape.grouping.is_none());
        assert!(loaded.shape.ranking.is_none());
        assert!(loaded.shape.template.is_none());
    }

    #[tokio::test]
    async fn create_view_writes_v2_columns_envelope() {
        let (store, _dir) = ephemeral_store().await;
        let created = create_view(
            store.pool(),
            "conn",
            "Orders",
            "public",
            "orders",
            &sample_shape(),
        )
        .await
        .unwrap();

        let row: (i64, String, String) = sqlx::query_as(
            "SELECT definition_version, columns, filters FROM saved_views WHERE id = ?",
        )
        .bind(created.id)
        .fetch_one(store.pool())
        .await
        .unwrap();

        assert_eq!(row.0, 2);
        let columns_json: serde_json::Value = serde_json::from_str(&row.1).unwrap();
        assert!(columns_json["columns"].is_array());
        assert_eq!(columns_json["template"], "scratch");
        let filters_json: serde_json::Value = serde_json::from_str(&row.2).unwrap();
        assert_eq!(filters_json["id"], "42");
    }
}
