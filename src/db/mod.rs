pub mod postgres;

use std::collections::HashMap;

use crate::config::{DatabaseConfig, DatabaseKind};
use serde::{Deserialize, Serialize};
use sqlx::Row;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViewAggregate {
    Sum,
    Avg,
    Count,
    Min,
    Max,
}

impl ViewAggregate {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Sum => "SUM",
            Self::Avg => "AVG",
            Self::Count => "COUNT",
            Self::Min => "MIN",
            Self::Max => "MAX",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewColumn {
    pub source_schema: String,
    pub source_table: String,
    pub column_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<ViewAggregate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FkHop {
    pub from_schema: String,
    pub from_table: String,
    pub from_columns: Vec<String>,
    pub to_schema: String,
    pub to_table: String,
    pub to_columns: Vec<String>,
    pub constraint_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SavedViewAggregate {
    Sum,
    Avg,
    Count,
    Min,
    Max,
    Latest,
}

impl SavedViewAggregate {
    pub const fn as_legacy(self) -> Option<ViewAggregate> {
        match self {
            Self::Sum => Some(ViewAggregate::Sum),
            Self::Avg => Some(ViewAggregate::Avg),
            Self::Count => Some(ViewAggregate::Count),
            Self::Min => Some(ViewAggregate::Min),
            Self::Max => Some(ViewAggregate::Max),
            Self::Latest => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedViewColumnKind {
    Source,
    Derived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewSourceKind {
    Fk,
    Match,
    #[serde(rename = "self")]
    SelfJoin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewSourceMatch {
    pub base_column: String,
    pub source_column: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewSelfDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewSelfSource {
    pub entity_column: String,
    pub order_column: String,
    pub direction: ViewSelfDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewSourceRef {
    pub id: String,
    pub kind: ViewSourceKind,
    pub schema: String,
    pub table: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#match: Option<ViewSourceMatch>,
    #[serde(default, rename = "self", skip_serializing_if = "Option::is_none")]
    pub self_join: Option<ViewSelfSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewColumnRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub source_schema: String,
    pub source_table: String,
    pub column_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedViewSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewOrderBy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub source_schema: String,
    pub source_table: String,
    pub column_name: String,
    pub direction: SavedViewSortDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewGrouping {
    pub keys: Vec<ViewColumnRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_by: Option<ViewOrderBy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewRanking {
    pub partition_by: Vec<ViewColumnRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_by: Option<ViewOrderBy>,
    pub limit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewDerivedOperation {
    Difference,
    RatioPercent,
    AgeOfTimestamp,
    DateBucket,
    DatePart,
    TextConcat,
    TextLength,
    IfThen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewDerivedInputKind {
    Column,
    Literal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDerivedInput {
    pub kind: ViewDerivedInputKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDerivedColumn {
    pub operation: ViewDerivedOperation,
    pub inputs: Vec<ViewDerivedInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewTemplateId {
    Scratch,
    MostRecentPerGroup,
    CountsPerDay,
    TopNPerGroup,
    TotalsByWeek,
    PreviousRowDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ViewFilterValue {
    Legacy(String),
    Structured(ViewStructuredFilter),
}

impl ViewFilterValue {
    fn as_legacy_exact_match(&self) -> Option<&str> {
        match self {
            Self::Legacy(value) => Some(value.as_str()),
            Self::Structured(ViewStructuredFilter::Eq { value }) => Some(value.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ViewStructuredFilter {
    Eq { value: String },
    Gt { value: String },
    Gte { value: String },
    Lt { value: String },
    Lte { value: String },
    Contains { value: String },
    StartsWith { value: String },
    Between { value: [String; 2] },
    IsEmpty,
    InList { value: Vec<String> },
}

pub type ViewDefinitionFilters = HashMap<String, ViewFilterValue>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedViewColumn {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SavedViewColumnKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub source_schema: String,
    pub source_table: String,
    pub column_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<SavedViewAggregate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived: Option<ViewDerivedColumn>,
}

impl SavedViewColumn {
    pub fn requires_advanced_planner(&self) -> bool {
        matches!(self.kind, Some(SavedViewColumnKind::Derived))
            || self.derived.is_some()
            || matches!(self.aggregate, Some(SavedViewAggregate::Latest))
    }

    fn as_legacy_column(&self) -> ViewColumn {
        ViewColumn {
            source_schema: self.source_schema.clone(),
            source_table: self.source_table.clone(),
            column_name: self.column_name.clone(),
            alias: self.alias.clone(),
            aggregate: self.aggregate.and_then(SavedViewAggregate::as_legacy),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDefinitionShape {
    pub columns: Vec<SavedViewColumn>,
    #[serde(default)]
    pub filters: ViewDefinitionFilters,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<ViewSourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouping: Option<ViewGrouping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ranking: Option<ViewRanking>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<ViewTemplateId>,
}

#[derive(Debug, Clone)]
pub struct LegacyViewDefinitionShape {
    pub columns: Vec<ViewColumn>,
    pub filters: HashMap<String, String>,
}

pub enum PlannerCompatibility {
    Legacy(LegacyViewDefinitionShape),
    RequiresPlannerV2(String),
}

impl ViewDefinitionShape {
    pub fn planner_compatibility(&self) -> Result<PlannerCompatibility, ValidationError> {
        if self.grouping.is_some() {
            return Ok(PlannerCompatibility::RequiresPlannerV2(
                "Grouping metadata requires planner v2".into(),
            ));
        }
        if self.ranking.is_some() {
            return Ok(PlannerCompatibility::RequiresPlannerV2(
                "Ranking metadata requires planner v2".into(),
            ));
        }
        if self
            .sources
            .iter()
            .any(|source| !matches!(source.kind, ViewSourceKind::Fk))
        {
            return Ok(PlannerCompatibility::RequiresPlannerV2(
                "Custom match/self sources require planner v2".into(),
            ));
        }
        if let Some(column) = self
            .columns
            .iter()
            .find(|column| column.requires_advanced_planner())
        {
            let alias = column.alias.as_deref().unwrap_or(&column.column_name);
            return Ok(PlannerCompatibility::RequiresPlannerV2(format!(
                "Column '{alias}' uses v2-only view metadata"
            )));
        }

        let mut filters = HashMap::new();
        for (name, filter) in &self.filters {
            let Some(value) = filter.as_legacy_exact_match() else {
                return Ok(PlannerCompatibility::RequiresPlannerV2(format!(
                    "Filter '{name}' uses a v2-only operator"
                )));
            };
            filters.insert(name.clone(), value.to_string());
        }

        Ok(PlannerCompatibility::Legacy(LegacyViewDefinitionShape {
            columns: self
                .columns
                .iter()
                .map(SavedViewColumn::as_legacy_column)
                .collect(),
            filters,
        }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedViewSummary {
    pub id: i64,
    pub name: String,
    pub base_schema: String,
    pub base_table: String,
    pub definition_version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedViewDefinition {
    pub id: i64,
    pub name: String,
    pub base_schema: String,
    pub base_table: String,
    pub definition_version: i64,
    #[serde(flatten)]
    pub shape: ViewDefinitionShape,
    pub created_at: String,
    pub updated_at: String,
}

impl SavedViewDefinition {
    pub fn summary(&self) -> SavedViewSummary {
        SavedViewSummary {
            id: self.id,
            name: self.name.clone(),
            base_schema: self.base_schema.clone(),
            base_table: self.base_table.clone(),
            definition_version: self.definition_version,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewDraft<'a> {
    pub base_schema: &'a str,
    pub base_table: &'a str,
    pub columns: &'a [ViewColumn],
    pub filters: &'a HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ViewRowsQueryParams<'a> {
    pub draft: ViewDraft<'a>,
    pub page: u32,
    pub page_size: u32,
    pub sort: &'a [SortEntry],
    pub search: Option<&'a str>,
    pub filters: &'a HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ViewExportQueryParams<'a> {
    pub draft: ViewDraft<'a>,
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

    pub async fn sample_column_values(
        &self,
        schema: &str,
        table: &str,
        column: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<String>> {
        if !postgres::is_valid_identifier(schema) {
            anyhow::bail!("Invalid schema name: {schema}");
        }
        if !postgres::is_valid_identifier(table) {
            anyhow::bail!("Invalid table name: {table}");
        }
        if !postgres::is_valid_identifier(column) {
            anyhow::bail!("Invalid column name: {column}");
        }

        let column_ident = format!("\"{column}\"");
        let query = format!(
            r#"
            SELECT sample
            FROM (
                SELECT DISTINCT NULLIF(BTRIM({column_ident}::text), '') AS sample
                FROM "{schema}"."{table}"
                WHERE {column_ident} IS NOT NULL
            ) distinct_samples
            WHERE sample IS NOT NULL
            ORDER BY sample
            LIMIT $1
            "#,
        );

        let limit = i64::from(limit.clamp(1, 5));
        match self {
            Self::Postgres(pool, _) => Ok(sqlx::query(&query)
                .bind(limit)
                .fetch_all(pool)
                .await?
                .into_iter()
                .filter_map(|row| row.try_get::<Option<String>, _>("sample").ok().flatten())
                .collect()),
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

    pub async fn lookup_fk_path(
        &self,
        base_schema: &str,
        base_table: &str,
        target_schema: &str,
        target_table: &str,
    ) -> anyhow::Result<Vec<FkHop>> {
        match self {
            Self::Postgres(pool, _) => {
                postgres::lookup_fk_path(pool, base_schema, base_table, target_schema, target_table)
                    .await
            }
        }
    }

    pub async fn preview_view(
        &self,
        draft: &ViewDraft<'_>,
        page_size: u32,
    ) -> anyhow::Result<QueryResult> {
        match self {
            Self::Postgres(pool, _) => postgres::preview_view(pool, draft, page_size).await,
        }
    }

    pub async fn query_view_rows(
        &self,
        params: &ViewRowsQueryParams<'_>,
    ) -> anyhow::Result<QueryResult> {
        match self {
            Self::Postgres(pool, _) => postgres::query_view_rows(pool, params).await,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_shape() -> ViewDefinitionShape {
        ViewDefinitionShape {
            columns: vec![SavedViewColumn {
                kind: None,
                source_id: None,
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "id".into(),
                alias: Some("order_id".into()),
                aggregate: None,
                derived: None,
            }],
            filters: HashMap::from([("id".to_string(), ViewFilterValue::Legacy("42".into()))]),
            sources: Vec::new(),
            grouping: None,
            ranking: None,
            template: None,
        }
    }

    #[test]
    fn planner_compatibility_keeps_legacy_views_runnable() {
        let compat = sample_shape().planner_compatibility().unwrap();

        match compat {
            PlannerCompatibility::Legacy(legacy) => {
                assert_eq!(legacy.columns.len(), 1);
                assert_eq!(legacy.filters["id"], "42");
            }
            PlannerCompatibility::RequiresPlannerV2(reason) => {
                panic!("expected legacy planner compatibility, got {reason}");
            }
        }
    }

    #[test]
    fn planner_compatibility_flags_structured_filters_for_v2() {
        let mut shape = sample_shape();
        shape.filters.insert(
            "battery_soc".into(),
            ViewFilterValue::Structured(ViewStructuredFilter::Lt { value: "20".into() }),
        );

        let compat = shape.planner_compatibility().unwrap();
        match compat {
            PlannerCompatibility::Legacy(_) => panic!("expected v2 planner requirement"),
            PlannerCompatibility::RequiresPlannerV2(reason) => {
                assert!(reason.contains("battery_soc"));
            }
        }
    }

    #[test]
    fn saved_view_column_round_trips_latest_aggregate() {
        let column = SavedViewColumn {
            kind: Some(SavedViewColumnKind::Source),
            source_id: Some("base".into()),
            source_schema: "public".into(),
            source_table: "orders".into(),
            column_name: "battery_soc".into(),
            alias: Some("latest_soc".into()),
            aggregate: Some(SavedViewAggregate::Latest),
            derived: None,
        };

        let json = serde_json::to_value(&column).unwrap();
        assert_eq!(json["aggregate"], "LATEST");

        let round_trip: SavedViewColumn = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip.aggregate, Some(SavedViewAggregate::Latest));
    }
}
