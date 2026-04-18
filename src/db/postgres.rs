use std::collections::{HashMap, HashSet, VecDeque};
use std::pin::Pin;

use futures::Stream;
use serde::Serialize;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use super::{
    ColumnInfo, ExportQueryParams, FkHop, QueryResult, RowQueryParams, SavedViewAggregate,
    SavedViewColumn, SavedViewColumnKind, SavedViewSortDirection, SortDirection, SortEntry,
    TableInfo, ValidationError, ViewAggregate, ViewColumn, ViewColumnRef, ViewDefinitionShape,
    ViewDerivedColumn, ViewDerivedInput, ViewDerivedInputKind, ViewDerivedOperation, ViewDraft,
    ViewExportQueryParams, ViewFilterValue, ViewOrderBy, ViewRowsQueryParams, ViewSelfDirection,
    ViewSourceKind,
};

/// Result of a connection test — describes a single table visible to the server.
#[derive(Debug, Serialize)]
pub struct TablePreview {
    pub schema: String,
    pub name: String, // always the bare table name (never "schema.table")
    pub estimated_rows: i64,
    pub is_system: bool,
}

/// Test a PostgreSQL connection and return a preview of all visible tables.
/// If `ssh` is provided, an SSH tunnel is established first.
pub async fn test_connection(
    url: &str,
    ssh: Option<(&crate::config::SshConfig, &crate::config::SecretsConfig)>,
) -> anyhow::Result<(Vec<TablePreview>, Vec<SchemaPreview>)> {
    let (connect_url, _tunnel) = if let Some((ssh_config, secrets)) = ssh {
        let parsed =
            url::Url::parse(url).map_err(|e| anyhow::anyhow!("Invalid database URL: {e}"))?;
        let db_host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Database URL has no host"))?
            .to_string();
        let db_port = parsed.port().unwrap_or(5432);

        let tunnel = crate::ssh::SshTunnel::connect(ssh_config, secrets, &db_host, db_port).await?;

        let mut new_url = parsed;
        new_url
            .set_host(Some("127.0.0.1"))
            .map_err(|_| anyhow::anyhow!("Failed to rewrite URL host for SSH tunnel"))?;
        new_url
            .set_port(Some(tunnel.local_port()))
            .map_err(|_| anyhow::anyhow!("Failed to rewrite URL port for SSH tunnel"))?;

        (new_url.to_string(), Some(tunnel))
    } else {
        (url.to_string(), None)
    };

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&connect_url)
        .await?;

    let rows = sqlx::query(
        r#"
        SELECT
            n.nspname AS schema_name,
            c.relname AS table_name,
            c.reltuples::bigint AS row_estimate
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE c.relkind = 'r'
        ORDER BY n.nspname, c.relname
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let system_schemas = ["information_schema", "pg_catalog"];
    let tables: Vec<TablePreview> = rows
        .iter()
        .map(|r| {
            let schema: String = r.get("schema_name");
            let table_name: String = r.get("table_name");
            let is_system =
                system_schemas.contains(&schema.as_str()) || table_name.starts_with("pg_");
            TablePreview {
                schema,
                name: table_name, // always bare; schema is carried in the dedicated field
                estimated_rows: r.get("row_estimate"),
                is_system,
            }
        })
        .collect();

    let schemas = list_schemas(&pool).await.unwrap_or_default();

    pool.close().await;

    Ok((tables, schemas))
}

/// A schema preview for the setup wizard: name + count of base tables.
#[derive(Debug, Serialize, Clone)]
pub struct SchemaPreview {
    pub name: String,
    pub table_count: i64,
}

/// List every non-system schema visible to the connected DB user, with table counts.
/// Excludes `pg_catalog`, `information_schema`, and any `pg_%` schemas.
pub async fn list_schemas(pool: &PgPool) -> anyhow::Result<Vec<SchemaPreview>> {
    let rows = sqlx::query(
        r#"
        SELECT
            n.nspname AS schema_name,
            COUNT(c.oid) FILTER (WHERE c.relkind = 'r') AS table_count
        FROM pg_namespace n
        LEFT JOIN pg_class c ON c.relnamespace = n.oid
        WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
          AND n.nspname NOT LIKE 'pg_%'
          AND has_schema_privilege(n.oid, 'USAGE')
        GROUP BY n.nspname
        ORDER BY n.nspname
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| SchemaPreview {
            name: r.get("schema_name"),
            table_count: r.try_get::<i64, _>("table_count").unwrap_or(0),
        })
        .collect())
}

/// List all user tables in the given schemas with estimated row counts.
pub async fn list_tables(pool: &PgPool, schemas: &[String]) -> anyhow::Result<Vec<TableInfo>> {
    for schema in schemas {
        if !is_valid_identifier(schema) {
            anyhow::bail!("Invalid schema name in config: {schema}");
        }
    }

    let rows = sqlx::query(
        r#"
        SELECT
            n.nspname AS schema_name,
            c.relname AS table_name,
            c.reltuples::bigint AS row_estimate
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = ANY($1)
          AND c.relkind = 'r'
        ORDER BY n.nspname, c.relname
        "#,
    )
    .bind(schemas)
    .fetch_all(pool)
    .await?;

    let tables = rows
        .iter()
        .map(|r| {
            let raw_estimate: i64 = r.get("row_estimate");
            TableInfo {
                schema: r.get("schema_name"),
                name: r.get("table_name"),
                row_count_estimate: if raw_estimate < 0 {
                    None
                } else {
                    Some(raw_estimate)
                },
            }
        })
        .collect();

    Ok(tables)
}

/// Get column metadata for a table in a specific schema.
pub async fn get_columns(
    pool: &PgPool,
    schema: &str,
    table: &str,
) -> anyhow::Result<Vec<ColumnInfo>> {
    if !is_valid_identifier(schema) {
        anyhow::bail!("Invalid schema name: {schema}");
    }
    let rows = sqlx::query(
        r#"
        SELECT
            c.column_name,
            c.data_type,
            c.udt_name,
            c.is_nullable,
            CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END AS is_pk
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
               AND tc.table_schema = kcu.table_schema
            WHERE tc.table_schema = $1
              AND tc.table_name = $2
              AND tc.constraint_type = 'PRIMARY KEY'
        ) pk ON pk.column_name = c.column_name
        WHERE c.table_schema = $1
          AND c.table_name = $2
        ORDER BY c.ordinal_position
        "#,
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await?;

    let columns = rows
        .iter()
        .map(|r| {
            let udt: String = r.get("udt_name");
            let data_type: String = r.get("data_type");
            ColumnInfo {
                name: r.get("column_name"),
                display_type: humanize_type(&data_type, &udt),
                data_type,
                is_nullable: r.get::<String, _>("is_nullable") == "YES",
                is_primary_key: r.get("is_pk"),
            }
        })
        .collect();

    Ok(columns)
}

/// Get column metadata for multiple (schema, table) pairs in a single query.
/// Returns a map from (schema, table) to its columns.
pub async fn get_columns_bulk(
    pool: &PgPool,
    refs: &[(&str, &str)],
) -> anyhow::Result<HashMap<(String, String), Vec<ColumnInfo>>> {
    if refs.is_empty() {
        return Ok(HashMap::new());
    }

    // Build paired (schema, table) arrays for the UNNEST filter.
    // Validate all names for defense-in-depth symmetry before binding.
    let mut pair_schemas: Vec<String> = Vec::with_capacity(refs.len());
    let mut pair_tables: Vec<String> = Vec::with_capacity(refs.len());
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for (s, t) in refs {
        if !is_valid_identifier(s) {
            anyhow::bail!("Invalid schema name: {s}");
        }
        if !is_valid_identifier(t) {
            anyhow::bail!("Invalid table name: {t}");
        }
        let pair = ((*s).to_string(), (*t).to_string());
        if seen.insert(pair.clone()) {
            pair_schemas.push(pair.0);
            pair_tables.push(pair.1);
        }
    }

    let wanted: std::collections::HashSet<(String, String)> = seen;

    let rows = sqlx::query(
        r#"
        SELECT
            c.table_schema,
            c.table_name,
            c.column_name,
            c.data_type,
            c.udt_name,
            c.is_nullable,
            CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END AS is_pk
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT tc.table_schema, tc.table_name, kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
               AND tc.table_schema = kcu.table_schema
            WHERE (tc.table_schema, tc.table_name) IN (
                SELECT s, t FROM UNNEST($1::text[], $2::text[]) AS u(s, t)
            )
              AND tc.constraint_type = 'PRIMARY KEY'
        ) pk
          ON pk.table_schema = c.table_schema
         AND pk.table_name = c.table_name
         AND pk.column_name = c.column_name
        WHERE (c.table_schema, c.table_name) IN (
            SELECT s, t FROM UNNEST($1::text[], $2::text[]) AS u(s, t)
        )
        ORDER BY c.table_schema, c.table_name, c.ordinal_position
        "#,
    )
    .bind(&pair_schemas)
    .bind(&pair_tables)
    .fetch_all(pool)
    .await?;

    let mut result: HashMap<(String, String), Vec<ColumnInfo>> = HashMap::new();
    for r in &rows {
        let schema_name: String = r.get("table_schema");
        let table_name: String = r.get("table_name");
        let key = (schema_name, table_name);
        // Defensive check: only include rows that matched a requested (schema, table) pair.
        if !wanted.contains(&key) {
            continue;
        }
        let udt: String = r.get("udt_name");
        let data_type: String = r.get("data_type");
        let col = ColumnInfo {
            name: r.get("column_name"),
            display_type: humanize_type(&data_type, &udt),
            data_type,
            is_nullable: r.get::<String, _>("is_nullable") == "YES",
            is_primary_key: r.get("is_pk"),
        };
        result.entry(key).or_default().push(col);
    }

    Ok(result)
}

/// Validate that a name is safe for use as a double-quoted SQL identifier.
/// Allows alphanumeric, underscore, hyphen, and space — all safe inside `"..."`.
pub(crate) fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TableKey {
    schema: String,
    table: String,
}

impl TableKey {
    fn new(schema: &str, table: &str) -> Self {
        Self {
            schema: schema.to_string(),
            table: table.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct FkEdge {
    source: TableKey,
    target: TableKey,
    source_columns: Vec<String>,
    target_columns: Vec<String>,
    constraint_name: String,
}

#[derive(Debug, Clone)]
struct PlannedViewQuery {
    inner_sql: String,
    output_columns: Vec<ColumnInfo>,
    bind_values: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannerAggregate {
    Sum,
    Avg,
    Count,
    Min,
    Max,
    Latest,
}

impl PlannerAggregate {
    const fn as_sql(self) -> &'static str {
        match self {
            Self::Sum => "SUM",
            Self::Avg => "AVG",
            Self::Count => "COUNT",
            Self::Min => "MIN",
            Self::Max => "MAX",
            Self::Latest => "LATEST",
        }
    }

    fn from_view_aggregate(aggregate: ViewAggregate) -> anyhow::Result<Self> {
        match aggregate.as_sql() {
            "SUM" => Ok(Self::Sum),
            "AVG" => Ok(Self::Avg),
            "COUNT" => Ok(Self::Count),
            "MIN" => Ok(Self::Min),
            "MAX" => Ok(Self::Max),
            "LATEST" => Ok(Self::Latest),
            other => Err(ValidationError(format!(
                "Unsupported view aggregate in planner: {other}"
            ))
            .into()),
        }
    }

    const fn is_standard_group_aggregate(self) -> bool {
        !matches!(self, Self::Latest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PlannerBinding {
    name: String,
    table: TableKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PlannerFieldRef {
    binding: String,
    column: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannerFilterValue {
    Eq(String),
    Gt(String),
    Gte(String),
    Lt(String),
    Lte(String),
    Contains(String),
    StartsWith(String),
    Between(String, String),
    IsEmpty,
    InList(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerFilter {
    output_name: String,
    value: PlannerFilterValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannerDateBucket {
    Day,
    Week,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannerDatePart {
    Year,
    Month,
    Weekday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannerCompareOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl PlannerCompareOp {
    const fn as_sql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannerLiteral {
    String(String),
    Number(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannerCondition {
    IsEmpty(Box<PlannerValueExpr>),
    Compare {
        left: Box<PlannerValueExpr>,
        op: PlannerCompareOp,
        right: Box<PlannerValueExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannerValueExpr {
    Field(PlannerFieldRef),
    Literal(PlannerLiteral),
    Difference {
        left: Box<PlannerValueExpr>,
        right: Box<PlannerValueExpr>,
    },
    RatioPercent {
        numerator: Box<PlannerValueExpr>,
        denominator: Box<PlannerValueExpr>,
    },
    AgeOfTimestamp {
        value: Box<PlannerValueExpr>,
    },
    DateBucket {
        value: Box<PlannerValueExpr>,
        bucket: PlannerDateBucket,
    },
    DatePart {
        value: Box<PlannerValueExpr>,
        part: PlannerDatePart,
    },
    TextConcat {
        parts: Vec<PlannerValueExpr>,
    },
    TextLength {
        value: Box<PlannerValueExpr>,
    },
    IfThen {
        condition: PlannerCondition,
        then_expr: Box<PlannerValueExpr>,
        else_expr: Box<PlannerValueExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerProjection {
    output_name: String,
    expr: PlannerValueExpr,
    aggregate: Option<PlannerAggregate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerOrderExpr {
    expr: PlannerValueExpr,
    direction: SortDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerLatestPerGroup {
    partition_by: Vec<PlannerValueExpr>,
    order_by: PlannerOrderExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerRankLimit {
    partition_by: Vec<PlannerValueExpr>,
    order_by: PlannerOrderExpr,
    per_group_limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerJoinColumnPair {
    source_column: String,
    target_column: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannerNeighborDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannerJoinKind {
    MatchColumns {
        pairs: Vec<PlannerJoinColumnPair>,
    },
    PreviousRow {
        entity_pairs: Vec<PlannerJoinColumnPair>,
        order_by_column: String,
        direction: PlannerNeighborDirection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerJoin {
    source_binding: String,
    target_binding: PlannerBinding,
    kind: PlannerJoinKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannerDraftDef {
    base_binding: String,
    bindings: Vec<PlannerBinding>,
    joins: Vec<PlannerJoin>,
    projections: Vec<PlannerProjection>,
    filters: Vec<PlannerFilter>,
    group_by: Vec<PlannerValueExpr>,
    latest_per_group: Option<PlannerLatestPerGroup>,
    rank_limit: Option<PlannerRankLimit>,
}

#[derive(Debug, Clone)]
struct PlannerCatalog {
    columns_by_table: HashMap<(String, String), Vec<ColumnInfo>>,
    fk_edges: Vec<FkEdge>,
}

#[derive(Debug, Clone)]
struct ExpressionInfo {
    data_type: String,
    display_type: String,
    is_nullable: bool,
    is_primary_key: bool,
}

impl ExpressionInfo {
    fn from_column(column: &ColumnInfo) -> Self {
        Self {
            data_type: column.data_type.clone(),
            display_type: column.display_type.clone(),
            is_nullable: column.is_nullable,
            is_primary_key: column.is_primary_key,
        }
    }

    fn to_output_column(&self, name: String) -> ColumnInfo {
        ColumnInfo {
            name,
            data_type: self.data_type.clone(),
            display_type: self.display_type.clone(),
            is_nullable: self.is_nullable,
            is_primary_key: self.is_primary_key,
        }
    }
}

#[derive(Debug, Clone)]
struct CompiledExpr {
    sql: String,
    info: ExpressionInfo,
}

#[derive(Debug, Clone)]
struct PlannedProjectionDef {
    output_name: String,
    expr_sql: String,
    info: ExpressionInfo,
    aggregate: Option<PlannerAggregate>,
}

#[derive(Debug, Clone)]
struct DefinitionFilterTarget {
    output_name: String,
    sql_expr: String,
    info: ExpressionInfo,
    aggregate: Option<PlannerAggregate>,
}

#[derive(Debug, Default, Clone)]
struct SqlBindState {
    values: Vec<String>,
}

impl SqlBindState {
    fn push(&mut self, value: impl Into<String>) -> u32 {
        self.values.push(value.into());
        self.values.len() as u32
    }
}

struct PlannerCompileContext<'a> {
    bindings_by_name: &'a HashMap<String, PlannerBinding>,
    binding_aliases: &'a HashMap<String, String>,
    columns_by_table: &'a HashMap<(String, String), Vec<ColumnInfo>>,
}

fn quote_identifier(name: &str) -> String {
    format!(r#""{}""#, name.replace('"', "\"\""))
}

fn qualified_table_sql(table: &TableKey) -> String {
    format!(
        "{}.{}",
        quote_identifier(&table.schema),
        quote_identifier(&table.table)
    )
}

fn is_numeric_type(data_type: &str) -> bool {
    matches!(
        data_type,
        "smallint" | "integer" | "bigint" | "real" | "double precision" | "numeric"
    )
}

fn supports_min_max_type(data_type: &str) -> bool {
    !matches!(data_type, "json" | "jsonb" | "bytea" | "ARRAY")
}

fn is_temporal_type(data_type: &str) -> bool {
    matches!(
        data_type,
        "date"
            | "timestamp without time zone"
            | "timestamp with time zone"
            | "time without time zone"
            | "time with time zone"
    )
}

fn aggregate_result_data_type(source_data_type: &str, aggregate: PlannerAggregate) -> String {
    match aggregate {
        PlannerAggregate::Count => "bigint".to_string(),
        PlannerAggregate::Sum => match source_data_type {
            "smallint" | "integer" => "bigint".to_string(),
            "bigint" | "numeric" => "numeric".to_string(),
            "real" | "double precision" => "double precision".to_string(),
            other => other.to_string(),
        },
        PlannerAggregate::Avg => match source_data_type {
            "real" | "double precision" => "double precision".to_string(),
            "smallint" | "integer" | "bigint" | "numeric" => "numeric".to_string(),
            other => other.to_string(),
        },
        PlannerAggregate::Min | PlannerAggregate::Max | PlannerAggregate::Latest => {
            source_data_type.to_string()
        }
    }
}

fn numeric_difference_result_type(left_type: &str, right_type: &str) -> String {
    if left_type == "double precision"
        || right_type == "double precision"
        || left_type == "real"
        || right_type == "real"
    {
        "double precision".to_string()
    } else if left_type == "numeric"
        || right_type == "numeric"
        || left_type == "bigint"
        || right_type == "bigint"
    {
        "numeric".to_string()
    } else {
        "bigint".to_string()
    }
}

fn build_fk_adjacency(edges: &[FkEdge]) -> HashMap<TableKey, Vec<FkHop>> {
    let mut adjacency: HashMap<TableKey, Vec<FkHop>> = HashMap::new();

    for edge in edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(FkHop {
                from_schema: edge.source.schema.clone(),
                from_table: edge.source.table.clone(),
                from_columns: edge.source_columns.clone(),
                to_schema: edge.target.schema.clone(),
                to_table: edge.target.table.clone(),
                to_columns: edge.target_columns.clone(),
                constraint_name: edge.constraint_name.clone(),
            });

        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push(FkHop {
                from_schema: edge.target.schema.clone(),
                from_table: edge.target.table.clone(),
                from_columns: edge.target_columns.clone(),
                to_schema: edge.source.schema.clone(),
                to_table: edge.source.table.clone(),
                to_columns: edge.source_columns.clone(),
                constraint_name: edge.constraint_name.clone(),
            });
    }

    for hops in adjacency.values_mut() {
        hops.sort_by(|a, b| {
            (
                a.constraint_name.as_str(),
                a.from_schema.as_str(),
                a.from_table.as_str(),
                a.to_schema.as_str(),
                a.to_table.as_str(),
                a.from_columns.as_slice(),
                a.to_columns.as_slice(),
            )
                .cmp(&(
                    b.constraint_name.as_str(),
                    b.from_schema.as_str(),
                    b.from_table.as_str(),
                    b.to_schema.as_str(),
                    b.to_table.as_str(),
                    b.from_columns.as_slice(),
                    b.to_columns.as_slice(),
                ))
        });
    }

    adjacency
}

fn find_fk_path(edges: &[FkEdge], base: &TableKey, target: &TableKey) -> Vec<FkHop> {
    if base == target {
        return Vec::new();
    }

    let adjacency = build_fk_adjacency(edges);
    let mut visited: HashSet<TableKey> = HashSet::from([base.clone()]);
    let mut queue: VecDeque<(TableKey, Vec<FkHop>)> = VecDeque::from([(base.clone(), Vec::new())]);

    while let Some((current, path)) = queue.pop_front() {
        let Some(hops) = adjacency.get(&current) else {
            continue;
        };

        for hop in hops {
            let next = TableKey::new(&hop.to_schema, &hop.to_table);
            if visited.contains(&next) {
                continue;
            }

            let mut next_path = path.clone();
            next_path.push(hop.clone());

            if &next == target {
                return next_path;
            }

            visited.insert(next.clone());
            queue.push_back((next, next_path));
        }
    }

    Vec::new()
}

fn resolve_view_output_names(columns: &[ViewColumn]) -> anyhow::Result<Vec<String>> {
    let mut bare_name_counts: HashMap<&str, usize> = HashMap::new();
    for column in columns {
        if column.alias.is_none() && column.aggregate.is_none() {
            *bare_name_counts
                .entry(column.column_name.as_str())
                .or_insert(0) += 1;
        }
    }

    let mut seen = HashSet::new();
    let mut resolved = Vec::with_capacity(columns.len());

    for column in columns {
        let output_name = if let Some(alias) = &column.alias {
            alias.clone()
        } else if let Some(aggregate) = column.aggregate {
            format!(
                "{}_{}__{}",
                aggregate.as_sql().to_lowercase(),
                column.source_table,
                column.column_name
            )
        } else if bare_name_counts
            .get(column.column_name.as_str())
            .copied()
            .unwrap_or(0)
            > 1
        {
            format!("{}__{}", column.source_table, column.column_name)
        } else {
            column.column_name.clone()
        };

        if !is_valid_identifier(&output_name) {
            return Err(
                ValidationError(format!("Invalid output column name: {output_name}")).into(),
            );
        }
        if !seen.insert(output_name.clone()) {
            return Err(
                ValidationError(format!("Duplicate output column name: {output_name}")).into(),
            );
        }
        resolved.push(output_name);
    }

    Ok(resolved)
}

fn saved_view_aggregate_sql(aggregate: SavedViewAggregate) -> &'static str {
    match aggregate {
        SavedViewAggregate::Sum => "SUM",
        SavedViewAggregate::Avg => "AVG",
        SavedViewAggregate::Count => "COUNT",
        SavedViewAggregate::Min => "MIN",
        SavedViewAggregate::Max => "MAX",
        SavedViewAggregate::Latest => "LATEST",
    }
}

fn sanitize_generated_identifier(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ' ' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.trim().is_empty() {
        "source".to_string()
    } else {
        sanitized
    }
}

fn output_name_prefix_for_saved_column(column: &SavedViewColumn) -> String {
    column
        .source_id
        .as_deref()
        .map(sanitize_generated_identifier)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| column.source_table.clone())
}

fn resolve_saved_view_output_names(columns: &[SavedViewColumn]) -> anyhow::Result<Vec<String>> {
    let mut bare_name_counts: HashMap<&str, usize> = HashMap::new();
    for column in columns {
        let has_alias = column
            .alias
            .as_deref()
            .is_some_and(|alias| !alias.trim().is_empty());
        let is_derived =
            column.derived.is_some() || matches!(column.kind, Some(SavedViewColumnKind::Derived));
        if !has_alias && column.aggregate.is_none() && !is_derived {
            *bare_name_counts
                .entry(column.column_name.as_str())
                .or_insert(0) += 1;
        }
    }

    let mut seen = HashSet::new();
    let mut resolved = Vec::with_capacity(columns.len());

    for column in columns {
        let output_name = if let Some(alias) = column.alias.as_deref().map(str::trim)
            && !alias.is_empty()
        {
            alias.to_string()
        } else if let Some(aggregate) = column.aggregate {
            format!(
                "{}_{}__{}",
                saved_view_aggregate_sql(aggregate).to_lowercase(),
                output_name_prefix_for_saved_column(column),
                column.column_name
            )
        } else if column.derived.is_some()
            || matches!(column.kind, Some(SavedViewColumnKind::Derived))
        {
            format!("{}_derived", column.column_name)
        } else if bare_name_counts
            .get(column.column_name.as_str())
            .copied()
            .unwrap_or(0)
            > 1
        {
            format!(
                "{}__{}",
                output_name_prefix_for_saved_column(column),
                column.column_name
            )
        } else {
            column.column_name.clone()
        };

        if !is_valid_identifier(&output_name) {
            return Err(
                ValidationError(format!("Invalid output column name: {output_name}")).into(),
            );
        }
        if !seen.insert(output_name.clone()) {
            return Err(
                ValidationError(format!("Duplicate output column name: {output_name}")).into(),
            );
        }
        resolved.push(output_name);
    }

    Ok(resolved)
}

fn build_output_column_from_info(
    info: &ExpressionInfo,
    output_name: String,
    aggregate: Option<PlannerAggregate>,
) -> ColumnInfo {
    match aggregate {
        None | Some(PlannerAggregate::Latest) => info.to_output_column(output_name),
        Some(aggregate) => {
            let data_type = aggregate_result_data_type(&info.data_type, aggregate);
            ColumnInfo {
                name: output_name,
                display_type: humanize_type(&data_type, ""),
                data_type,
                is_nullable: !matches!(aggregate, PlannerAggregate::Count),
                is_primary_key: false,
            }
        }
    }
}

#[cfg(test)]
fn build_output_column(
    source: &ColumnInfo,
    output_name: String,
    aggregate: Option<ViewAggregate>,
) -> ColumnInfo {
    let planner_aggregate = aggregate
        .map(PlannerAggregate::from_view_aggregate)
        .transpose()
        .expect("public view aggregate should be supported by planner");
    build_output_column_from_info(
        &ExpressionInfo::from_column(source),
        output_name,
        planner_aggregate,
    )
}

/// Shared WHERE clause and bind values builder for query_rows and export.
struct QueryBuilder {
    where_clause: String,
    order_clause: String,
    bind_values: Vec<String>,
}

fn build_order_clause(
    columns: &[ColumnInfo],
    sort: &[SortEntry],
    include_pk_tiebreakers: bool,
) -> anyhow::Result<String> {
    if sort.is_empty() {
        return Ok(String::new());
    }

    let valid_column_names: std::collections::HashSet<&str> =
        columns.iter().map(|c| c.name.as_str()).collect();
    let mut order_parts: Vec<String> = Vec::new();
    let mut seen_columns: std::collections::HashSet<String> = std::collections::HashSet::new();

    for entry in sort {
        let col = entry.column.as_str();
        if !is_valid_identifier(col) {
            return Err(ValidationError(format!("Invalid sort column name: {col}")).into());
        }
        if !valid_column_names.contains(col) {
            return Err(ValidationError(format!("Unknown sort column: {col}")).into());
        }
        if !seen_columns.insert(entry.column.clone()) {
            return Err(ValidationError(format!("Duplicate sort column: {col}")).into());
        }
        let direction = match entry.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        order_parts.push(format!("\"{}\" {direction}", entry.column));
    }

    if include_pk_tiebreakers {
        for col in columns.iter().filter(|c| c.is_primary_key) {
            if seen_columns.insert(col.name.clone()) {
                order_parts.push(format!("\"{}\" ASC", col.name));
            }
        }
    }

    Ok(if order_parts.is_empty() {
        String::new()
    } else {
        format!("ORDER BY {}", order_parts.join(", "))
    })
}

fn build_query_clauses(
    columns: &[ColumnInfo],
    search: Option<&str>,
    filters: &std::collections::HashMap<String, String>,
    sort: &[SortEntry],
    include_pk_tiebreakers: bool,
    start_param_idx: u32,
) -> anyhow::Result<QueryBuilder> {
    // Validate filter column names against the actual schema
    let valid_column_names: std::collections::HashSet<&str> =
        columns.iter().map(|c| c.name.as_str()).collect();

    for col_name in filters.keys() {
        if !is_valid_identifier(col_name) {
            return Err(ValidationError(format!("Invalid filter column name: {col_name}")).into());
        }
        if !valid_column_names.contains(col_name.as_str()) {
            return Err(ValidationError(format!("Unknown filter column: {col_name}")).into());
        }
    }

    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();
    let mut param_idx: u32 = start_param_idx;

    // Search condition (searches across all text columns)
    let has_search = search.is_some_and(|s| !s.is_empty());
    if has_search {
        let text_cols: Vec<String> = columns
            .iter()
            .filter(|c| is_text_type(&c.data_type) && is_valid_identifier(&c.name))
            .map(|c| format!("\"{}\"::text ILIKE '%' || ${param_idx} || '%'", c.name))
            .collect();

        if !text_cols.is_empty() {
            conditions.push(format!("({})", text_cols.join(" OR ")));
            bind_values.push(search.unwrap().to_string());
            param_idx += 1;
        }
    }

    // Per-column filter conditions (AND-ed)
    // Boolean columns use = TRUE/FALSE instead of ILIKE to match Yes/No display.
    let column_types: std::collections::HashMap<&str, &str> = columns
        .iter()
        .map(|c| (c.name.as_str(), c.data_type.as_str()))
        .collect();

    let mut filter_entries: Vec<_> = filters.iter().collect();
    filter_entries.sort_by_key(|(k, _)| k.as_str());

    for (col_name, value) in &filter_entries {
        let col_type = column_types.get(col_name.as_str()).copied().unwrap_or("");
        if col_type == "boolean" {
            // Map user-friendly input to SQL boolean
            let normalized = value.trim().to_lowercase();
            let bool_val = match normalized.as_str() {
                "yes" | "true" | "t" | "1" => Some(true),
                "no" | "false" | "f" | "0" => Some(false),
                _ => None,
            };
            if let Some(b) = bool_val {
                conditions.push(format!(
                    "\"{}\" = {}",
                    col_name,
                    if b { "TRUE" } else { "FALSE" }
                ));
            } else {
                // Non-boolean input on a boolean column → no rows match
                conditions.push("FALSE".to_string());
            }
        } else {
            conditions.push(format!("\"{}\"::text ILIKE ${param_idx}", col_name));
            bind_values.push(format!("%{value}%"));
            param_idx += 1;
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let order_clause = build_order_clause(columns, sort, include_pk_tiebreakers)?;

    Ok(QueryBuilder {
        where_clause,
        order_clause,
        bind_values,
    })
}

async fn load_fk_edges(pool: &PgPool, schema: &str) -> anyhow::Result<Vec<FkEdge>> {
    if !is_valid_identifier(schema) {
        anyhow::bail!("Invalid schema name: {schema}");
    }

    let rows = sqlx::query(
        r#"
        SELECT
            src_ns.nspname AS source_schema,
            src.relname AS source_table,
            ARRAY_AGG(src_attr.attname ORDER BY ord.ord) AS source_columns,
            dst_ns.nspname AS target_schema,
            dst.relname AS target_table,
            ARRAY_AGG(dst_attr.attname ORDER BY ord.ord) AS target_columns,
            c.conname AS constraint_name
        FROM pg_constraint c
        JOIN pg_class src ON src.oid = c.conrelid
        JOIN pg_namespace src_ns ON src_ns.oid = src.relnamespace
        JOIN pg_class dst ON dst.oid = c.confrelid
        JOIN pg_namespace dst_ns ON dst_ns.oid = dst.relnamespace
        JOIN LATERAL generate_subscripts(c.conkey, 1) AS ord(ord) ON true
        JOIN pg_attribute src_attr
            ON src_attr.attrelid = src.oid
           AND src_attr.attnum = c.conkey[ord.ord]
        JOIN pg_attribute dst_attr
            ON dst_attr.attrelid = dst.oid
           AND dst_attr.attnum = c.confkey[ord.ord]
        WHERE c.contype = 'f'
          AND src_ns.nspname = $1
          AND dst_ns.nspname = $1
        GROUP BY
            src_ns.nspname,
            src.relname,
            dst_ns.nspname,
            dst.relname,
            c.conname
        ORDER BY c.conname, src.relname, dst.relname
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| FkEdge {
            source: TableKey::new(
                row.get::<String, _>("source_schema").as_str(),
                row.get::<String, _>("source_table").as_str(),
            ),
            target: TableKey::new(
                row.get::<String, _>("target_schema").as_str(),
                row.get::<String, _>("target_table").as_str(),
            ),
            source_columns: row.try_get("source_columns").unwrap_or_default(),
            target_columns: row.try_get("target_columns").unwrap_or_default(),
            constraint_name: row.get("constraint_name"),
        })
        .collect())
}

pub async fn lookup_fk_path(
    pool: &PgPool,
    base_schema: &str,
    base_table: &str,
    target_schema: &str,
    target_table: &str,
) -> anyhow::Result<Vec<FkHop>> {
    if !is_valid_identifier(base_schema) || !is_valid_identifier(target_schema) {
        anyhow::bail!("Invalid schema name");
    }
    if !is_valid_identifier(base_table) || !is_valid_identifier(target_table) {
        anyhow::bail!("Invalid table name");
    }
    if base_schema != target_schema {
        return Ok(Vec::new());
    }

    let base = TableKey::new(base_schema, base_table);
    let target = TableKey::new(target_schema, target_table);
    let edges = load_fk_edges(pool, base_schema).await?;
    Ok(find_fk_path(&edges, &base, &target))
}

fn normalize_boolean_filter(value: &str) -> Option<bool> {
    match value.trim().to_lowercase().as_str() {
        "yes" | "true" | "t" | "1" => Some(true),
        "no" | "false" | "f" | "0" => Some(false),
        _ => None,
    }
}

fn sql_cast_type(data_type: &str) -> Option<&'static str> {
    match data_type {
        "smallint" => Some("smallint"),
        "integer" => Some("integer"),
        "bigint" => Some("bigint"),
        "real" => Some("real"),
        "double precision" => Some("double precision"),
        "numeric" => Some("numeric"),
        "boolean" => Some("boolean"),
        "uuid" => Some("uuid"),
        "date" => Some("date"),
        "time without time zone" => Some("time"),
        "time with time zone" => Some("timetz"),
        "timestamp without time zone" => Some("timestamp"),
        "timestamp with time zone" => Some("timestamptz"),
        "character varying" | "character" | "text" => Some("text"),
        "interval" => Some("interval"),
        _ => None,
    }
}

fn bind_typed_value_sql(
    bind_state: &mut SqlBindState,
    value: String,
    data_type: &str,
) -> anyhow::Result<String> {
    let idx = bind_state.push(value);
    let Some(cast_type) = sql_cast_type(data_type) else {
        return Err(ValidationError(format!(
            "Structured filters are not supported for {data_type} columns"
        ))
        .into());
    };
    Ok(format!("${idx}::{cast_type}"))
}

fn build_bindings_by_name(
    draft: &PlannerDraftDef,
) -> anyhow::Result<HashMap<String, PlannerBinding>> {
    let mut bindings_by_name = HashMap::new();

    for binding in &draft.bindings {
        if !is_valid_identifier(&binding.name) {
            return Err(ValidationError(format!(
                "Invalid planner source binding name: {}",
                binding.name
            ))
            .into());
        }
        if !is_valid_identifier(&binding.table.schema) {
            return Err(
                ValidationError(format!("Invalid schema name: {}", binding.table.schema)).into(),
            );
        }
        if !is_valid_identifier(&binding.table.table) {
            return Err(
                ValidationError(format!("Invalid table name: {}", binding.table.table)).into(),
            );
        }
        if let Some(existing) = bindings_by_name.get(&binding.name) {
            if existing != binding {
                return Err(ValidationError(format!(
                    "Duplicate planner source binding with different table: {}",
                    binding.name
                ))
                .into());
            }
        } else {
            bindings_by_name.insert(binding.name.clone(), binding.clone());
        }
    }

    for join in &draft.joins {
        if let Some(existing) = bindings_by_name.get(&join.target_binding.name) {
            if existing != &join.target_binding {
                return Err(ValidationError(format!(
                    "Duplicate planner source binding with different table: {}",
                    join.target_binding.name
                ))
                .into());
            }
        } else {
            bindings_by_name.insert(
                join.target_binding.name.clone(),
                join.target_binding.clone(),
            );
        }
    }

    if !bindings_by_name.contains_key(&draft.base_binding) {
        return Err(ValidationError(format!(
            "Unknown base planner binding: {}",
            draft.base_binding
        ))
        .into());
    }

    Ok(bindings_by_name)
}

fn collect_condition_binding_names(condition: &PlannerCondition, names: &mut HashSet<String>) {
    match condition {
        PlannerCondition::IsEmpty(expr) => collect_expr_binding_names(expr, names),
        PlannerCondition::Compare { left, right, .. } => {
            collect_expr_binding_names(left, names);
            collect_expr_binding_names(right, names);
        }
    }
}

fn collect_expr_binding_names(expr: &PlannerValueExpr, names: &mut HashSet<String>) {
    match expr {
        PlannerValueExpr::Field(field) => {
            names.insert(field.binding.clone());
        }
        PlannerValueExpr::Literal(_) => {}
        PlannerValueExpr::Difference { left, right }
        | PlannerValueExpr::RatioPercent {
            numerator: left,
            denominator: right,
        } => {
            collect_expr_binding_names(left, names);
            collect_expr_binding_names(right, names);
        }
        PlannerValueExpr::AgeOfTimestamp { value }
        | PlannerValueExpr::TextLength { value }
        | PlannerValueExpr::DateBucket { value, .. }
        | PlannerValueExpr::DatePart { value, .. } => collect_expr_binding_names(value, names),
        PlannerValueExpr::TextConcat { parts } => {
            for part in parts {
                collect_expr_binding_names(part, names);
            }
        }
        PlannerValueExpr::IfThen {
            condition,
            then_expr,
            else_expr,
        } => {
            collect_condition_binding_names(condition, names);
            collect_expr_binding_names(then_expr, names);
            collect_expr_binding_names(else_expr, names);
        }
    }
}

fn collect_referenced_binding_names(draft: &PlannerDraftDef) -> HashSet<String> {
    let mut names = HashSet::from([draft.base_binding.clone()]);

    for projection in &draft.projections {
        collect_expr_binding_names(&projection.expr, &mut names);
    }
    for expr in &draft.group_by {
        collect_expr_binding_names(expr, &mut names);
    }
    if let Some(latest) = &draft.latest_per_group {
        for expr in &latest.partition_by {
            collect_expr_binding_names(expr, &mut names);
        }
        collect_expr_binding_names(&latest.order_by.expr, &mut names);
    }
    if let Some(ranking) = &draft.rank_limit {
        for expr in &ranking.partition_by {
            collect_expr_binding_names(expr, &mut names);
        }
        collect_expr_binding_names(&ranking.order_by.expr, &mut names);
    }
    for join in &draft.joins {
        names.insert(join.source_binding.clone());
        names.insert(join.target_binding.name.clone());
    }

    names
}

fn compile_value_expr(
    expr: &PlannerValueExpr,
    ctx: &PlannerCompileContext<'_>,
    bind_state: &mut SqlBindState,
) -> anyhow::Result<CompiledExpr> {
    match expr {
        PlannerValueExpr::Field(field) => {
            if !is_valid_identifier(&field.binding) {
                return Err(ValidationError(format!(
                    "Invalid planner binding name: {}",
                    field.binding
                ))
                .into());
            }
            if !is_valid_identifier(&field.column) {
                return Err(
                    ValidationError(format!("Invalid column name: {}", field.column)).into(),
                );
            }

            let binding = ctx.bindings_by_name.get(&field.binding).ok_or_else(|| {
                ValidationError(format!("Unknown planner binding: {}", field.binding))
            })?;
            let alias = ctx.binding_aliases.get(&field.binding).ok_or_else(|| {
                ValidationError(format!(
                    "Planner binding is not joined into the query: {}",
                    field.binding
                ))
            })?;
            let columns = ctx
                .columns_by_table
                .get(&(binding.table.schema.clone(), binding.table.table.clone()))
                .ok_or_else(|| {
                    ValidationError(format!(
                        "Unknown planner table: {}.{}",
                        binding.table.schema, binding.table.table
                    ))
                })?;
            let column = columns
                .iter()
                .find(|candidate| candidate.name == field.column)
                .ok_or_else(|| {
                    ValidationError(format!(
                        "Unknown planner column: {}.{}.{}",
                        binding.table.schema, binding.table.table, field.column
                    ))
                })?;

            Ok(CompiledExpr {
                sql: format!(r#"{alias}.{}"#, quote_identifier(&column.name)),
                info: ExpressionInfo::from_column(column),
            })
        }
        PlannerValueExpr::Literal(literal) => match literal {
            PlannerLiteral::String(value) => {
                let idx = bind_state.push(value.clone());
                Ok(CompiledExpr {
                    sql: format!("${idx}"),
                    info: ExpressionInfo {
                        data_type: "text".to_string(),
                        display_type: "Text".to_string(),
                        is_nullable: false,
                        is_primary_key: false,
                    },
                })
            }
            PlannerLiteral::Number(value) => Ok(CompiledExpr {
                sql: bind_typed_value_sql(bind_state, value.clone(), "numeric")?,
                info: ExpressionInfo {
                    data_type: "numeric".to_string(),
                    display_type: "Decimal".to_string(),
                    is_nullable: false,
                    is_primary_key: false,
                },
            }),
            PlannerLiteral::Boolean(value) => Ok(CompiledExpr {
                sql: bind_typed_value_sql(bind_state, value.to_string(), "boolean")?,
                info: ExpressionInfo {
                    data_type: "boolean".to_string(),
                    display_type: "Yes/No".to_string(),
                    is_nullable: false,
                    is_primary_key: false,
                },
            }),
        },
        PlannerValueExpr::Difference { left, right } => {
            let left = compile_value_expr(left, ctx, bind_state)?;
            let right = compile_value_expr(right, ctx, bind_state)?;
            if !is_numeric_type(&left.info.data_type) || !is_numeric_type(&right.info.data_type) {
                return Err(ValidationError(
                    "difference is only supported for numeric expressions".into(),
                )
                .into());
            }
            let data_type =
                numeric_difference_result_type(&left.info.data_type, &right.info.data_type);
            Ok(CompiledExpr {
                sql: format!("({}) - ({})", left.sql, right.sql),
                info: ExpressionInfo {
                    data_type: data_type.clone(),
                    display_type: humanize_type(&data_type, ""),
                    is_nullable: left.info.is_nullable || right.info.is_nullable,
                    is_primary_key: false,
                },
            })
        }
        PlannerValueExpr::RatioPercent {
            numerator,
            denominator,
        } => {
            let numerator = compile_value_expr(numerator, ctx, bind_state)?;
            let denominator = compile_value_expr(denominator, ctx, bind_state)?;
            if !is_numeric_type(&numerator.info.data_type)
                || !is_numeric_type(&denominator.info.data_type)
            {
                return Err(ValidationError(
                    "ratio percent is only supported for numeric expressions".into(),
                )
                .into());
            }
            Ok(CompiledExpr {
                sql: format!(
                    "CASE WHEN ({den}) IS NULL OR ({den}) = 0 THEN NULL ELSE (({num})::numeric / NULLIF(({den})::numeric, 0)) * 100 END",
                    den = denominator.sql,
                    num = numerator.sql
                ),
                info: ExpressionInfo {
                    data_type: "numeric".to_string(),
                    display_type: "Decimal".to_string(),
                    is_nullable: true,
                    is_primary_key: false,
                },
            })
        }
        PlannerValueExpr::AgeOfTimestamp { value } => {
            let value = compile_value_expr(value, ctx, bind_state)?;
            if !is_temporal_type(&value.info.data_type) {
                return Err(ValidationError(
                    "age of timestamp is only supported for temporal expressions".into(),
                )
                .into());
            }
            Ok(CompiledExpr {
                sql: format!("AGE(CURRENT_TIMESTAMP, ({})::timestamptz)", value.sql),
                info: ExpressionInfo {
                    data_type: "interval".to_string(),
                    display_type: "Duration".to_string(),
                    is_nullable: true,
                    is_primary_key: false,
                },
            })
        }
        PlannerValueExpr::DateBucket { value, bucket } => {
            let value = compile_value_expr(value, ctx, bind_state)?;
            if !is_temporal_type(&value.info.data_type) {
                return Err(ValidationError(
                    "date bucket is only supported for temporal expressions".into(),
                )
                .into());
            }
            let bucket_sql = match bucket {
                PlannerDateBucket::Day => "day",
                PlannerDateBucket::Week => "week",
            };
            Ok(CompiledExpr {
                sql: format!(
                    "DATE_TRUNC('{bucket_sql}', ({})::timestamptz)::date",
                    value.sql
                ),
                info: ExpressionInfo {
                    data_type: "date".to_string(),
                    display_type: "Date".to_string(),
                    is_nullable: value.info.is_nullable,
                    is_primary_key: false,
                },
            })
        }
        PlannerValueExpr::DatePart { value, part } => {
            let value = compile_value_expr(value, ctx, bind_state)?;
            if !is_temporal_type(&value.info.data_type) {
                return Err(ValidationError(
                    "date part is only supported for temporal expressions".into(),
                )
                .into());
            }
            let (sql, data_type, display_type) = match part {
                PlannerDatePart::Year => (
                    format!("EXTRACT(YEAR FROM ({})::timestamptz)::integer", value.sql),
                    "integer".to_string(),
                    "Number".to_string(),
                ),
                PlannerDatePart::Month => (
                    format!("EXTRACT(MONTH FROM ({})::timestamptz)::integer", value.sql),
                    "integer".to_string(),
                    "Number".to_string(),
                ),
                PlannerDatePart::Weekday => (
                    format!("TRIM(TO_CHAR(({})::timestamptz, 'Day'))", value.sql),
                    "text".to_string(),
                    "Text".to_string(),
                ),
            };
            Ok(CompiledExpr {
                sql,
                info: ExpressionInfo {
                    data_type,
                    display_type,
                    is_nullable: value.info.is_nullable,
                    is_primary_key: false,
                },
            })
        }
        PlannerValueExpr::TextConcat { parts } => {
            if parts.is_empty() {
                return Err(ValidationError(
                    "text concat requires at least one input expression".into(),
                )
                .into());
            }
            let mut compiled_parts = Vec::with_capacity(parts.len());
            let mut nullable = false;
            for part in parts {
                let compiled = compile_value_expr(part, ctx, bind_state)?;
                nullable |= compiled.info.is_nullable;
                compiled_parts.push(format!("COALESCE(({})::text, '')", compiled.sql));
            }
            Ok(CompiledExpr {
                sql: compiled_parts.join(" || "),
                info: ExpressionInfo {
                    data_type: "text".to_string(),
                    display_type: "Text".to_string(),
                    is_nullable: nullable,
                    is_primary_key: false,
                },
            })
        }
        PlannerValueExpr::TextLength { value } => {
            let value = compile_value_expr(value, ctx, bind_state)?;
            Ok(CompiledExpr {
                sql: format!("CHAR_LENGTH(COALESCE(({})::text, ''))", value.sql),
                info: ExpressionInfo {
                    data_type: "integer".to_string(),
                    display_type: "Number".to_string(),
                    is_nullable: false,
                    is_primary_key: false,
                },
            })
        }
        PlannerValueExpr::IfThen {
            condition,
            then_expr,
            else_expr,
        } => {
            let condition_sql = compile_condition(condition, ctx, bind_state)?;
            let then_expr = compile_value_expr(then_expr, ctx, bind_state)?;
            let else_expr = compile_value_expr(else_expr, ctx, bind_state)?;
            if then_expr.info.data_type == else_expr.info.data_type {
                Ok(CompiledExpr {
                    sql: format!(
                        "CASE WHEN {condition_sql} THEN {} ELSE {} END",
                        then_expr.sql, else_expr.sql
                    ),
                    info: ExpressionInfo {
                        data_type: then_expr.info.data_type.clone(),
                        display_type: then_expr.info.display_type.clone(),
                        is_nullable: then_expr.info.is_nullable || else_expr.info.is_nullable,
                        is_primary_key: false,
                    },
                })
            } else {
                Ok(CompiledExpr {
                    sql: format!(
                        "CASE WHEN {condition_sql} THEN ({})::text ELSE ({})::text END",
                        then_expr.sql, else_expr.sql
                    ),
                    info: ExpressionInfo {
                        data_type: "text".to_string(),
                        display_type: "Text".to_string(),
                        is_nullable: then_expr.info.is_nullable || else_expr.info.is_nullable,
                        is_primary_key: false,
                    },
                })
            }
        }
    }
}

fn compile_condition(
    condition: &PlannerCondition,
    ctx: &PlannerCompileContext<'_>,
    bind_state: &mut SqlBindState,
) -> anyhow::Result<String> {
    match condition {
        PlannerCondition::IsEmpty(expr) => {
            let expr = compile_value_expr(expr, ctx, bind_state)?;
            Ok(format!(
                "({sql}) IS NULL OR BTRIM(({sql})::text) = ''",
                sql = expr.sql
            ))
        }
        PlannerCondition::Compare { left, op, right } => {
            let left = compile_value_expr(left, ctx, bind_state)?;
            let right = compile_value_expr(right, ctx, bind_state)?;
            Ok(format!("({}) {} ({})", left.sql, op.as_sql(), right.sql))
        }
    }
}

fn build_projection_defs(
    draft: &PlannerDraftDef,
    ctx: &PlannerCompileContext<'_>,
    bind_state: &mut SqlBindState,
) -> anyhow::Result<Vec<PlannedProjectionDef>> {
    if draft.projections.is_empty() {
        return Err(ValidationError("Saved views must include at least one column".into()).into());
    }

    let mut seen = HashSet::new();
    let mut defs = Vec::with_capacity(draft.projections.len());

    for projection in &draft.projections {
        if !is_valid_identifier(&projection.output_name) {
            return Err(ValidationError(format!(
                "Invalid output column name: {}",
                projection.output_name
            ))
            .into());
        }
        if !seen.insert(projection.output_name.clone()) {
            return Err(ValidationError(format!(
                "Duplicate output column name: {}",
                projection.output_name
            ))
            .into());
        }

        let compiled = compile_value_expr(&projection.expr, ctx, bind_state)?;
        if let Some(aggregate) = projection.aggregate {
            if matches!(aggregate, PlannerAggregate::Sum | PlannerAggregate::Avg)
                && !is_numeric_type(&compiled.info.data_type)
            {
                return Err(ValidationError(format!(
                    "{} is only supported for numeric columns",
                    aggregate.as_sql()
                ))
                .into());
            }
            if matches!(aggregate, PlannerAggregate::Min | PlannerAggregate::Max)
                && !supports_min_max_type(&compiled.info.data_type)
            {
                return Err(ValidationError(format!(
                    "{} is not supported for {} columns",
                    aggregate.as_sql(),
                    compiled.info.data_type
                ))
                .into());
            }
        }

        defs.push(PlannedProjectionDef {
            output_name: projection.output_name.clone(),
            expr_sql: compiled.sql,
            info: compiled.info,
            aggregate: projection.aggregate,
        });
    }

    Ok(defs)
}

fn build_definition_filters(
    filter_targets: &[DefinitionFilterTarget],
    filters: &[PlannerFilter],
    bind_state: &mut SqlBindState,
) -> anyhow::Result<Vec<String>> {
    let mut targets_by_output: HashMap<&str, &DefinitionFilterTarget> = HashMap::new();
    for target in filter_targets {
        targets_by_output.insert(target.output_name.as_str(), target);
    }

    let mut entries: Vec<_> = filters.iter().collect();
    entries.sort_by_key(|filter| filter.output_name.as_str());

    let mut conditions = Vec::with_capacity(entries.len());

    for filter in entries {
        if !is_valid_identifier(&filter.output_name) {
            return Err(ValidationError(format!(
                "Invalid filter column name: {}",
                filter.output_name
            ))
            .into());
        }
        let Some(target) = targets_by_output.get(filter.output_name.as_str()) else {
            return Err(
                ValidationError(format!("Unknown filter column: {}", filter.output_name)).into(),
            );
        };
        if target.aggregate.is_some() {
            return Err(ValidationError(format!(
                "Aggregate columns cannot be used in saved-view definition filters: {}",
                filter.output_name
            ))
            .into());
        }

        let column_expr = target.sql_expr.as_str();
        let condition = match &filter.value {
            PlannerFilterValue::Eq(value) => {
                if target.info.data_type == "boolean" {
                    match normalize_boolean_filter(value) {
                        Some(boolean) => format!(
                            "{column_expr} = {}",
                            bind_typed_value_sql(bind_state, boolean.to_string(), "boolean")?
                        ),
                        None => "FALSE".to_string(),
                    }
                } else {
                    format!(
                        "{column_expr} = {}",
                        bind_typed_value_sql(
                            bind_state,
                            value.clone(),
                            target.info.data_type.as_str()
                        )?
                    )
                }
            }
            PlannerFilterValue::Gt(value) => format!(
                "{column_expr} > {}",
                bind_typed_value_sql(bind_state, value.clone(), target.info.data_type.as_str())?
            ),
            PlannerFilterValue::Gte(value) => format!(
                "{column_expr} >= {}",
                bind_typed_value_sql(bind_state, value.clone(), target.info.data_type.as_str())?
            ),
            PlannerFilterValue::Lt(value) => format!(
                "{column_expr} < {}",
                bind_typed_value_sql(bind_state, value.clone(), target.info.data_type.as_str())?
            ),
            PlannerFilterValue::Lte(value) => format!(
                "{column_expr} <= {}",
                bind_typed_value_sql(bind_state, value.clone(), target.info.data_type.as_str())?
            ),
            PlannerFilterValue::Contains(value) => {
                let idx = bind_state.push(value.clone());
                format!("{column_expr}::text ILIKE '%' || ${idx} || '%'")
            }
            PlannerFilterValue::StartsWith(value) => {
                let idx = bind_state.push(value.clone());
                format!("{column_expr}::text ILIKE ${idx} || '%'")
            }
            PlannerFilterValue::Between(start, end) => {
                let start_sql = bind_typed_value_sql(
                    bind_state,
                    start.clone(),
                    target.info.data_type.as_str(),
                )?;
                let end_sql =
                    bind_typed_value_sql(bind_state, end.clone(), target.info.data_type.as_str())?;
                format!("{column_expr} BETWEEN {start_sql} AND {end_sql}")
            }
            PlannerFilterValue::IsEmpty => {
                format!("({column_expr}) IS NULL OR BTRIM(({column_expr})::text) = ''")
            }
            PlannerFilterValue::InList(values) => {
                if values.is_empty() {
                    "FALSE".to_string()
                } else if target.info.data_type == "boolean" {
                    let mut placeholders = Vec::new();
                    for value in values {
                        let Some(boolean) = normalize_boolean_filter(value) else {
                            return Ok(vec!["FALSE".to_string()]);
                        };
                        placeholders.push(bind_typed_value_sql(
                            bind_state,
                            boolean.to_string(),
                            "boolean",
                        )?);
                    }
                    format!("{column_expr} IN ({})", placeholders.join(", "))
                } else {
                    let mut placeholders = Vec::with_capacity(values.len());
                    for value in values {
                        placeholders.push(bind_typed_value_sql(
                            bind_state,
                            value.clone(),
                            target.info.data_type.as_str(),
                        )?);
                    }
                    format!("{column_expr} IN ({})", placeholders.join(", "))
                }
            }
        };
        conditions.push(condition);
    }

    Ok(conditions)
}

fn build_binding_aliases_and_joins(
    draft: &PlannerDraftDef,
    bindings_by_name: &HashMap<String, PlannerBinding>,
    catalog: &PlannerCatalog,
    referenced_bindings: &HashSet<String>,
) -> anyhow::Result<(HashMap<String, String>, Vec<String>)> {
    let base_binding = bindings_by_name.get(&draft.base_binding).ok_or_else(|| {
        ValidationError(format!(
            "Unknown base planner binding: {}",
            draft.base_binding
        ))
    })?;

    let explicit_targets: HashSet<&str> = draft
        .joins
        .iter()
        .map(|join| join.target_binding.name.as_str())
        .collect();

    let mut binding_aliases = HashMap::from([(draft.base_binding.clone(), "t0".to_string())]);
    let mut physical_aliases = HashMap::from([(base_binding.table.clone(), "t0".to_string())]);
    let mut join_clauses = Vec::new();
    let mut next_alias_index = 1usize;

    let mut implicit_bindings: Vec<_> = bindings_by_name
        .values()
        .filter(|binding| {
            binding.name != draft.base_binding
                && referenced_bindings.contains(&binding.name)
                && !explicit_targets.contains(binding.name.as_str())
        })
        .cloned()
        .collect();
    implicit_bindings.sort_by(|a, b| a.name.cmp(&b.name));

    for binding in implicit_bindings {
        if binding.table == base_binding.table {
            return Err(ValidationError(format!(
                "Self-joins require explicit sources metadata: {}",
                binding.name
            ))
            .into());
        }
        let path = find_fk_path(&catalog.fk_edges, &base_binding.table, &binding.table);
        if path.is_empty() {
            return Err(ValidationError(format!(
                "No foreign-key path found from {}.{} to {}.{}",
                base_binding.table.schema,
                base_binding.table.table,
                binding.table.schema,
                binding.table.table
            ))
            .into());
        }

        for hop in path {
            let from = TableKey::new(&hop.from_schema, &hop.from_table);
            let to = TableKey::new(&hop.to_schema, &hop.to_table);
            if !physical_aliases.contains_key(&to) {
                let from_alias = physical_aliases
                    .get(&from)
                    .expect("foreign-key join path should build from a known alias")
                    .clone();
                let to_alias = format!("t{next_alias_index}");
                next_alias_index += 1;
                let conditions = hop
                    .from_columns
                    .iter()
                    .zip(&hop.to_columns)
                    .map(|(from_col, to_col)| {
                        format!(
                            r#"{from_alias}.{} = {to_alias}.{}"#,
                            quote_identifier(from_col),
                            quote_identifier(to_col)
                        )
                    })
                    .collect::<Vec<_>>();

                join_clauses.push(format!(
                    "LEFT JOIN {} {to_alias} ON {}",
                    qualified_table_sql(&to),
                    conditions.join(" AND ")
                ));
                physical_aliases.insert(to.clone(), to_alias);
            }
        }

        let target_alias = physical_aliases
            .get(&binding.table)
            .expect("foreign-key join should end with target alias")
            .clone();
        binding_aliases.insert(binding.name.clone(), target_alias);
    }

    let mut pending: Vec<_> = draft
        .joins
        .iter()
        .filter(|join| referenced_bindings.contains(&join.target_binding.name))
        .cloned()
        .collect();

    while !pending.is_empty() {
        let pending_len = pending.len();
        let mut index = 0usize;

        while index < pending.len() {
            let join = &pending[index];
            if !binding_aliases.contains_key(&join.source_binding) {
                index += 1;
                continue;
            }
            if binding_aliases.contains_key(&join.target_binding.name) {
                return Err(ValidationError(format!(
                    "Planner join target is already available: {}",
                    join.target_binding.name
                ))
                .into());
            }

            let source_binding = bindings_by_name.get(&join.source_binding).ok_or_else(|| {
                ValidationError(format!(
                    "Unknown source binding in planner join: {}",
                    join.source_binding
                ))
            })?;
            let source_alias = binding_aliases
                .get(&join.source_binding)
                .expect("source binding should already have an alias")
                .clone();
            let target_alias = format!("t{next_alias_index}");
            next_alias_index += 1;
            let target_columns = catalog
                .columns_by_table
                .get(&(
                    join.target_binding.table.schema.clone(),
                    join.target_binding.table.table.clone(),
                ))
                .ok_or_else(|| {
                    ValidationError(format!(
                        "Unknown planner table: {}.{}",
                        join.target_binding.table.schema, join.target_binding.table.table
                    ))
                })?;
            let source_columns = catalog
                .columns_by_table
                .get(&(
                    source_binding.table.schema.clone(),
                    source_binding.table.table.clone(),
                ))
                .ok_or_else(|| {
                    ValidationError(format!(
                        "Unknown planner table: {}.{}",
                        source_binding.table.schema, source_binding.table.table
                    ))
                })?;

            let join_sql = match &join.kind {
                PlannerJoinKind::MatchColumns { pairs } => {
                    if pairs.is_empty() {
                        return Err(ValidationError(
                            "Custom joins require at least one match column".into(),
                        )
                        .into());
                    }
                    let mut conditions = Vec::with_capacity(pairs.len());
                    for pair in pairs {
                        if !source_columns
                            .iter()
                            .any(|column| column.name == pair.source_column)
                        {
                            return Err(ValidationError(format!(
                                "Unknown planner column: {}.{}.{}",
                                source_binding.table.schema,
                                source_binding.table.table,
                                pair.source_column
                            ))
                            .into());
                        }
                        if !target_columns
                            .iter()
                            .any(|column| column.name == pair.target_column)
                        {
                            return Err(ValidationError(format!(
                                "Unknown planner column: {}.{}.{}",
                                join.target_binding.table.schema,
                                join.target_binding.table.table,
                                pair.target_column
                            ))
                            .into());
                        }
                        conditions.push(format!(
                            r#"{source_alias}.{} = {target_alias}.{}"#,
                            quote_identifier(&pair.source_column),
                            quote_identifier(&pair.target_column)
                        ));
                    }
                    format!(
                        "LEFT JOIN {} {target_alias} ON {}",
                        qualified_table_sql(&join.target_binding.table),
                        conditions.join(" AND ")
                    )
                }
                PlannerJoinKind::PreviousRow {
                    entity_pairs,
                    order_by_column,
                    direction,
                } => {
                    if source_binding.table != join.target_binding.table {
                        return Err(ValidationError(format!(
                            "Previous-row joins must target the same table as their source binding: {}",
                            join.target_binding.name
                        ))
                        .into());
                    }
                    if entity_pairs.is_empty() {
                        return Err(ValidationError(
                            "Previous-row joins require at least one entity match column".into(),
                        )
                        .into());
                    }
                    if !source_columns
                        .iter()
                        .any(|column| column.name == *order_by_column)
                    {
                        return Err(ValidationError(format!(
                            "Unknown planner column: {}.{}.{}",
                            source_binding.table.schema,
                            source_binding.table.table,
                            order_by_column
                        ))
                        .into());
                    }

                    let inner_alias = format!("{target_alias}_src");
                    let mut conditions = Vec::with_capacity(entity_pairs.len() + 1);
                    for pair in entity_pairs {
                        if !source_columns
                            .iter()
                            .any(|column| column.name == pair.source_column)
                        {
                            return Err(ValidationError(format!(
                                "Unknown planner column: {}.{}.{}",
                                source_binding.table.schema,
                                source_binding.table.table,
                                pair.source_column
                            ))
                            .into());
                        }
                        if !target_columns
                            .iter()
                            .any(|column| column.name == pair.target_column)
                        {
                            return Err(ValidationError(format!(
                                "Unknown planner column: {}.{}.{}",
                                join.target_binding.table.schema,
                                join.target_binding.table.table,
                                pair.target_column
                            ))
                            .into());
                        }
                        conditions.push(format!(
                            r#"{inner_alias}.{} = {source_alias}.{}"#,
                            quote_identifier(&pair.target_column),
                            quote_identifier(&pair.source_column)
                        ));
                    }

                    let (operator, direction_sql) = match direction {
                        PlannerNeighborDirection::Previous => ("<", "DESC"),
                        PlannerNeighborDirection::Next => (">", "ASC"),
                    };
                    conditions.push(format!(
                        r#"{inner_alias}.{} {operator} {source_alias}.{}"#,
                        quote_identifier(order_by_column),
                        quote_identifier(order_by_column)
                    ));

                    let mut order_parts = vec![format!(
                        r#"{inner_alias}.{} {direction_sql}"#,
                        quote_identifier(order_by_column)
                    )];
                    for pk in target_columns.iter().filter(|column| column.is_primary_key) {
                        order_parts.push(format!(
                            r#"{inner_alias}.{} {direction_sql}"#,
                            quote_identifier(&pk.name)
                        ));
                    }

                    format!(
                        "LEFT JOIN LATERAL (SELECT * FROM {} {inner_alias} WHERE {} ORDER BY {} LIMIT 1) {target_alias} ON TRUE",
                        qualified_table_sql(&join.target_binding.table),
                        conditions.join(" AND "),
                        order_parts.join(", ")
                    )
                }
            };

            join_clauses.push(join_sql);
            binding_aliases.insert(join.target_binding.name.clone(), target_alias);
            pending.remove(index);
        }

        if pending.len() == pending_len {
            let remaining = pending
                .iter()
                .map(|join| join.target_binding.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ValidationError(format!(
                "Planner joins could not be resolved from the available sources: {remaining}"
            ))
            .into());
        }
    }

    Ok((binding_aliases, join_clauses))
}

async fn load_planner_catalog(
    pool: &PgPool,
    draft: &PlannerDraftDef,
) -> anyhow::Result<PlannerCatalog> {
    let bindings_by_name = build_bindings_by_name(draft)?;
    let base_binding = bindings_by_name
        .get(&draft.base_binding)
        .expect("base binding existence was validated");

    let mut unique_tables: Vec<(String, String)> = Vec::new();
    let mut seen_tables = HashSet::new();
    for binding in bindings_by_name.values() {
        if binding.table.schema != base_binding.table.schema {
            return Err(ValidationError(
                "Cross-schema joins are not supported for saved views".into(),
            )
            .into());
        }
        let key = (binding.table.schema.clone(), binding.table.table.clone());
        if seen_tables.insert(key.clone()) {
            unique_tables.push(key);
        }
    }

    let refs = unique_tables
        .iter()
        .map(|(schema, table)| (schema.as_str(), table.as_str()))
        .collect::<Vec<_>>();

    let columns_by_table = get_columns_bulk(pool, &refs).await?;
    let fk_edges = load_fk_edges(pool, &base_binding.table.schema).await?;

    Ok(PlannerCatalog {
        columns_by_table,
        fk_edges,
    })
}

fn compile_planner_view_query(
    draft: &PlannerDraftDef,
    catalog: &PlannerCatalog,
) -> anyhow::Result<PlannedViewQuery> {
    let bindings_by_name = build_bindings_by_name(draft)?;
    let referenced_bindings = collect_referenced_binding_names(draft);
    let (binding_aliases, join_clauses) =
        build_binding_aliases_and_joins(draft, &bindings_by_name, catalog, &referenced_bindings)?;
    let base_binding = bindings_by_name
        .get(&draft.base_binding)
        .expect("base binding existence was validated");

    if draft.latest_per_group.is_some() && draft.rank_limit.is_some() {
        return Err(ValidationError(
            "latest-per-group and per-group ranking cannot be combined in one view".into(),
        )
        .into());
    }
    if let Some(ranking) = &draft.rank_limit
        && ranking.per_group_limit == 0
    {
        return Err(ValidationError("per-group ranking limit must be at least 1".into()).into());
    }

    let mut bind_state = SqlBindState::default();
    let ctx = PlannerCompileContext {
        bindings_by_name: &bindings_by_name,
        binding_aliases: &binding_aliases,
        columns_by_table: &catalog.columns_by_table,
    };
    let projections = build_projection_defs(draft, &ctx, &mut bind_state)?;

    let has_standard_aggregates = projections.iter().any(|projection| {
        projection
            .aggregate
            .is_some_and(PlannerAggregate::is_standard_group_aggregate)
    });
    let has_latest_projection = projections
        .iter()
        .any(|projection| projection.aggregate == Some(PlannerAggregate::Latest));

    if has_latest_projection && draft.latest_per_group.is_none() {
        return Err(ValidationError(
            "LATEST requires latest-per-group context in the view definition".into(),
        )
        .into());
    }
    if has_standard_aggregates && (draft.latest_per_group.is_some() || draft.rank_limit.is_some()) {
        return Err(ValidationError(
            "Grouped aggregates cannot be combined with latest-per-group or per-group ranking"
                .into(),
        )
        .into());
    }

    let filter_targets = projections
        .iter()
        .map(|projection| DefinitionFilterTarget {
            output_name: projection.output_name.clone(),
            sql_expr: projection.expr_sql.clone(),
            info: projection.info.clone(),
            aggregate: projection.aggregate,
        })
        .collect::<Vec<_>>();
    let definition_conditions =
        build_definition_filters(&filter_targets, &draft.filters, &mut bind_state)?;
    let where_clause = if definition_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", definition_conditions.join(" AND "))
    };

    let output_columns = projections
        .iter()
        .map(|projection| {
            build_output_column_from_info(
                &projection.info,
                projection.output_name.clone(),
                projection.aggregate,
            )
        })
        .collect::<Vec<_>>();

    let select_exprs = projections
        .iter()
        .map(|projection| match projection.aggregate {
            Some(aggregate) if aggregate.is_standard_group_aggregate() => format!(
                "{}({}) AS {}",
                aggregate.as_sql(),
                projection.expr_sql,
                quote_identifier(&projection.output_name)
            ),
            _ => format!(
                "{} AS {}",
                projection.expr_sql,
                quote_identifier(&projection.output_name)
            ),
        })
        .collect::<Vec<_>>();

    let from_clause = format!("FROM {} t0", qualified_table_sql(&base_binding.table));

    let inner_sql = if let Some(latest) = &draft.latest_per_group {
        if latest.partition_by.is_empty() {
            return Err(ValidationError(
                "latest-per-group requires at least one grouping expression".into(),
            )
            .into());
        }
        let partition_exprs = latest
            .partition_by
            .iter()
            .map(|expr| {
                compile_value_expr(expr, &ctx, &mut bind_state).map(|compiled| compiled.sql)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let order_expr = compile_value_expr(&latest.order_by.expr, &ctx, &mut bind_state)?;
        let order_sql = format!("{} {}", order_expr.sql, latest.order_by.direction.as_sql());
        [
            format!(
                "SELECT DISTINCT ON ({}) {}",
                partition_exprs.join(", "),
                select_exprs.join(", ")
            ),
            from_clause,
            join_clauses.join(" "),
            where_clause,
            format!("ORDER BY {}, {}", partition_exprs.join(", "), order_sql),
        ]
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
    } else if let Some(ranking) = &draft.rank_limit {
        if ranking.partition_by.is_empty() {
            return Err(ValidationError(
                "per-group ranking requires at least one grouping expression".into(),
            )
            .into());
        }
        let partition_exprs = ranking
            .partition_by
            .iter()
            .map(|expr| {
                compile_value_expr(expr, &ctx, &mut bind_state).map(|compiled| compiled.sql)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let order_expr = compile_value_expr(&ranking.order_by.expr, &ctx, &mut bind_state)?;
        let rank_alias = "__planner_rank";
        let ranked_inner_sql = [
            format!(
                "SELECT {}, ROW_NUMBER() OVER (PARTITION BY {} ORDER BY {} {}) AS {}",
                select_exprs.join(", "),
                partition_exprs.join(", "),
                order_expr.sql,
                ranking.order_by.direction.as_sql(),
                quote_identifier(rank_alias)
            ),
            from_clause,
            join_clauses.join(" "),
            where_clause,
        ]
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

        format!(
            "SELECT {} FROM ({ranked_inner_sql}) AS ranked_rows WHERE {} <= {}",
            projections
                .iter()
                .map(|projection| quote_identifier(&projection.output_name))
                .collect::<Vec<_>>()
                .join(", "),
            quote_identifier(rank_alias),
            ranking.per_group_limit
        )
    } else if has_standard_aggregates {
        let mut group_by_exprs = draft
            .group_by
            .iter()
            .map(|expr| {
                compile_value_expr(expr, &ctx, &mut bind_state).map(|compiled| compiled.sql)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        for projection in &projections {
            if projection.aggregate.is_none() && !group_by_exprs.contains(&projection.expr_sql) {
                group_by_exprs.push(projection.expr_sql.clone());
            }
        }
        let group_by_clause = if group_by_exprs.is_empty() {
            String::new()
        } else {
            format!("GROUP BY {}", group_by_exprs.join(", "))
        };
        [
            format!("SELECT {}", select_exprs.join(", ")),
            from_clause,
            join_clauses.join(" "),
            where_clause,
            group_by_clause,
        ]
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
    } else {
        [
            format!("SELECT {}", select_exprs.join(", ")),
            from_clause,
            join_clauses.join(" "),
            where_clause,
        ]
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
    };

    Ok(PlannedViewQuery {
        inner_sql,
        output_columns,
        bind_values: bind_state.values,
    })
}

fn build_v1_planner_draft(draft: &ViewDraft<'_>) -> anyhow::Result<PlannerDraftDef> {
    if !is_valid_identifier(draft.base_schema) {
        return Err(ValidationError(format!("Invalid schema name: {}", draft.base_schema)).into());
    }
    if !is_valid_identifier(draft.base_table) {
        return Err(ValidationError(format!("Invalid table name: {}", draft.base_table)).into());
    }
    if draft.columns.is_empty() {
        return Err(ValidationError("Saved views must include at least one column".into()).into());
    }

    let base_binding_name = "base".to_string();
    let mut bindings = vec![PlannerBinding {
        name: base_binding_name.clone(),
        table: TableKey::new(draft.base_schema, draft.base_table),
    }];
    let mut binding_names_by_table = HashMap::from([(
        (draft.base_schema.to_string(), draft.base_table.to_string()),
        base_binding_name.clone(),
    )]);

    let output_names = resolve_view_output_names(draft.columns)?;
    let mut projections = Vec::with_capacity(draft.columns.len());
    for (column, output_name) in draft.columns.iter().zip(output_names.into_iter()) {
        if !is_valid_identifier(&column.source_schema) {
            return Err(
                ValidationError(format!("Invalid schema name: {}", column.source_schema)).into(),
            );
        }
        if !is_valid_identifier(&column.source_table) {
            return Err(
                ValidationError(format!("Invalid table name: {}", column.source_table)).into(),
            );
        }
        if !is_valid_identifier(&column.column_name) {
            return Err(
                ValidationError(format!("Invalid column name: {}", column.column_name)).into(),
            );
        }
        if let Some(alias) = &column.alias
            && !is_valid_identifier(alias)
        {
            return Err(ValidationError(format!("Invalid alias: {alias}")).into());
        }
        if column.source_schema != draft.base_schema {
            return Err(ValidationError(
                "Cross-schema joins are not supported for saved views".into(),
            )
            .into());
        }

        let table_key = (column.source_schema.clone(), column.source_table.clone());
        let binding_name = if let Some(existing) = binding_names_by_table.get(&table_key) {
            existing.clone()
        } else {
            let candidate = format!("{}_{}", column.source_schema, column.source_table);
            if !is_valid_identifier(&candidate) {
                return Err(ValidationError(format!(
                    "Invalid planner binding name derived from source table: {candidate}"
                ))
                .into());
            }
            bindings.push(PlannerBinding {
                name: candidate.clone(),
                table: TableKey::new(&column.source_schema, &column.source_table),
            });
            binding_names_by_table.insert(table_key, candidate.clone());
            candidate
        };

        projections.push(PlannerProjection {
            output_name,
            expr: PlannerValueExpr::Field(PlannerFieldRef {
                binding: binding_name,
                column: column.column_name.clone(),
            }),
            aggregate: column
                .aggregate
                .map(PlannerAggregate::from_view_aggregate)
                .transpose()?,
        });
    }

    let mut filter_entries: Vec<_> = draft.filters.iter().collect();
    filter_entries.sort_by_key(|(output_name, _)| output_name.as_str());
    let filters = filter_entries
        .into_iter()
        .map(|(output_name, value)| PlannerFilter {
            output_name: output_name.clone(),
            value: PlannerFilterValue::Contains(value.clone()),
        })
        .collect();

    Ok(PlannerDraftDef {
        base_binding: base_binding_name,
        bindings,
        joins: Vec::new(),
        projections,
        filters,
        group_by: Vec::new(),
        latest_per_group: None,
        rank_limit: None,
    })
}

struct PlannerBindingRegistry {
    base_binding: String,
    base_table: TableKey,
    bindings: Vec<PlannerBinding>,
    joins: Vec<PlannerJoin>,
    source_bindings: HashMap<String, String>,
    binding_tables: HashMap<String, TableKey>,
    bindings_by_table: HashMap<(String, String), Vec<String>>,
    next_binding_index: usize,
}

impl PlannerBindingRegistry {
    fn new(base_schema: &str, base_table: &str) -> Self {
        let base_binding = "base".to_string();
        let base_table_ref = TableKey::new(base_schema, base_table);
        Self {
            base_binding: base_binding.clone(),
            base_table: base_table_ref.clone(),
            bindings: vec![PlannerBinding {
                name: base_binding.clone(),
                table: base_table_ref.clone(),
            }],
            joins: Vec::new(),
            source_bindings: HashMap::new(),
            binding_tables: HashMap::from([(base_binding.clone(), base_table_ref.clone())]),
            bindings_by_table: HashMap::from([(
                (base_schema.to_string(), base_table.to_string()),
                vec![base_binding.clone()],
            )]),
            next_binding_index: 1,
        }
    }

    fn register_binding(&mut self, table: TableKey) -> PlannerBinding {
        let name = format!("src{}", self.next_binding_index);
        self.next_binding_index += 1;
        let binding = PlannerBinding {
            name: name.clone(),
            table: table.clone(),
        };
        self.bindings.push(binding.clone());
        self.binding_tables.insert(name.clone(), table.clone());
        self.bindings_by_table
            .entry((table.schema.clone(), table.table.clone()))
            .or_default()
            .push(name);
        binding
    }

    fn register_source_binding(
        &mut self,
        source_id: &str,
        schema: &str,
        table: &str,
    ) -> anyhow::Result<PlannerBinding> {
        if source_id.trim().is_empty() {
            return Err(ValidationError("Saved-view source IDs must not be empty".into()).into());
        }
        if self.source_bindings.contains_key(source_id) {
            return Err(
                ValidationError(format!("Duplicate saved-view source ID: {source_id}")).into(),
            );
        }

        let binding = self.register_binding(TableKey::new(schema, table));
        self.source_bindings
            .insert(source_id.to_string(), binding.name.clone());
        Ok(binding)
    }

    fn resolve_binding_name(
        &mut self,
        source_id: Option<&str>,
        source_schema: &str,
        source_table: &str,
    ) -> anyhow::Result<String> {
        if source_schema == self.base_table.schema
            && source_table == self.base_table.table
            && source_id.is_none()
        {
            return Ok(self.base_binding.clone());
        }

        if let Some(source_id) = source_id {
            let binding_name = self.source_bindings.get(source_id).ok_or_else(|| {
                ValidationError(format!("Unknown saved-view source ID: {source_id}"))
            })?;
            let binding_table = self.binding_tables.get(binding_name).ok_or_else(|| {
                ValidationError(format!("Unknown planner binding: {binding_name}"))
            })?;
            if binding_table.schema != source_schema || binding_table.table != source_table {
                return Err(ValidationError(format!(
                    "Saved-view source '{source_id}' does not match {source_schema}.{source_table}"
                ))
                .into());
            }
            return Ok(binding_name.clone());
        }

        if source_schema == self.base_table.schema && source_table == self.base_table.table {
            return Ok(self.base_binding.clone());
        }

        let key = (source_schema.to_string(), source_table.to_string());
        match self.bindings_by_table.get(&key) {
            Some(binding_names) if binding_names.len() == 1 => Ok(binding_names[0].clone()),
            Some(binding_names) if binding_names.len() > 1 => Err(ValidationError(format!(
                "Multiple saved-view sources target {source_schema}.{source_table}; specify source_id"
            ))
            .into()),
            _ => Ok(self
                .register_binding(TableKey::new(source_schema, source_table))
                .name),
        }
    }
}

fn planner_sort_direction(direction: SavedViewSortDirection) -> SortDirection {
    match direction {
        SavedViewSortDirection::Asc => SortDirection::Asc,
        SavedViewSortDirection::Desc => SortDirection::Desc,
    }
}

fn planner_aggregate_from_saved_view(
    aggregate: SavedViewAggregate,
) -> anyhow::Result<PlannerAggregate> {
    match aggregate {
        SavedViewAggregate::Sum => Ok(PlannerAggregate::Sum),
        SavedViewAggregate::Avg => Ok(PlannerAggregate::Avg),
        SavedViewAggregate::Count => Ok(PlannerAggregate::Count),
        SavedViewAggregate::Min => Ok(PlannerAggregate::Min),
        SavedViewAggregate::Max => Ok(PlannerAggregate::Max),
        SavedViewAggregate::Latest => Ok(PlannerAggregate::Latest),
    }
}

fn planner_neighbor_direction(direction: ViewSelfDirection) -> PlannerNeighborDirection {
    match direction {
        ViewSelfDirection::Previous => PlannerNeighborDirection::Previous,
        ViewSelfDirection::Next => PlannerNeighborDirection::Next,
    }
}

fn planner_filter_from_view_filter(output_name: String, filter: &ViewFilterValue) -> PlannerFilter {
    let value = match filter {
        ViewFilterValue::Legacy(value) => PlannerFilterValue::Contains(value.clone()),
        ViewFilterValue::Structured(filter) => match filter {
            super::ViewStructuredFilter::Eq { value } => PlannerFilterValue::Eq(value.clone()),
            super::ViewStructuredFilter::Gt { value } => PlannerFilterValue::Gt(value.clone()),
            super::ViewStructuredFilter::Gte { value } => PlannerFilterValue::Gte(value.clone()),
            super::ViewStructuredFilter::Lt { value } => PlannerFilterValue::Lt(value.clone()),
            super::ViewStructuredFilter::Lte { value } => PlannerFilterValue::Lte(value.clone()),
            super::ViewStructuredFilter::Contains { value } => {
                PlannerFilterValue::Contains(value.clone())
            }
            super::ViewStructuredFilter::StartsWith { value } => {
                PlannerFilterValue::StartsWith(value.clone())
            }
            super::ViewStructuredFilter::Between { value } => {
                PlannerFilterValue::Between(value[0].clone(), value[1].clone())
            }
            super::ViewStructuredFilter::IsEmpty => PlannerFilterValue::IsEmpty,
            super::ViewStructuredFilter::InList { value } => {
                PlannerFilterValue::InList(value.clone())
            }
        },
    };

    PlannerFilter { output_name, value }
}

fn parse_planner_literal(value: &str) -> PlannerLiteral {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        PlannerLiteral::Boolean(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        PlannerLiteral::Boolean(false)
    } else if trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok() {
        PlannerLiteral::Number(trimmed.to_string())
    } else {
        PlannerLiteral::String(value.to_string())
    }
}

fn option_string<'a>(options: Option<&'a serde_json::Value>, key: &str) -> Option<&'a str> {
    options
        .and_then(|options| options.get(key))
        .and_then(serde_json::Value::as_str)
}

fn condition_option_string<'a>(
    options: Option<&'a serde_json::Value>,
    key: &str,
) -> Option<&'a str> {
    options
        .and_then(|options| options.get("condition"))
        .and_then(|condition| condition.get(key))
        .and_then(serde_json::Value::as_str)
}

fn resolve_planner_field_expr(
    registry: &mut PlannerBindingRegistry,
    source_id: Option<&str>,
    source_schema: &str,
    source_table: &str,
    column_name: &str,
) -> anyhow::Result<PlannerValueExpr> {
    if !is_valid_identifier(source_schema) {
        return Err(ValidationError(format!("Invalid schema name: {source_schema}")).into());
    }
    if !is_valid_identifier(source_table) {
        return Err(ValidationError(format!("Invalid table name: {source_table}")).into());
    }
    if !is_valid_identifier(column_name) {
        return Err(ValidationError(format!("Invalid column name: {column_name}")).into());
    }

    Ok(PlannerValueExpr::Field(PlannerFieldRef {
        binding: registry.resolve_binding_name(source_id, source_schema, source_table)?,
        column: column_name.to_string(),
    }))
}

fn planner_expr_from_column_ref(
    registry: &mut PlannerBindingRegistry,
    reference: &ViewColumnRef,
) -> anyhow::Result<PlannerValueExpr> {
    resolve_planner_field_expr(
        registry,
        reference.source_id.as_deref(),
        &reference.source_schema,
        &reference.source_table,
        &reference.column_name,
    )
}

fn planner_order_expr_from_view(
    registry: &mut PlannerBindingRegistry,
    order_by: &ViewOrderBy,
) -> anyhow::Result<PlannerOrderExpr> {
    Ok(PlannerOrderExpr {
        expr: resolve_planner_field_expr(
            registry,
            order_by.source_id.as_deref(),
            &order_by.source_schema,
            &order_by.source_table,
            &order_by.column_name,
        )?,
        direction: planner_sort_direction(order_by.direction),
    })
}

fn planner_expr_from_derived_input(
    registry: &mut PlannerBindingRegistry,
    input: &ViewDerivedInput,
) -> anyhow::Result<PlannerValueExpr> {
    match input.kind {
        ViewDerivedInputKind::Column => {
            let schema = input.source_schema.as_deref().ok_or_else(|| {
                ValidationError("Derived column input is missing source_schema".into())
            })?;
            let table = input.source_table.as_deref().ok_or_else(|| {
                ValidationError("Derived column input is missing source_table".into())
            })?;
            let column_name = input.column_name.as_deref().ok_or_else(|| {
                ValidationError("Derived column input is missing column_name".into())
            })?;
            resolve_planner_field_expr(
                registry,
                input.source_id.as_deref(),
                schema,
                table,
                column_name,
            )
        }
        ViewDerivedInputKind::Literal => {
            let value = input
                .value
                .as_deref()
                .ok_or_else(|| ValidationError("Derived literal input is missing value".into()))?;
            Ok(PlannerValueExpr::Literal(parse_planner_literal(value)))
        }
    }
}

fn planner_expr_from_derived_column(
    registry: &mut PlannerBindingRegistry,
    derived: &ViewDerivedColumn,
) -> anyhow::Result<PlannerValueExpr> {
    let inputs = derived
        .inputs
        .iter()
        .map(|input| planner_expr_from_derived_input(registry, input))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let options = derived.options.as_ref();

    let require_input = |index: usize| -> anyhow::Result<PlannerValueExpr> {
        inputs.get(index).cloned().ok_or_else(|| {
            ValidationError(format!(
                "Derived operation {:?} requires input {}",
                derived.operation,
                index + 1
            ))
            .into()
        })
    };

    match derived.operation {
        ViewDerivedOperation::Difference => Ok(PlannerValueExpr::Difference {
            left: Box::new(require_input(0)?),
            right: Box::new(require_input(1)?),
        }),
        ViewDerivedOperation::RatioPercent => Ok(PlannerValueExpr::RatioPercent {
            numerator: Box::new(require_input(0)?),
            denominator: Box::new(require_input(1)?),
        }),
        ViewDerivedOperation::AgeOfTimestamp => Ok(PlannerValueExpr::AgeOfTimestamp {
            value: Box::new(require_input(0)?),
        }),
        ViewDerivedOperation::DateBucket => {
            let bucket = match option_string(options, "bucket") {
                Some("day") => PlannerDateBucket::Day,
                Some("week") => PlannerDateBucket::Week,
                Some(other) => {
                    return Err(ValidationError(format!(
                        "Unsupported date bucket option: {other}"
                    ))
                    .into());
                }
                None => {
                    return Err(
                        ValidationError("date_bucket requires a bucket option".into()).into(),
                    );
                }
            };
            Ok(PlannerValueExpr::DateBucket {
                value: Box::new(require_input(0)?),
                bucket,
            })
        }
        ViewDerivedOperation::DatePart => {
            let part = match option_string(options, "part") {
                Some("year") => PlannerDatePart::Year,
                Some("month") => PlannerDatePart::Month,
                Some("weekday") => PlannerDatePart::Weekday,
                Some(other) => {
                    return Err(
                        ValidationError(format!("Unsupported date part option: {other}")).into(),
                    );
                }
                None => {
                    return Err(ValidationError("date_part requires a part option".into()).into());
                }
            };
            Ok(PlannerValueExpr::DatePart {
                value: Box::new(require_input(0)?),
                part,
            })
        }
        ViewDerivedOperation::TextConcat => Ok(PlannerValueExpr::TextConcat { parts: inputs }),
        ViewDerivedOperation::TextLength => Ok(PlannerValueExpr::TextLength {
            value: Box::new(require_input(0)?),
        }),
        ViewDerivedOperation::IfThen => {
            let op = option_string(options, "op")
                .or_else(|| condition_option_string(options, "op"))
                .ok_or_else(|| {
                    ValidationError("if_then requires an options.op condition".into())
                })?;
            let condition = match op {
                "is_empty" => PlannerCondition::IsEmpty(Box::new(require_input(0)?)),
                "eq" | "gt" | "gte" | "lt" | "lte" => PlannerCondition::Compare {
                    left: Box::new(require_input(0)?),
                    op: match op {
                        "eq" => PlannerCompareOp::Eq,
                        "gt" => PlannerCompareOp::Gt,
                        "gte" => PlannerCompareOp::Gte,
                        "lt" => PlannerCompareOp::Lt,
                        "lte" => PlannerCompareOp::Lte,
                        _ => unreachable!(),
                    },
                    right: Box::new(require_input(1)?),
                },
                other => {
                    return Err(ValidationError(format!(
                        "Unsupported if_then condition operator: {other}"
                    ))
                    .into());
                }
            };

            let (then_index, else_index) = if op == "is_empty" { (1, 2) } else { (2, 3) };
            Ok(PlannerValueExpr::IfThen {
                condition,
                then_expr: Box::new(require_input(then_index)?),
                else_expr: Box::new(require_input(else_index)?),
            })
        }
    }
}

fn build_v2_planner_draft(
    base_schema: &str,
    base_table: &str,
    shape: &ViewDefinitionShape,
) -> anyhow::Result<PlannerDraftDef> {
    if !is_valid_identifier(base_schema) {
        return Err(ValidationError(format!("Invalid schema name: {base_schema}")).into());
    }
    if !is_valid_identifier(base_table) {
        return Err(ValidationError(format!("Invalid table name: {base_table}")).into());
    }
    if shape.columns.is_empty() {
        return Err(ValidationError("Saved views must include at least one column".into()).into());
    }

    let mut registry = PlannerBindingRegistry::new(base_schema, base_table);

    for source in &shape.sources {
        if !is_valid_identifier(&source.schema) {
            return Err(ValidationError(format!("Invalid schema name: {}", source.schema)).into());
        }
        if !is_valid_identifier(&source.table) {
            return Err(ValidationError(format!("Invalid table name: {}", source.table)).into());
        }

        let binding =
            registry.register_source_binding(&source.id, &source.schema, &source.table)?;
        match source.kind {
            ViewSourceKind::Fk => {}
            ViewSourceKind::Match => {
                let match_ref = source.r#match.as_ref().ok_or_else(|| {
                    ValidationError(format!(
                        "Saved-view source '{}' is missing match columns",
                        source.id
                    ))
                })?;
                if !is_valid_identifier(&match_ref.base_column) {
                    return Err(ValidationError(format!(
                        "Invalid column name: {}",
                        match_ref.base_column
                    ))
                    .into());
                }
                if !is_valid_identifier(&match_ref.source_column) {
                    return Err(ValidationError(format!(
                        "Invalid column name: {}",
                        match_ref.source_column
                    ))
                    .into());
                }
                registry.joins.push(PlannerJoin {
                    source_binding: registry.base_binding.clone(),
                    target_binding: binding,
                    kind: PlannerJoinKind::MatchColumns {
                        pairs: vec![PlannerJoinColumnPair {
                            source_column: match_ref.base_column.clone(),
                            target_column: match_ref.source_column.clone(),
                        }],
                    },
                });
            }
            ViewSourceKind::SelfJoin => {
                let self_join = source.self_join.as_ref().ok_or_else(|| {
                    ValidationError(format!(
                        "Saved-view source '{}' is missing self-join settings",
                        source.id
                    ))
                })?;
                if source.schema != base_schema || source.table != base_table {
                    return Err(ValidationError(format!(
                        "Self-join sources must target the base table: {}",
                        source.id
                    ))
                    .into());
                }
                if !is_valid_identifier(&self_join.entity_column) {
                    return Err(ValidationError(format!(
                        "Invalid column name: {}",
                        self_join.entity_column
                    ))
                    .into());
                }
                if !is_valid_identifier(&self_join.order_column) {
                    return Err(ValidationError(format!(
                        "Invalid column name: {}",
                        self_join.order_column
                    ))
                    .into());
                }
                registry.joins.push(PlannerJoin {
                    source_binding: registry.base_binding.clone(),
                    target_binding: binding,
                    kind: PlannerJoinKind::PreviousRow {
                        entity_pairs: vec![PlannerJoinColumnPair {
                            source_column: self_join.entity_column.clone(),
                            target_column: self_join.entity_column.clone(),
                        }],
                        order_by_column: self_join.order_column.clone(),
                        direction: planner_neighbor_direction(self_join.direction.clone()),
                    },
                });
            }
        }
    }

    let output_names = resolve_saved_view_output_names(&shape.columns)?;
    let projections = shape
        .columns
        .iter()
        .zip(output_names)
        .map(|(column, output_name)| {
            let expr = if let Some(derived) = &column.derived {
                planner_expr_from_derived_column(&mut registry, derived)?
            } else {
                resolve_planner_field_expr(
                    &mut registry,
                    column.source_id.as_deref(),
                    &column.source_schema,
                    &column.source_table,
                    &column.column_name,
                )?
            };

            Ok(PlannerProjection {
                output_name,
                expr,
                aggregate: column
                    .aggregate
                    .map(planner_aggregate_from_saved_view)
                    .transpose()?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut filter_entries: Vec<_> = shape.filters.iter().collect();
    filter_entries.sort_by_key(|(output_name, _)| output_name.as_str());
    let filters = filter_entries
        .into_iter()
        .map(|(output_name, filter)| planner_filter_from_view_filter(output_name.clone(), filter))
        .collect::<Vec<_>>();

    let group_by = shape
        .grouping
        .as_ref()
        .map(|grouping| {
            grouping
                .keys
                .iter()
                .map(|reference| planner_expr_from_column_ref(&mut registry, reference))
                .collect::<anyhow::Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();

    let latest_per_group = shape
        .grouping
        .as_ref()
        .and_then(|grouping| {
            grouping
                .latest_by
                .as_ref()
                .map(|latest_by| (grouping, latest_by))
        })
        .map(|(grouping, latest_by)| {
            Ok::<PlannerLatestPerGroup, anyhow::Error>(PlannerLatestPerGroup {
                partition_by: grouping
                    .keys
                    .iter()
                    .map(|reference| planner_expr_from_column_ref(&mut registry, reference))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                order_by: planner_order_expr_from_view(&mut registry, latest_by)?,
            })
        })
        .transpose()?;

    let rank_limit = shape
        .ranking
        .as_ref()
        .map(|ranking| {
            let order_by = ranking.order_by.as_ref().ok_or_else(|| {
                ValidationError("Per-group ranking requires an order_by field".into())
            })?;
            Ok::<PlannerRankLimit, anyhow::Error>(PlannerRankLimit {
                partition_by: ranking
                    .partition_by
                    .iter()
                    .map(|reference| planner_expr_from_column_ref(&mut registry, reference))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                order_by: planner_order_expr_from_view(&mut registry, order_by)?,
                per_group_limit: ranking.limit,
            })
        })
        .transpose()?;

    Ok(PlannerDraftDef {
        base_binding: registry.base_binding,
        bindings: registry.bindings,
        joins: registry.joins,
        projections,
        filters,
        group_by,
        latest_per_group,
        rank_limit,
    })
}

async fn plan_view_query(pool: &PgPool, draft: &ViewDraft<'_>) -> anyhow::Result<PlannedViewQuery> {
    let planner_draft = build_v1_planner_draft(draft)?;
    let catalog = load_planner_catalog(pool, &planner_draft).await?;
    compile_planner_view_query(&planner_draft, &catalog)
}

async fn plan_view_shape_query(
    pool: &PgPool,
    base_schema: &str,
    base_table: &str,
    shape: &ViewDefinitionShape,
) -> anyhow::Result<PlannedViewQuery> {
    let planner_draft = build_v2_planner_draft(base_schema, base_table, shape)?;
    let catalog = load_planner_catalog(pool, &planner_draft).await?;
    compile_planner_view_query(&planner_draft, &catalog)
}

fn rows_to_json(rows: &[sqlx::postgres::PgRow], columns: &[ColumnInfo]) -> Vec<serde_json::Value> {
    rows.iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for col in columns {
                let val = pg_value_to_json(row, &col.name, &col.data_type);
                map.insert(col.name.clone(), val);
            }
            serde_json::Value::Object(map)
        })
        .collect()
}

async fn query_projected_rows(
    pool: &PgPool,
    plan: &PlannedViewQuery,
    page: u32,
    page_size: u32,
    sort: &[SortEntry],
    search: Option<&str>,
    filters: &HashMap<String, String>,
) -> anyhow::Result<QueryResult> {
    let qb = build_query_clauses(
        &plan.output_columns,
        search,
        filters,
        sort,
        true,
        plan.bind_values.len() as u32 + 1,
    )?;

    let count_sql = format!(
        "SELECT COUNT(*) AS cnt FROM ({}) AS saved_view_rows {}",
        plan.inner_sql, qb.where_clause
    );
    let mut count_query = sqlx::query(&count_sql);
    for value in &plan.bind_values {
        count_query = count_query.bind(value);
    }
    for value in &qb.bind_values {
        count_query = count_query.bind(value);
    }
    let total_rows: i64 = count_query.fetch_one(pool).await?.get("cnt");

    let offset = (page.saturating_sub(1) as u64) * (page_size as u64);
    let data_sql = format!(
        "SELECT * FROM ({}) AS saved_view_rows {} {} LIMIT {} OFFSET {offset}",
        plan.inner_sql, qb.where_clause, qb.order_clause, page_size
    );
    let mut data_query = sqlx::query(&data_sql);
    for value in &plan.bind_values {
        data_query = data_query.bind(value);
    }
    for value in &qb.bind_values {
        data_query = data_query.bind(value);
    }
    let rows = data_query.fetch_all(pool).await?;

    Ok(QueryResult {
        columns: plan.output_columns.clone(),
        rows: rows_to_json(&rows, &plan.output_columns),
        total_rows,
        page,
        page_size,
    })
}

pub async fn preview_view(
    pool: &PgPool,
    draft: &ViewDraft<'_>,
    page_size: u32,
) -> anyhow::Result<QueryResult> {
    if draft.columns.is_empty() {
        return Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            total_rows: 0,
            page: 1,
            page_size: page_size.clamp(1, 100),
        });
    }

    let plan = plan_view_query(pool, draft).await?;
    query_projected_rows(
        pool,
        &plan,
        1,
        page_size.clamp(1, 100),
        &[],
        None,
        &HashMap::new(),
    )
    .await
}

pub async fn query_view_rows(
    pool: &PgPool,
    params: &ViewRowsQueryParams<'_>,
) -> anyhow::Result<QueryResult> {
    let plan = plan_view_query(pool, &params.draft).await?;
    query_projected_rows(
        pool,
        &plan,
        params.page,
        params.page_size,
        params.sort,
        params.search,
        params.filters,
    )
    .await
}

pub async fn preview_view_shape(
    pool: &PgPool,
    base_schema: &str,
    base_table: &str,
    shape: &ViewDefinitionShape,
    page_size: u32,
) -> anyhow::Result<QueryResult> {
    if shape.columns.is_empty() {
        return Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            total_rows: 0,
            page: 1,
            page_size: page_size.clamp(1, 100),
        });
    }

    let plan = plan_view_shape_query(pool, base_schema, base_table, shape).await?;
    query_projected_rows(
        pool,
        &plan,
        1,
        page_size.clamp(1, 100),
        &[],
        None,
        &HashMap::new(),
    )
    .await
}

pub async fn query_view_shape_rows(
    pool: &PgPool,
    base_schema: &str,
    base_table: &str,
    shape: &ViewDefinitionShape,
    page: u32,
    page_size: u32,
    sort: &[SortEntry],
    search: Option<&str>,
    filters: &HashMap<String, String>,
) -> anyhow::Result<QueryResult> {
    let plan = plan_view_shape_query(pool, base_schema, base_table, shape).await?;
    query_projected_rows(pool, &plan, page, page_size, sort, search, filters).await
}

pub async fn export_view_shape_rows_stream<'a>(
    pool: &'a PgPool,
    base_schema: &'a str,
    base_table: &'a str,
    shape: &'a ViewDefinitionShape,
    sort: &'a [SortEntry],
    search: Option<&'a str>,
    filters: &'a HashMap<String, String>,
) -> anyhow::Result<(
    Vec<ColumnInfo>,
    Pin<Box<dyn Stream<Item = Result<sqlx::postgres::PgRow, sqlx::Error>> + Send + 'a>>,
)> {
    let plan = plan_view_shape_query(pool, base_schema, base_table, shape).await?;
    let qb = build_query_clauses(
        &plan.output_columns,
        search,
        filters,
        sort,
        false,
        plan.bind_values.len() as u32 + 1,
    )?;

    let query_sql = format!(
        "SELECT * FROM ({}) AS saved_view_rows {} {}",
        plan.inner_sql, qb.where_clause, qb.order_clause
    );

    let mut bind_values = plan.bind_values.clone();
    bind_values.extend(qb.bind_values);

    let columns = plan.output_columns.clone();
    let stream = async_stream::stream! {
        let mut query = sqlx::query(&query_sql);
        for value in &bind_values {
            query = query.bind(value);
        }
        use futures::StreamExt;
        let mut row_stream = query.fetch(pool);
        while let Some(row) = row_stream.next().await {
            yield row;
        }
    };

    Ok((columns, Box::pin(stream)))
}

pub async fn export_view_rows_stream<'a>(
    pool: &'a PgPool,
    params: &'a ViewExportQueryParams<'a>,
) -> anyhow::Result<(
    Vec<ColumnInfo>,
    Pin<Box<dyn Stream<Item = Result<sqlx::postgres::PgRow, sqlx::Error>> + Send + 'a>>,
)> {
    let plan = plan_view_query(pool, &params.draft).await?;
    let qb = build_query_clauses(
        &plan.output_columns,
        params.search,
        params.filters,
        params.sort,
        false,
        plan.bind_values.len() as u32 + 1,
    )?;

    let query_sql = format!(
        "SELECT * FROM ({}) AS saved_view_rows {} {}",
        plan.inner_sql, qb.where_clause, qb.order_clause
    );

    let mut bind_values = plan.bind_values.clone();
    bind_values.extend(qb.bind_values);

    let columns = plan.output_columns.clone();
    let stream = async_stream::stream! {
        let mut query = sqlx::query(&query_sql);
        for value in &bind_values {
            query = query.bind(value);
        }
        use futures::StreamExt;
        let mut row_stream = query.fetch(pool);
        while let Some(row) = row_stream.next().await {
            yield row;
        }
    };

    Ok((columns, Box::pin(stream)))
}

/// Query paginated rows from a table with optional sort, search, and per-column filters.
pub async fn query_rows(pool: &PgPool, params: &RowQueryParams<'_>) -> anyhow::Result<QueryResult> {
    let schema = params.schema;
    let table = params.table;

    if !is_valid_identifier(schema) {
        anyhow::bail!("Invalid schema name");
    }
    if !is_valid_identifier(table) {
        anyhow::bail!("Invalid table name");
    }

    let columns = get_columns(pool, schema, table).await?;
    let qb = build_query_clauses(
        &columns,
        params.search,
        params.filters,
        params.sort,
        true,
        1,
    )?;

    // Count total matching rows
    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM \"{schema}\".\"{table}\" {}",
        qb.where_clause
    );
    let mut count_query = sqlx::query(&count_sql);
    for val in &qb.bind_values {
        count_query = count_query.bind(val);
    }
    let total_rows: i64 = count_query.fetch_one(pool).await?.get("cnt");

    let offset = (params.page.saturating_sub(1) as u64) * (params.page_size as u64);
    let query_sql = format!(
        "SELECT * FROM \"{schema}\".\"{table}\" {} {} LIMIT {} OFFSET {offset}",
        qb.where_clause, qb.order_clause, params.page_size
    );

    let mut data_query = sqlx::query(&query_sql);
    for val in &qb.bind_values {
        data_query = data_query.bind(val);
    }
    let rows = data_query.fetch_all(pool).await?;

    let json_rows: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for col in &columns {
                let val = pg_value_to_json(row, &col.name, &col.data_type);
                map.insert(col.name.clone(), val);
            }
            serde_json::Value::Object(map)
        })
        .collect();

    Ok(QueryResult {
        columns,
        rows: json_rows,
        total_rows,
        page: params.page,
        page_size: params.page_size,
    })
}

/// Build a streaming query for CSV export — returns all matching rows without pagination.
/// The returned stream yields `Result<PgRow>` items for incremental processing.
pub async fn export_rows_stream<'a>(
    pool: &'a PgPool,
    params: &'a ExportQueryParams<'a>,
) -> anyhow::Result<(
    Vec<ColumnInfo>,
    Pin<Box<dyn Stream<Item = Result<sqlx::postgres::PgRow, sqlx::Error>> + Send + 'a>>,
)> {
    let schema = params.schema;
    let table = params.table;

    if !is_valid_identifier(schema) {
        anyhow::bail!("Invalid schema name");
    }
    if !is_valid_identifier(table) {
        anyhow::bail!("Invalid table name");
    }

    let columns = get_columns(pool, schema, table).await?;
    let qb = build_query_clauses(
        &columns,
        params.search,
        params.filters,
        params.sort,
        false,
        1,
    )?;

    let query_sql = format!(
        "SELECT * FROM \"{schema}\".\"{table}\" {} {}",
        qb.where_clause, qb.order_clause
    );

    // We need to own the bind values so the stream can reference them.
    // Use a channel-based approach: build the query with owned values.
    let bind_values = qb.bind_values;

    let stream = async_stream::stream! {
        let mut query = sqlx::query(&query_sql);
        for val in &bind_values {
            query = query.bind(val);
        }
        use futures::StreamExt;
        let mut row_stream = query.fetch(pool);
        while let Some(row) = row_stream.next().await {
            yield row;
        }
    };

    Ok((columns, Box::pin(stream)))
}

/// Convert a PostgreSQL column value to a serde_json::Value.
fn pg_value_to_json(row: &sqlx::postgres::PgRow, col: &str, data_type: &str) -> serde_json::Value {
    use serde_json::Value;

    match data_type {
        "smallint" => row
            .try_get::<i16, _>(col)
            .map(|v| Value::from(v as i64))
            .unwrap_or(Value::Null),
        "integer" => row
            .try_get::<i32, _>(col)
            .map(|v| Value::from(v as i64))
            .unwrap_or(Value::Null),
        "bigint" => row
            .try_get::<i64, _>(col)
            .map(|v| {
                // Values beyond ±2^53 lose precision in JavaScript's float64 JSON.parse.
                // Serialize as string to preserve full i64 range.
                if v.unsigned_abs() > (1u64 << 53) {
                    Value::String(v.to_string())
                } else {
                    Value::from(v)
                }
            })
            .unwrap_or(Value::Null),
        "real" => row
            .try_get::<f32, _>(col)
            .map(|v| {
                // Parse via f32's string repr to avoid f64 widening artifacts
                // (e.g., 3.14f32 as f64 → 3.140000104904175)
                let clean: f64 = v.to_string().parse().unwrap_or(v as f64);
                Value::from(clean)
            })
            .unwrap_or(Value::Null),
        "double precision" => row
            .try_get::<f64, _>(col)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "numeric" => row
            .try_get::<rust_decimal::Decimal, _>(col)
            .map(|v| Value::String(v.to_string()))
            .unwrap_or(Value::Null),
        "boolean" => row
            .try_get::<bool, _>(col)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "json" | "jsonb" => row
            .try_get::<serde_json::Value, _>(col)
            .unwrap_or(Value::Null),
        "timestamp without time zone" => row
            .try_get::<chrono::NaiveDateTime, _>(col)
            .map(|v| Value::String(v.format("%Y-%m-%d %H:%M:%S").to_string()))
            .unwrap_or(Value::Null),
        "timestamp with time zone" => row
            .try_get::<chrono::DateTime<chrono::Utc>, _>(col)
            .map(|v| Value::String(v.to_rfc3339()))
            .unwrap_or(Value::Null),
        "date" => row
            .try_get::<chrono::NaiveDate, _>(col)
            .map(|v| Value::String(v.format("%Y-%m-%d").to_string()))
            .unwrap_or(Value::Null),
        "time without time zone" | "time with time zone" => row
            .try_get::<chrono::NaiveTime, _>(col)
            .map(|v| Value::String(v.format("%H:%M:%S").to_string()))
            .unwrap_or(Value::Null),
        "uuid" => row
            .try_get::<uuid::Uuid, _>(col)
            .map(|v| Value::String(v.to_string()))
            .unwrap_or(Value::Null),
        _ => row
            .try_get::<String, _>(col)
            .map(Value::from)
            .unwrap_or(Value::Null),
    }
}

/// Convert SQL type names to human-friendly labels.
fn humanize_type(data_type: &str, _udt: &str) -> String {
    match data_type {
        "character varying" | "character" | "text" => "Text".to_string(),
        "integer" | "smallint" | "bigint" => "Number".to_string(),
        "real" | "double precision" | "numeric" => "Decimal".to_string(),
        "boolean" => "Yes/No".to_string(),
        "date" => "Date".to_string(),
        "timestamp without time zone" | "timestamp with time zone" => "Date & Time".to_string(),
        "json" | "jsonb" => "JSON".to_string(),
        "time without time zone" | "time with time zone" => "Time".to_string(),
        "interval" => "Duration".to_string(),
        "uuid" => "UUID".to_string(),
        "inet" | "cidr" => "Network".to_string(),
        "money" => "Currency".to_string(),
        "bytea" => "Binary".to_string(),
        "ARRAY" => "List".to_string(),
        other => other.to_string(),
    }
}

fn is_text_type(data_type: &str) -> bool {
    matches!(
        data_type,
        "character varying" | "character" | "text" | "uuid"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expression_info(data_type: &str, display_type: &str) -> ExpressionInfo {
        ExpressionInfo {
            data_type: data_type.into(),
            display_type: display_type.into(),
            is_nullable: false,
            is_primary_key: false,
        }
    }

    fn planner_catalog_with_tables(
        tables: Vec<((&str, &str), Vec<ColumnInfo>)>,
        fk_edges: Vec<FkEdge>,
    ) -> PlannerCatalog {
        PlannerCatalog {
            columns_by_table: tables
                .into_iter()
                .map(|((schema, table), columns)| {
                    ((schema.to_string(), table.to_string()), columns)
                })
                .collect(),
            fk_edges,
        }
    }

    fn test_columns() -> Vec<ColumnInfo> {
        vec![
            ColumnInfo {
                name: "id".into(),
                data_type: "bigint".into(),
                display_type: "Number".into(),
                is_nullable: false,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "vehicle_id".into(),
                data_type: "text".into(),
                display_type: "Text".into(),
                is_nullable: false,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "logged_at".into(),
                data_type: "timestamp with time zone".into(),
                display_type: "Date & Time".into(),
                is_nullable: false,
                is_primary_key: false,
            },
        ]
    }

    #[test]
    fn valid_identifier_accepts_safe_chars() {
        assert!(is_valid_identifier("vehicle_id"));
        assert!(is_valid_identifier("col1"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("CamelCase"));
        assert!(is_valid_identifier("col-name"));
        assert!(is_valid_identifier("col name"));
        assert!(is_valid_identifier("my-table"));
    }

    #[test]
    fn valid_identifier_rejects_special_chars() {
        assert!(!is_valid_identifier("col.name"));
        assert!(!is_valid_identifier("col;DROP TABLE"));
        assert!(!is_valid_identifier("col\"name"));
        assert!(!is_valid_identifier(""));
    }

    #[test]
    fn humanize_type_maps_common_types() {
        assert_eq!(humanize_type("character varying", "varchar"), "Text");
        assert_eq!(humanize_type("integer", "int4"), "Number");
        assert_eq!(humanize_type("boolean", "bool"), "Yes/No");
        assert_eq!(
            humanize_type("timestamp with time zone", "timestamptz"),
            "Date & Time"
        );
    }

    #[test]
    fn is_text_type_identifies_text_columns() {
        assert!(is_text_type("character varying"));
        assert!(is_text_type("text"));
        assert!(is_text_type("uuid"));
        assert!(!is_text_type("integer"));
        assert!(!is_text_type("boolean"));
    }

    #[test]
    fn build_order_clause_supports_multi_sort_and_pk_tiebreaker() {
        let columns = test_columns();
        let sort = vec![
            SortEntry {
                column: "vehicle_id".into(),
                direction: SortDirection::Asc,
            },
            SortEntry {
                column: "logged_at".into(),
                direction: SortDirection::Desc,
            },
        ];

        let clause = build_order_clause(&columns, &sort, true).unwrap();
        assert_eq!(
            clause,
            r#"ORDER BY "vehicle_id" ASC, "logged_at" DESC, "id" ASC"#
        );
    }

    #[test]
    fn build_order_clause_is_empty_when_no_sort_is_active() {
        let columns = test_columns();

        let clause = build_order_clause(&columns, &[], true).unwrap();
        assert!(clause.is_empty());
    }

    #[test]
    fn build_order_clause_skips_duplicate_primary_key_tiebreaker() {
        let columns = test_columns();
        let sort = vec![SortEntry {
            column: "id".into(),
            direction: SortDirection::Desc,
        }];

        let clause = build_order_clause(&columns, &sort, true).unwrap();
        assert_eq!(clause, r#"ORDER BY "id" DESC"#);
    }

    #[test]
    fn build_order_clause_omits_pk_tiebreaker_for_export_queries() {
        let columns = test_columns();
        let sort = vec![SortEntry {
            column: "vehicle_id".into(),
            direction: SortDirection::Asc,
        }];

        let clause = build_order_clause(&columns, &sort, false).unwrap();
        assert_eq!(clause, r#"ORDER BY "vehicle_id" ASC"#);
    }

    #[test]
    fn build_order_clause_rejects_invalid_sort_column() {
        let columns = test_columns();
        let sort = vec![SortEntry {
            column: "id;drop table".into(),
            direction: SortDirection::Asc,
        }];

        let err = build_order_clause(&columns, &sort, false).unwrap_err();
        assert!(err.to_string().contains("Invalid sort column name"));
    }

    #[test]
    fn build_order_clause_rejects_unknown_sort_column() {
        let columns = test_columns();
        let sort = vec![SortEntry {
            column: "missing".into(),
            direction: SortDirection::Asc,
        }];

        let err = build_order_clause(&columns, &sort, false).unwrap_err();
        assert!(err.to_string().contains("Unknown sort column"));
    }

    fn sample_view_columns() -> Vec<ViewColumn> {
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
                source_table: "customers".into(),
                column_name: "id".into(),
                alias: None,
                aggregate: None,
            },
            ViewColumn {
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "total".into(),
                alias: None,
                aggregate: Some(ViewAggregate::Sum),
            },
        ]
    }

    fn fk_edge(
        source_table: &str,
        target_table: &str,
        source_columns: &[&str],
        target_columns: &[&str],
        constraint_name: &str,
    ) -> FkEdge {
        FkEdge {
            source: TableKey::new("public", source_table),
            target: TableKey::new("public", target_table),
            source_columns: source_columns
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            target_columns: target_columns
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            constraint_name: constraint_name.to_string(),
        }
    }

    #[test]
    fn resolve_view_output_names_prefixes_duplicate_non_aggregate_columns() {
        let output_names = resolve_view_output_names(&sample_view_columns()).unwrap();
        assert_eq!(
            output_names,
            vec![
                "orders__id".to_string(),
                "customers__id".to_string(),
                "sum_orders__total".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_view_output_names_rejects_duplicate_aliases() {
        let columns = vec![
            ViewColumn {
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "id".into(),
                alias: Some("duplicate".into()),
                aggregate: None,
            },
            ViewColumn {
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "total".into(),
                alias: Some("duplicate".into()),
                aggregate: Some(ViewAggregate::Sum),
            },
        ];

        let err = resolve_view_output_names(&columns).unwrap_err();
        assert!(err.to_string().contains("Duplicate output column name"));
    }

    #[test]
    fn find_fk_path_prefers_shortest_path_then_lexical_constraint_order() {
        let edges = vec![
            fk_edge(
                "orders",
                "customers",
                &["customer_id"],
                &["id"],
                "b_orders_customers",
            ),
            fk_edge(
                "orders",
                "accounts",
                &["account_id"],
                &["id"],
                "a_orders_accounts",
            ),
            fk_edge(
                "accounts",
                "regions",
                &["region_id"],
                &["id"],
                "z_accounts_regions",
            ),
            fk_edge(
                "customers",
                "regions",
                &["region_id"],
                &["id"],
                "a_customers_regions",
            ),
        ];

        let path = find_fk_path(
            &edges,
            &TableKey::new("public", "orders"),
            &TableKey::new("public", "regions"),
        );

        assert_eq!(path.len(), 2);
        assert_eq!(path[0].constraint_name, "a_orders_accounts");
        assert_eq!(path[0].to_table, "accounts");
        assert_eq!(path[1].constraint_name, "z_accounts_regions");
    }

    #[test]
    fn build_output_column_uses_postgres_aggregate_return_types() {
        let integer_source = ColumnInfo {
            name: "total".into(),
            data_type: "integer".into(),
            display_type: "Number".into(),
            is_nullable: false,
            is_primary_key: false,
        };
        let double_source = ColumnInfo {
            name: "ratio".into(),
            data_type: "double precision".into(),
            display_type: "Decimal".into(),
            is_nullable: false,
            is_primary_key: false,
        };

        let sum_integer = build_output_column(
            &integer_source,
            "sum_orders__total".into(),
            Some(ViewAggregate::Sum),
        );
        let avg_double = build_output_column(
            &double_source,
            "avg_orders__ratio".into(),
            Some(ViewAggregate::Avg),
        );

        assert_eq!(sum_integer.data_type, "bigint");
        assert_eq!(sum_integer.display_type, "Number");
        assert_eq!(avg_double.data_type, "double precision");
        assert_eq!(avg_double.display_type, "Decimal");
    }

    #[test]
    fn build_definition_filters_rejects_aggregate_columns() {
        let filter_targets = vec![DefinitionFilterTarget {
            output_name: "sum_orders__total".into(),
            sql_expr: r#"t0."total""#.into(),
            info: expression_info("numeric", "Decimal"),
            aggregate: Some(PlannerAggregate::Sum),
        }];
        let filters = vec![PlannerFilter {
            output_name: "sum_orders__total".into(),
            value: PlannerFilterValue::Eq("100".into()),
        }];
        let mut bind_state = SqlBindState::default();

        let err = build_definition_filters(&filter_targets, &filters, &mut bind_state).unwrap_err();
        assert!(err.to_string().contains("Aggregate columns cannot be used"));
    }

    #[test]
    fn build_definition_filters_supports_boolean_exact_match() {
        let filter_targets = vec![DefinitionFilterTarget {
            output_name: "is_active".into(),
            sql_expr: r#"t0."is_active""#.into(),
            info: expression_info("boolean", "Yes/No"),
            aggregate: None,
        }];
        let filters = vec![PlannerFilter {
            output_name: "is_active".into(),
            value: PlannerFilterValue::Eq("yes".into()),
        }];
        let mut bind_state = SqlBindState::default();

        let conditions =
            build_definition_filters(&filter_targets, &filters, &mut bind_state).unwrap();

        assert_eq!(
            conditions,
            vec![r#"t0."is_active" = $1::boolean"#.to_string()]
        );
        assert_eq!(bind_state.values, vec!["true".to_string()]);
    }

    #[test]
    fn compile_planner_view_query_uses_explicit_group_by_keys() {
        let draft = PlannerDraftDef {
            base_binding: "base".into(),
            bindings: vec![PlannerBinding {
                name: "base".into(),
                table: TableKey::new("public", "vehicle_logs"),
            }],
            joins: Vec::new(),
            projections: vec![PlannerProjection {
                output_name: "row_count".into(),
                expr: PlannerValueExpr::Field(PlannerFieldRef {
                    binding: "base".into(),
                    column: "id".into(),
                }),
                aggregate: Some(PlannerAggregate::Count),
            }],
            filters: Vec::new(),
            group_by: vec![PlannerValueExpr::Field(PlannerFieldRef {
                binding: "base".into(),
                column: "vehicle_id".into(),
            })],
            latest_per_group: None,
            rank_limit: None,
        };
        let catalog = planner_catalog_with_tables(
            vec![(
                ("public", "vehicle_logs"),
                vec![
                    ColumnInfo {
                        name: "id".into(),
                        data_type: "bigint".into(),
                        display_type: "Number".into(),
                        is_nullable: false,
                        is_primary_key: true,
                    },
                    ColumnInfo {
                        name: "vehicle_id".into(),
                        data_type: "text".into(),
                        display_type: "Text".into(),
                        is_nullable: false,
                        is_primary_key: false,
                    },
                ],
            )],
            Vec::new(),
        );

        let planned = compile_planner_view_query(&draft, &catalog).unwrap();
        assert!(planned.inner_sql.contains(r#"GROUP BY t0."vehicle_id""#));
    }

    #[test]
    fn build_v2_planner_draft_maps_match_self_sources_and_derived_ops() {
        let shape = ViewDefinitionShape {
            columns: vec![
                SavedViewColumn {
                    kind: Some(SavedViewColumnKind::Source),
                    source_id: None,
                    source_schema: "public".into(),
                    source_table: "vehicle_logs".into(),
                    column_name: "vehicle_id".into(),
                    alias: None,
                    aggregate: None,
                    derived: None,
                },
                SavedViewColumn {
                    kind: Some(SavedViewColumnKind::Source),
                    source_id: Some("match-vehicles".into()),
                    source_schema: "public".into(),
                    source_table: "vehicles".into(),
                    column_name: "label".into(),
                    alias: Some("vehicle_label".into()),
                    aggregate: None,
                    derived: None,
                },
                SavedViewColumn {
                    kind: Some(SavedViewColumnKind::Derived),
                    source_id: Some("self-previous".into()),
                    source_schema: "public".into(),
                    source_table: "vehicle_logs".into(),
                    column_name: "battery_soc".into(),
                    alias: Some("soc_delta".into()),
                    aggregate: None,
                    derived: Some(ViewDerivedColumn {
                        operation: ViewDerivedOperation::Difference,
                        inputs: vec![
                            ViewDerivedInput {
                                kind: ViewDerivedInputKind::Column,
                                source_id: None,
                                source_schema: Some("public".into()),
                                source_table: Some("vehicle_logs".into()),
                                column_name: Some("battery_soc".into()),
                                value: None,
                            },
                            ViewDerivedInput {
                                kind: ViewDerivedInputKind::Column,
                                source_id: Some("self-previous".into()),
                                source_schema: Some("public".into()),
                                source_table: Some("vehicle_logs".into()),
                                column_name: Some("battery_soc".into()),
                                value: None,
                            },
                        ],
                        options: None,
                    }),
                },
            ],
            filters: HashMap::from([(
                "vehicle_label".into(),
                ViewFilterValue::Structured(crate::db::ViewStructuredFilter::StartsWith {
                    value: "A".into(),
                }),
            )]),
            sources: vec![
                crate::db::ViewSourceRef {
                    id: "match-vehicles".into(),
                    kind: ViewSourceKind::Match,
                    schema: "public".into(),
                    table: "vehicles".into(),
                    label: None,
                    r#match: Some(crate::db::ViewSourceMatch {
                        base_column: "vehicle_id".into(),
                        source_column: "id".into(),
                    }),
                    self_join: None,
                },
                crate::db::ViewSourceRef {
                    id: "self-previous".into(),
                    kind: ViewSourceKind::SelfJoin,
                    schema: "public".into(),
                    table: "vehicle_logs".into(),
                    label: None,
                    r#match: None,
                    self_join: Some(crate::db::ViewSelfSource {
                        entity_column: "vehicle_id".into(),
                        order_column: "scanned_at".into(),
                        direction: ViewSelfDirection::Previous,
                    }),
                },
            ],
            grouping: None,
            ranking: None,
            template: None,
        };

        let draft = build_v2_planner_draft("public", "vehicle_logs", &shape).unwrap();

        assert_eq!(draft.bindings.len(), 3);
        assert_eq!(draft.joins.len(), 2);
        assert!(matches!(
            draft.joins[0].kind,
            PlannerJoinKind::MatchColumns { .. }
        ));
        assert!(matches!(
            draft.joins[1].kind,
            PlannerJoinKind::PreviousRow { .. }
        ));
        assert!(matches!(
            draft.projections[2].expr,
            PlannerValueExpr::Difference { .. }
        ));
        assert!(matches!(
            draft.filters[0].value,
            PlannerFilterValue::StartsWith(_)
        ));
    }
}
