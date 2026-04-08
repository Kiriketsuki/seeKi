pub mod postgres;

use std::collections::HashMap;

use crate::config::{DatabaseConfig, DatabaseKind};
use serde::Serialize;

/// Client-facing validation error (e.g. invalid column name in filter).
/// The API layer maps this to HTTP 400.
#[derive(Debug)]
pub struct ValidationError(pub String);

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ValidationError {}

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

/// Bundled parameters for querying rows from a table.
pub struct RowQueryParams<'a> {
    pub table: &'a str,
    pub page: u32,
    pub page_size: u32,
    pub sort_column: Option<&'a str>,
    pub sort_direction: Option<&'a str>,
    pub search: Option<&'a str>,
    pub filters: &'a HashMap<String, String>,
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

    pub async fn query_rows(&self, params: &RowQueryParams<'_>) -> anyhow::Result<QueryResult> {
        match self {
            Self::Postgres(pool) => postgres::query_rows(pool, params).await,
        }
    }
}
