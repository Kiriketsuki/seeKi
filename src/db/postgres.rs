use std::collections::HashMap;
use std::pin::Pin;

use futures::Stream;
use serde::Serialize;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use super::{
    ColumnInfo, ExportQueryParams, QueryResult, RowQueryParams, TableInfo, ValidationError,
};

/// Result of a connection test — describes a single table visible to the server.
#[derive(Debug, Serialize)]
pub struct TablePreview {
    pub name: String,
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
            let display_name = if schema == "public" {
                table_name
            } else {
                format!("{schema}.{table_name}")
            };
            TablePreview {
                name: display_name,
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

    // Deduplicate schemas and tables, and validate schema names.
    let mut schemas: Vec<String> = refs.iter().map(|(s, _)| (*s).to_string()).collect();
    schemas.sort();
    schemas.dedup();
    for s in &schemas {
        if !is_valid_identifier(s) {
            anyhow::bail!("Invalid schema name: {s}");
        }
    }
    let mut tables: Vec<String> = refs.iter().map(|(_, t)| (*t).to_string()).collect();
    tables.sort();
    tables.dedup();

    let wanted: std::collections::HashSet<(String, String)> = refs
        .iter()
        .map(|(s, t)| ((*s).to_string(), (*t).to_string()))
        .collect();

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
            WHERE tc.table_schema = ANY($1)
              AND tc.table_name = ANY($2)
              AND tc.constraint_type = 'PRIMARY KEY'
        ) pk
          ON pk.table_schema = c.table_schema
         AND pk.table_name = c.table_name
         AND pk.column_name = c.column_name
        WHERE c.table_schema = ANY($1)
          AND c.table_name = ANY($2)
        ORDER BY c.table_schema, c.table_name, c.ordinal_position
        "#,
    )
    .bind(&schemas)
    .bind(&tables)
    .fetch_all(pool)
    .await?;

    let mut result: HashMap<(String, String), Vec<ColumnInfo>> = HashMap::new();
    for r in &rows {
        let schema_name: String = r.get("table_schema");
        let table_name: String = r.get("table_name");
        let key = (schema_name, table_name);
        // Filter out rows that don't exactly match a requested (schema, table) pair.
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
fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
}

/// Shared WHERE clause and bind values builder for query_rows and export.
struct QueryBuilder {
    where_clause: String,
    order_clause: String,
    bind_values: Vec<String>,
}

fn build_query_clauses(
    columns: &[ColumnInfo],
    search: Option<&str>,
    filters: &std::collections::HashMap<String, String>,
    sort_column: Option<&str>,
    sort_direction: Option<&str>,
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
    let mut param_idx: u32 = 1;

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

    let order_clause = if let Some(col) = sort_column {
        if !is_valid_identifier(col) {
            return Err(ValidationError(format!("Invalid sort column name: {col}")).into());
        }
        if !valid_column_names.contains(col) {
            return Err(ValidationError(format!("Unknown sort column: {col}")).into());
        }
        let dir = match sort_direction {
            Some("desc" | "DESC") => "DESC",
            _ => "ASC",
        };
        format!("ORDER BY \"{col}\" {dir}")
    } else {
        String::new()
    };

    Ok(QueryBuilder {
        where_clause,
        order_clause,
        bind_values,
    })
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
        params.sort_column,
        params.sort_direction,
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
        params.sort_column,
        params.sort_direction,
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
}
