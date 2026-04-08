use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use super::{ColumnInfo, QueryResult, RowQueryParams, TableInfo, ValidationError};

/// Test a PostgreSQL connection and return the list of public table names.
/// Opens a temporary pool, queries table names, then closes it.
pub async fn test_connection(url: &str) -> anyhow::Result<Vec<String>> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(url)
        .await?;

    let rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name",
    )
    .fetch_all(&pool)
    .await?;

    let tables: Vec<String> = rows.iter().map(|r| r.get("table_name")).collect();

    pool.close().await;

    Ok(tables)
}

/// List all user tables in the public schema with estimated row counts.
pub async fn list_tables(pool: &PgPool) -> anyhow::Result<Vec<TableInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT
            c.relname AS table_name,
            c.reltuples::bigint AS row_estimate
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relkind = 'r'
        ORDER BY c.relname
        "#,
    )
    .fetch_all(pool)
    .await?;

    let tables = rows
        .iter()
        .map(|r| TableInfo {
            name: r.get("table_name"),
            row_count_estimate: r.get("row_estimate"),
        })
        .collect();

    Ok(tables)
}

/// Get column metadata for a table.
pub async fn get_columns(pool: &PgPool, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
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
            WHERE tc.table_name = $1
              AND tc.constraint_type = 'PRIMARY KEY'
        ) pk ON pk.column_name = c.column_name
        WHERE c.table_schema = 'public'
          AND c.table_name = $1
        ORDER BY c.ordinal_position
        "#,
    )
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

/// Validate that a name contains only alphanumeric characters and underscores.
fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Query paginated rows from a table with optional sort, search, and per-column filters.
pub async fn query_rows(
    pool: &PgPool,
    params: &RowQueryParams<'_>,
) -> anyhow::Result<QueryResult> {
    let table = params.table;
    let page = params.page;
    let page_size = params.page_size;
    let sort_column = params.sort_column;
    let sort_direction = params.sort_direction;
    let search = params.search;
    let filters = params.filters;

    // Validate table name (prevent SQL injection — only allow alphanumeric + underscore)
    if !is_valid_identifier(table) {
        anyhow::bail!("Invalid table name");
    }

    let columns = get_columns(pool, table).await?;

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

    // Build WHERE conditions with tracked bind parameters
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();
    let mut param_idx: u32 = 1;

    // Search condition (searches across all text columns)
    let has_search = search.is_some_and(|s| !s.is_empty());
    if has_search {
        let text_cols: Vec<String> = columns
            .iter()
            .filter(|c| is_text_type(&c.data_type))
            .map(|c| format!("\"{}\"::text ILIKE '%' || ${param_idx} || '%'", c.name))
            .collect();

        if !text_cols.is_empty() {
            conditions.push(format!("({})", text_cols.join(" OR ")));
            bind_values.push(search.unwrap().to_string());
            param_idx += 1;
        }
    }

    // Per-column filter conditions (AND-ed, ILIKE with %value%)
    // Sort by key for deterministic parameter ordering
    let mut filter_entries: Vec<_> = filters.iter().collect();
    filter_entries.sort_by_key(|(k, _)| k.as_str());

    for (col_name, value) in &filter_entries {
        conditions.push(format!("\"{}\"::text ILIKE ${param_idx}", col_name));
        bind_values.push(format!("%{value}%"));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count total matching rows
    let count_sql = format!("SELECT COUNT(*) as cnt FROM \"{table}\" {where_clause}");
    let mut count_query = sqlx::query(&count_sql);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }
    let total_rows: i64 = count_query.fetch_one(pool).await?.get("cnt");

    // Build ORDER BY
    let order_clause = if let Some(col) = sort_column {
        if !is_valid_identifier(col) {
            anyhow::bail!("Invalid sort column name");
        }
        let dir = match sort_direction {
            Some("desc" | "DESC") => "DESC",
            _ => "ASC",
        };
        format!("ORDER BY \"{col}\" {dir}")
    } else {
        String::new()
    };

    let offset = (page.saturating_sub(1)) * page_size;
    let query_sql = format!(
        "SELECT * FROM \"{table}\" {where_clause} {order_clause} LIMIT {page_size} OFFSET {offset}"
    );

    let mut data_query = sqlx::query(&query_sql);
    for val in &bind_values {
        data_query = data_query.bind(val);
    }
    let rows = data_query.fetch_all(pool).await?;

    // Convert rows to JSON values
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
        page,
        page_size,
    })
}

/// Convert a PostgreSQL column value to a serde_json::Value.
fn pg_value_to_json(row: &sqlx::postgres::PgRow, col: &str, data_type: &str) -> serde_json::Value {
    use serde_json::Value;

    // Try to extract based on type, falling back to string representation
    match data_type {
        "integer" | "smallint" | "bigint" => row
            .try_get::<i64, _>(col)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "real" | "double precision" | "numeric" => row
            .try_get::<f64, _>(col)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "boolean" => row
            .try_get::<bool, _>(col)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "json" | "jsonb" => row
            .try_get::<serde_json::Value, _>(col)
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
    fn valid_identifier_accepts_alphanumeric_underscore() {
        assert!(is_valid_identifier("vehicle_id"));
        assert!(is_valid_identifier("col1"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("CamelCase"));
    }

    #[test]
    fn valid_identifier_rejects_special_chars() {
        assert!(!is_valid_identifier("col-name"));
        assert!(!is_valid_identifier("col.name"));
        assert!(!is_valid_identifier("col name"));
        assert!(!is_valid_identifier("col;DROP TABLE"));
        assert!(!is_valid_identifier(""));
    }

    #[test]
    fn humanize_type_maps_common_types() {
        assert_eq!(humanize_type("character varying", "varchar"), "Text");
        assert_eq!(humanize_type("integer", "int4"), "Number");
        assert_eq!(humanize_type("boolean", "bool"), "Yes/No");
        assert_eq!(humanize_type("timestamp with time zone", "timestamptz"), "Date & Time");
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
