use sqlx::{PgPool, Row};

use super::{ColumnInfo, QueryResult, TableInfo};

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

/// Query paginated rows from a table with optional sort and search.
pub async fn query_rows(
    pool: &PgPool,
    table: &str,
    page: u32,
    page_size: u32,
    sort_column: Option<&str>,
    sort_direction: Option<&str>,
    search: Option<&str>,
) -> anyhow::Result<QueryResult> {
    // Validate table name (prevent SQL injection — only allow alphanumeric + underscore)
    if !table.chars().all(|c| c.is_alphanumeric() || c == '_') {
        anyhow::bail!("Invalid table name");
    }

    let columns = get_columns(pool, table).await?;

    // Build WHERE clause for search
    let where_clause = if let Some(term) = search {
        if term.is_empty() {
            String::new()
        } else {
            let text_cols: Vec<String> = columns
                .iter()
                .filter(|c| is_text_type(&c.data_type))
                .map(|c| format!("\"{}\"::text ILIKE '%' || $1 || '%'", c.name))
                .collect();

            if text_cols.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", text_cols.join(" OR "))
            }
        }
    } else {
        String::new()
    };

    // Count total matching rows
    let count_sql = format!("SELECT COUNT(*) as cnt FROM \"{table}\" {where_clause}");
    let total_rows: i64 = if search.is_some_and(|s| !s.is_empty()) {
        sqlx::query(&count_sql)
            .bind(search.unwrap_or_default())
            .fetch_one(pool)
            .await?
            .get("cnt")
    } else {
        sqlx::query(&format!("SELECT COUNT(*) as cnt FROM \"{table}\""))
            .fetch_one(pool)
            .await?
            .get("cnt")
    };

    // Build ORDER BY
    let order_clause = if let Some(col) = sort_column {
        if !col.chars().all(|c| c.is_alphanumeric() || c == '_') {
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

    let rows = if search.is_some_and(|s| !s.is_empty()) {
        sqlx::query(&query_sql)
            .bind(search.unwrap_or_default())
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query(&query_sql).fetch_all(pool).await?
    };

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
            .map(|v| Value::from(v))
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
        "uuid" => "UUID".to_string(),
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
