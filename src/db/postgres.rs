use std::collections::{HashMap, HashSet, VecDeque};
use std::pin::Pin;

use futures::Stream;
use serde::Serialize;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use super::{
    ColumnInfo, ExportQueryParams, FkHop, QueryResult, RowQueryParams, SortDirection, SortEntry,
    TableInfo, ValidationError, ViewAggregate, ViewColumn, ViewDraft, ViewExportQueryParams,
    ViewRowsQueryParams,
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
struct PlannedViewColumn {
    source: TableKey,
    source_column: ColumnInfo,
    output_name: String,
    aggregate: Option<ViewAggregate>,
}

#[derive(Debug, Clone)]
struct PlannedViewQuery {
    inner_sql: String,
    output_columns: Vec<ColumnInfo>,
    bind_values: Vec<String>,
}

fn quote_identifier(name: &str) -> String {
    format!(r#""{name}""#)
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

fn aggregate_result_data_type(source_data_type: &str, aggregate: ViewAggregate) -> String {
    match aggregate {
        ViewAggregate::Count => "bigint".to_string(),
        ViewAggregate::Sum => match source_data_type {
            "smallint" | "integer" => "bigint".to_string(),
            "bigint" | "numeric" => "numeric".to_string(),
            "real" | "double precision" => "double precision".to_string(),
            other => other.to_string(),
        },
        ViewAggregate::Avg => match source_data_type {
            "real" | "double precision" => "double precision".to_string(),
            "smallint" | "integer" | "bigint" | "numeric" => "numeric".to_string(),
            other => other.to_string(),
        },
        ViewAggregate::Min | ViewAggregate::Max => source_data_type.to_string(),
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

fn build_output_column(
    source: &ColumnInfo,
    output_name: String,
    aggregate: Option<ViewAggregate>,
) -> ColumnInfo {
    match aggregate {
        None => ColumnInfo {
            name: output_name,
            data_type: source.data_type.clone(),
            display_type: source.display_type.clone(),
            is_nullable: source.is_nullable,
            is_primary_key: source.is_primary_key,
        },
        Some(aggregate) => {
            let data_type = aggregate_result_data_type(&source.data_type, aggregate);
            ColumnInfo {
                name: output_name,
                display_type: humanize_type(&data_type, ""),
                data_type,
                is_nullable: !matches!(aggregate, ViewAggregate::Count),
                is_primary_key: false,
            }
        }
    }
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

fn build_definition_filters(
    planned_columns: &[PlannedViewColumn],
    aliases: &HashMap<TableKey, String>,
    filters: &HashMap<String, String>,
    start_param_idx: u32,
) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    let mut planned_by_output: HashMap<&str, &PlannedViewColumn> = HashMap::new();
    for planned in planned_columns {
        planned_by_output.insert(planned.output_name.as_str(), planned);
    }

    let mut entries: Vec<_> = filters.iter().collect();
    entries.sort_by_key(|(key, _)| key.as_str());

    let mut conditions = Vec::new();
    let mut bind_values = Vec::new();
    let mut param_idx = start_param_idx;

    for (output_name, value) in entries {
        if !is_valid_identifier(output_name) {
            return Err(
                ValidationError(format!("Invalid filter column name: {output_name}")).into(),
            );
        }
        let Some(planned) = planned_by_output.get(output_name.as_str()) else {
            return Err(ValidationError(format!("Unknown filter column: {output_name}")).into());
        };
        if planned.aggregate.is_some() {
            return Err(ValidationError(format!(
                "Aggregate columns cannot be used in saved-view definition filters: {output_name}"
            ))
            .into());
        }

        let table_alias = aliases
            .get(&planned.source)
            .expect("planned view table alias should exist");
        let column_expr = format!(
            r#"{table_alias}.{}"#,
            quote_identifier(&planned.source_column.name)
        );

        if planned.source_column.data_type == "boolean" {
            let normalized = value.trim().to_lowercase();
            let bool_val = match normalized.as_str() {
                "yes" | "true" | "t" | "1" => Some(true),
                "no" | "false" | "f" | "0" => Some(false),
                _ => None,
            };
            if let Some(boolean) = bool_val {
                conditions.push(format!(
                    "{column_expr} = {}",
                    if boolean { "TRUE" } else { "FALSE" }
                ));
            } else {
                conditions.push("FALSE".to_string());
            }
        } else {
            conditions.push(format!(r#"{column_expr}::text ILIKE ${param_idx}"#));
            bind_values.push(format!("%{value}%"));
            param_idx += 1;
        }
    }

    Ok((conditions, bind_values))
}

async fn plan_view_query(pool: &PgPool, draft: &ViewDraft<'_>) -> anyhow::Result<PlannedViewQuery> {
    if !is_valid_identifier(draft.base_schema) {
        return Err(ValidationError(format!("Invalid schema name: {}", draft.base_schema)).into());
    }
    if !is_valid_identifier(draft.base_table) {
        return Err(ValidationError(format!("Invalid table name: {}", draft.base_table)).into());
    }
    if draft.columns.is_empty() {
        return Err(ValidationError("Saved views must include at least one column".into()).into());
    }

    let base = TableKey::new(draft.base_schema, draft.base_table);
    let output_names = resolve_view_output_names(draft.columns)?;

    let mut refs = vec![(draft.base_schema, draft.base_table)];
    let mut seen_refs = HashSet::from([(base.schema.clone(), base.table.clone())]);
    for column in draft.columns {
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
        if seen_refs.insert((column.source_schema.clone(), column.source_table.clone())) {
            refs.push((column.source_schema.as_str(), column.source_table.as_str()));
        }
    }

    let columns_by_table = get_columns_bulk(pool, &refs).await?;
    let fk_edges = load_fk_edges(pool, draft.base_schema).await?;

    let mut planned_columns = Vec::with_capacity(draft.columns.len());
    let mut target_order = Vec::new();
    let mut seen_targets = HashSet::new();

    for (column, output_name) in draft.columns.iter().zip(output_names.into_iter()) {
        let source = TableKey::new(&column.source_schema, &column.source_table);
        let table_columns = columns_by_table
            .get(&(source.schema.clone(), source.table.clone()))
            .ok_or_else(|| {
                ValidationError(format!(
                    "Unknown table in saved view: {}.{}",
                    column.source_schema, column.source_table
                ))
            })?;

        let source_column = table_columns
            .iter()
            .find(|candidate| candidate.name == column.column_name)
            .cloned()
            .ok_or_else(|| {
                ValidationError(format!(
                    "Unknown column in saved view: {}.{}.{}",
                    column.source_schema, column.source_table, column.column_name
                ))
            })?;

        if matches!(
            column.aggregate,
            Some(ViewAggregate::Sum | ViewAggregate::Avg)
        ) && !is_numeric_type(&source_column.data_type)
        {
            return Err(ValidationError(format!(
                "{} is only supported for numeric columns",
                column.aggregate.expect("checked").as_sql()
            ))
            .into());
        }
        if matches!(
            column.aggregate,
            Some(ViewAggregate::Min | ViewAggregate::Max)
        ) && !supports_min_max_type(&source_column.data_type)
        {
            return Err(ValidationError(format!(
                "{} is not supported for {} columns",
                column.aggregate.expect("checked").as_sql(),
                source_column.data_type
            ))
            .into());
        }

        if source != base && seen_targets.insert(source.clone()) {
            target_order.push(source.clone());
        }

        planned_columns.push(PlannedViewColumn {
            source,
            source_column,
            output_name,
            aggregate: column.aggregate,
        });
    }

    let mut aliases = HashMap::from([(base.clone(), "t0".to_string())]);
    let mut join_clauses = Vec::new();

    for target in &target_order {
        let path = find_fk_path(&fk_edges, &base, target);
        if path.is_empty() {
            return Err(ValidationError(format!(
                "No foreign-key path found from {}.{} to {}.{}",
                draft.base_schema, draft.base_table, target.schema, target.table
            ))
            .into());
        }

        for hop in path {
            let from = TableKey::new(&hop.from_schema, &hop.from_table);
            let to = TableKey::new(&hop.to_schema, &hop.to_table);
            if aliases.contains_key(&to) {
                continue;
            }

            let from_alias = aliases
                .get(&from)
                .expect("join path should build from known aliases")
                .clone();
            let to_alias = format!("t{}", aliases.len());
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
            aliases.insert(to, to_alias);
        }
    }

    let (definition_conditions, bind_values) =
        build_definition_filters(&planned_columns, &aliases, draft.filters, 1)?;

    let has_aggregate = planned_columns
        .iter()
        .any(|column| column.aggregate.is_some());
    let mut group_by_exprs = Vec::new();
    let mut select_exprs = Vec::with_capacity(planned_columns.len());
    let mut output_columns = Vec::with_capacity(planned_columns.len());

    for planned in &planned_columns {
        let table_alias = aliases
            .get(&planned.source)
            .expect("planned view table alias should exist");
        let source_expr = format!(
            r#"{table_alias}.{}"#,
            quote_identifier(&planned.source_column.name)
        );

        if has_aggregate && planned.aggregate.is_none() && !group_by_exprs.contains(&source_expr) {
            group_by_exprs.push(source_expr.clone());
        }

        let select_expr = match planned.aggregate {
            Some(aggregate) => format!(
                r#"{}({source_expr}) AS {}"#,
                aggregate.as_sql(),
                quote_identifier(&planned.output_name)
            ),
            None => format!(
                r#"{source_expr} AS {}"#,
                quote_identifier(&planned.output_name)
            ),
        };
        select_exprs.push(select_expr);
        output_columns.push(build_output_column(
            &planned.source_column,
            planned.output_name.clone(),
            planned.aggregate,
        ));
    }

    let where_clause = if definition_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", definition_conditions.join(" AND "))
    };

    let group_by_clause = if has_aggregate && !group_by_exprs.is_empty() {
        format!("GROUP BY {}", group_by_exprs.join(", "))
    } else {
        String::new()
    };

    let inner_sql = [
        format!("SELECT {}", select_exprs.join(", ")),
        format!("FROM {} t0", qualified_table_sql(&base)),
        join_clauses.join(" "),
        where_clause,
        group_by_clause,
    ]
    .into_iter()
    .filter(|segment| !segment.is_empty())
    .collect::<Vec<_>>()
    .join(" ");

    Ok(PlannedViewQuery {
        inner_sql,
        output_columns,
        bind_values,
    })
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
        let source_column = ColumnInfo {
            name: "total".into(),
            data_type: "numeric".into(),
            display_type: "Decimal".into(),
            is_nullable: false,
            is_primary_key: false,
        };
        let planned_columns = vec![PlannedViewColumn {
            source: TableKey::new("public", "orders"),
            source_column,
            output_name: "sum_orders__total".into(),
            aggregate: Some(ViewAggregate::Sum),
        }];
        let aliases = HashMap::from([(TableKey::new("public", "orders"), "t0".to_string())]);
        let filters = HashMap::from([("sum_orders__total".to_string(), "100".to_string())]);

        let err = build_definition_filters(&planned_columns, &aliases, &filters, 1).unwrap_err();
        assert!(err.to_string().contains("Aggregate columns cannot be used"));
    }

    #[test]
    fn build_definition_filters_supports_boolean_exact_match() {
        let source_column = ColumnInfo {
            name: "is_active".into(),
            data_type: "boolean".into(),
            display_type: "Yes/No".into(),
            is_nullable: false,
            is_primary_key: false,
        };
        let planned_columns = vec![PlannedViewColumn {
            source: TableKey::new("public", "orders"),
            source_column,
            output_name: "is_active".into(),
            aggregate: None,
        }];
        let aliases = HashMap::from([(TableKey::new("public", "orders"), "t0".to_string())]);
        let filters = HashMap::from([("is_active".to_string(), "yes".to_string())]);

        let (conditions, bind_values) =
            build_definition_filters(&planned_columns, &aliases, &filters, 1).unwrap();

        assert_eq!(conditions, vec!["t0.\"is_active\" = TRUE".to_string()]);
        assert!(bind_values.is_empty());
    }
}
