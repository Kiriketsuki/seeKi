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

#[derive(Debug, Clone, Serialize)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub row_count_estimate: Option<i64>,
}

impl TableInfo {
    /// Qualified name: `"schema.name"`.
    pub fn qualified(&self) -> String {
        format!("{}.{}", self.schema, self.name)
    }
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortEntry {
    pub column: String,
    pub direction: SortDirection,
}

/// Bundled parameters for querying rows from a table.
pub struct RowQueryParams<'a> {
    pub schema: &'a str,
    pub table: &'a str,
    pub page: u32,
    pub page_size: u32,
    pub sort: &'a [SortEntry],
    pub search: Option<&'a str>,
    pub filters: &'a HashMap<String, String>,
}

/// Parameters for CSV export (no pagination).
pub struct ExportQueryParams<'a> {
    pub schema: &'a str,
    pub table: &'a str,
    pub sort: &'a [SortEntry],
    pub search: Option<&'a str>,
    pub filters: &'a HashMap<String, String>,
}

pub enum DatabasePool {
    Postgres(sqlx::PgPool, Option<crate::ssh::SshTunnel>),
}

impl DatabasePool {
    pub async fn connect(
        config: &DatabaseConfig,
        ssh: Option<(&crate::config::SshConfig, &crate::config::SecretsConfig)>,
    ) -> anyhow::Result<Self> {
        match config.kind {
            DatabaseKind::Postgres => {
                let (connect_url, tunnel) = if let Some((ssh_config, secrets)) = ssh {
                    let parsed = url::Url::parse(&config.url)
                        .map_err(|e| anyhow::anyhow!("Invalid database URL: {e}"))?;
                    let db_host = parsed
                        .host_str()
                        .ok_or_else(|| anyhow::anyhow!("Database URL has no host"))?
                        .to_string();
                    let db_port = parsed.port().unwrap_or(5432);

                    let tunnel =
                        crate::ssh::SshTunnel::connect(ssh_config, secrets, &db_host, db_port)
                            .await?;

                    let mut new_url = parsed;
                    new_url
                        .set_host(Some("127.0.0.1"))
                        .map_err(|_| anyhow::anyhow!("Failed to rewrite URL host"))?;
                    new_url
                        .set_port(Some(tunnel.local_port()))
                        .map_err(|_| anyhow::anyhow!("Failed to rewrite URL port"))?;

                    (new_url.to_string(), Some(tunnel))
                } else {
                    (config.url.clone(), None)
                };

                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .connect(&connect_url)
                    .await?;
                Ok(Self::Postgres(pool, tunnel))
            }
            DatabaseKind::Sqlite => {
                anyhow::bail!("SQLite support coming in v0.2")
            }
        }
    }

    pub async fn list_tables(&self, schemas: &[String]) -> anyhow::Result<Vec<TableInfo>> {
        match self {
            Self::Postgres(pool, _) => postgres::list_tables(pool, schemas).await,
        }
    }

    /// List every non-system schema visible to the connected DB user.
    pub async fn list_schemas(&self) -> anyhow::Result<Vec<postgres::SchemaPreview>> {
        match self {
            Self::Postgres(pool, _) => postgres::list_schemas(pool).await,
        }
    }

    pub async fn get_columns(&self, schema: &str, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        match self {
            Self::Postgres(pool, _) => postgres::get_columns(pool, schema, table).await,
        }
    }

    pub async fn get_columns_bulk(
        &self,
        refs: &[(&str, &str)],
    ) -> anyhow::Result<std::collections::HashMap<(String, String), Vec<ColumnInfo>>> {
        match self {
            Self::Postgres(pool, _) => postgres::get_columns_bulk(pool, refs).await,
        }
    }

    pub async fn query_rows(&self, params: &RowQueryParams<'_>) -> anyhow::Result<QueryResult> {
        match self {
            Self::Postgres(pool, _) => postgres::query_rows(pool, params).await,
        }
    }

    pub fn ssh_connected(&self) -> bool {
        match self {
            Self::Postgres(_, tunnel) => tunnel.is_some(),
        }
    }

    /// Get a reference to the underlying PostgreSQL pool for streaming operations.
    /// Returns None if the pool is not PostgreSQL.
    pub fn pg_pool(&self) -> Option<&sqlx::PgPool> {
        match self {
            Self::Postgres(pool, _) => Some(pool),
        }
    }
}
