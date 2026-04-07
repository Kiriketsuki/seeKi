mod postgres;

use crate::config::{DatabaseConfig, DatabaseKind};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub row_count_estimate: i64,
}

#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub display_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<serde_json::Value>,
    pub total_rows: i64,
    pub page: u32,
    pub page_size: u32,
}

pub enum DatabasePool {
    Postgres(sqlx::PgPool),
}

impl DatabasePool {
    pub async fn connect(config: &DatabaseConfig) -> anyhow::Result<Self> {
        match config.kind {
            DatabaseKind::Postgres => {
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .connect(&config.url)
                    .await?;
                Ok(Self::Postgres(pool))
            }
            DatabaseKind::Sqlite => {
                anyhow::bail!("SQLite support coming in v0.2")
            }
        }
    }

    pub async fn list_tables(&self) -> anyhow::Result<Vec<TableInfo>> {
        match self {
            Self::Postgres(pool) => postgres::list_tables(pool).await,
        }
    }

    pub async fn get_columns(&self, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        match self {
            Self::Postgres(pool) => postgres::get_columns(pool, table).await,
        }
    }

    pub async fn query_rows(
        &self,
        table: &str,
        page: u32,
        page_size: u32,
        sort_column: Option<&str>,
        sort_direction: Option<&str>,
        search: Option<&str>,
    ) -> anyhow::Result<QueryResult> {
        match self {
            Self::Postgres(pool) => {
                postgres::query_rows(pool, table, page, page_size, sort_column, sort_direction, search).await
            }
        }
    }
}
