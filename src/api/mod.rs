pub mod preferences;
pub mod setup;
pub mod update;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Path, Query},
    http::header,
    response::IntoResponse,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::app_mode::{AppMode, SharedAppMode};
use crate::config::{display_name_column, display_name_table};
use crate::db::postgres::is_valid_identifier;
use crate::db::{
    ColumnInfo, ExportQueryParams, RowQueryParams, SortDirection, SortEntry, ValidationError,
};
use crate::store::Store;

pub fn router(mode: SharedAppMode, store: Store) -> Router {
    Router::new()
        .route("/tables", get(list_tables))
        .route("/tables/{schema}/{table}/columns", get(get_columns))
        .route("/tables/{schema}/{table}/rows", get(get_rows))
        .route("/config/display", get(get_display_config))
        .route("/connection-status", get(get_connection_status))
        .route("/export/{schema}/{table}/csv", get(export_csv))
        .route("/status", get(status))
        .route("/setup/test-connection", post(setup::test_connection))
        .route("/setup/save", post(setup::save_config))
        .route("/version", get(update::get_version))
        .route("/update/status", get(update::get_update_status))
        .route("/update/check", post(update::check_update))
        .route("/update/settings", patch(update::update_settings))
        // The three mutating update endpoints are gated by bearer-token auth.
        .merge(update::protected_update_router())
        .nest("/preferences", preferences::router())
        .layer(Extension(store))
        .layer(Extension(mode))
}

async fn status(Extension(mode): Extension<SharedAppMode>) -> Json<serde_json::Value> {
    let guard = mode.read().await;
    let mode_str = match &*guard {
        AppMode::Normal(_) => "normal",
        AppMode::Setup => "setup",
    };
    Json(serde_json::json!({ "mode": mode_str }))
}

/// Extract `Arc<AppState>` from the shared mode, returning 503 if in setup mode.
async fn require_state(mode: &SharedAppMode) -> Result<Arc<AppState>, AppError> {
    let guard = mode.read().await;
    match &*guard {
        AppMode::Normal(s) => Ok(Arc::clone(s)),
        AppMode::Setup => Err(AppError::service_unavailable(
            "This endpoint is not available in setup mode",
        )),
    }
}

#[derive(Serialize)]
struct DisplayConfigResponse {
    branding: BrandingResponse,
    tables: HashMap<String, TableDisplayConfig>,
}

#[derive(Serialize)]
struct BrandingResponse {
    title: Option<String>,
    subtitle: Option<String>,
}

#[derive(Serialize)]
struct ConnectionStatusResponse {
    database_kind: &'static str,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    schemas: Vec<String>,
    ssh_enabled: bool,
    ssh_connected: bool,
}

#[derive(Serialize)]
struct TableDisplayConfig {
    display_name: String,
    columns: HashMap<String, ColumnDisplayConfig>,
}

#[derive(Serialize)]
struct ColumnDisplayConfig {
    display_name: String,
}

async fn get_display_config(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
) -> Result<Json<DisplayConfigResponse>, AppError> {
    let state = require_state(&mode).await?;
    let settings = load_settings_map(&store).await?;
    let schemas = state.config.database.effective_schemas();
    let all_tables = state.db.list_tables(&schemas).await?;
    let allowed_tables: Vec<_> = all_tables
        .into_iter()
        .filter(|t| state.config.tables.allows(&t.schema, &t.name))
        .collect();

    let table_refs: Vec<(&str, &str)> = allowed_tables
        .iter()
        .map(|t| (t.schema.as_str(), t.name.as_str()))
        .collect();
    let all_columns = state.db.get_columns_bulk(&table_refs).await?;

    let mut tables = HashMap::new();
    for table in &allowed_tables {
        let key = (table.schema.clone(), table.name.clone());
        let columns: HashMap<String, ColumnDisplayConfig> = all_columns
            .get(&key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|c| {
                let display =
                    display_name_column(&table.schema, &table.name, &c.name, &state.config.display);
                (
                    c.name,
                    ColumnDisplayConfig {
                        display_name: display,
                    },
                )
            })
            .collect();

        let display = display_name_table(&table.schema, &table.name, &state.config.display);
        tables.insert(
            table.qualified(),
            TableDisplayConfig {
                display_name: display,
                columns,
            },
        );
    }

    Ok(Json(DisplayConfigResponse {
        branding: overlay_branding_settings(
            BrandingResponse {
                title: state.config.branding.title.clone(),
                subtitle: state.config.branding.subtitle.clone(),
            },
            &settings,
        ),
        tables,
    }))
}

async fn get_connection_status(
    Extension(mode): Extension<SharedAppMode>,
) -> Result<Json<ConnectionStatusResponse>, AppError> {
    let state = require_state(&mode).await?;
    let details = state
        .config
        .database
        .sanitized_connection_info()
        .map_err(AppError::internal)?;

    Ok(Json(ConnectionStatusResponse {
        database_kind: state.config.database.kind.as_str(),
        host: details.host,
        port: details.port,
        database: details.database,
        schemas: state.config.database.effective_schemas(),
        ssh_enabled: state.config.ssh.is_some(),
        ssh_connected: state.db.ssh_connected(),
    }))
}

async fn load_settings_map(store: &Store) -> Result<HashMap<String, serde_json::Value>, AppError> {
    Ok(crate::store::settings::get_all(store.pool())
        .await?
        .into_iter()
        .collect())
}

fn overlay_branding_settings(
    defaults: BrandingResponse,
    settings: &HashMap<String, serde_json::Value>,
) -> BrandingResponse {
    BrandingResponse {
        title: read_optional_string_setting(settings, "branding.title", defaults.title),
        subtitle: read_optional_string_setting(settings, "branding.subtitle", defaults.subtitle),
    }
}

fn read_optional_string_setting(
    settings: &HashMap<String, serde_json::Value>,
    key: &str,
    fallback: Option<String>,
) -> Option<String> {
    match settings.get(key) {
        Some(serde_json::Value::String(value)) => Some(value.clone()),
        Some(serde_json::Value::Null) => None,
        Some(other) => {
            tracing::warn!(setting = key, value = ?other, "ignoring non-string app setting");
            fallback
        }
        None => fallback,
    }
}

async fn list_tables(
    Extension(mode): Extension<SharedAppMode>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    let schemas = state.config.database.effective_schemas();
    let all_tables = state.db.list_tables(&schemas).await?;
    let tables: Vec<serde_json::Value> = all_tables
        .into_iter()
        .filter(|t| state.config.tables.allows(&t.schema, &t.name))
        .map(|t| {
            let display = display_name_table(&t.schema, &t.name, &state.config.display);
            serde_json::json!({
                "schema": t.schema,
                "name": t.name,
                "display_name": display,
                "row_count_estimate": t.row_count_estimate,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "tables": tables })))
}

async fn get_columns(
    Extension(mode): Extension<SharedAppMode>,
    Path((schema, table)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }
    let raw_columns = state
        .db
        .get_columns(&schema, &table)
        .await
        .map_err(|e| map_table_query_error(e, &table))?;
    if raw_columns.is_empty() {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }
    let columns: Vec<serde_json::Value> = raw_columns
        .into_iter()
        .map(|c| {
            let display = display_name_column(&schema, &table, &c.name, &state.config.display);
            serde_json::json!({
                "name": c.name,
                "display_name": display,
                "data_type": c.data_type,
                "display_type": c.display_type,
                "is_nullable": c.is_nullable,
                "is_primary_key": c.is_primary_key,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "columns": columns })))
}

#[derive(Deserialize)]
struct RowsQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
    sort: Option<String>,
    search: Option<String>,
}

fn default_page() -> u32 {
    1
}
fn default_page_size() -> u32 {
    50
}

const MAX_PAGE_SIZE: u32 = 1000;

/// Extract per-column filters from query params with the `filter.` prefix.
/// e.g. `?filter.vehicle_id=ADT3&filter.supervisor=Local` → {"vehicle_id": "ADT3", "supervisor": "Local"}
fn parse_filters(all_params: &HashMap<String, String>) -> HashMap<String, String> {
    all_params
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix("filter.")
                .map(|col| (col.to_string(), v.clone()))
        })
        .collect()
}

/// Reject the pre-PR-#72 `sort_column` / `sort_direction` query params with a clear
/// deprecation message so saved bookmarks fail loudly instead of silently returning
/// unsorted data.
fn reject_legacy_sort_params(all_params: &HashMap<String, String>) -> Result<(), AppError> {
    if all_params.contains_key("sort_column") || all_params.contains_key("sort_direction") {
        return Err(AppError::bad_request(
            "`sort_column` and `sort_direction` are no longer supported. \
             Use `?sort=<column>:asc` (comma-separated for multi-column).",
        ));
    }
    Ok(())
}

fn trim_ascii_whitespace(value: &str) -> &str {
    value.trim_matches(|c: char| c.is_ascii_whitespace())
}

/// Cap user-controlled strings before echoing them in 400 error messages.
/// Prevents unbounded payload reflection in error responses and log lines.
fn truncate_for_error(value: &str) -> String {
    const MAX: usize = 64;
    if value.chars().count() <= MAX {
        return value.to_string();
    }
    let truncated: String = value.chars().take(MAX).collect();
    format!("{truncated}…")
}

fn parse_sort_param(sort: Option<&str>, columns: &[ColumnInfo]) -> anyhow::Result<Vec<SortEntry>> {
    let Some(sort) = sort else {
        return Ok(Vec::new());
    };

    if sort.is_empty() {
        return Ok(Vec::new());
    }

    let valid_columns: std::collections::HashSet<&str> =
        columns.iter().map(|column| column.name.as_str()).collect();
    let mut seen_columns: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut parsed = Vec::new();

    for segment in sort.split(',') {
        let segment = trim_ascii_whitespace(segment);
        if segment.is_empty() {
            return Err(ValidationError("Malformed sort segment: empty segment".into()).into());
        }

        let safe_segment = truncate_for_error(segment);
        let (column_part, direction_part) = segment
            .split_once(':')
            .ok_or_else(|| ValidationError(format!("Malformed sort segment: {safe_segment}")))?;
        let column = trim_ascii_whitespace(column_part);
        let direction = trim_ascii_whitespace(direction_part);

        if column.is_empty() || direction.is_empty() {
            return Err(ValidationError(format!("Malformed sort segment: {safe_segment}")).into());
        }
        if !is_valid_identifier(column) {
            return Err(ValidationError(format!(
                "Invalid sort column name in segment: {safe_segment}"
            ))
            .into());
        }
        if !valid_columns.contains(column) {
            return Err(
                ValidationError(format!("Unknown sort column in segment: {safe_segment}")).into(),
            );
        }
        let direction = if direction.eq_ignore_ascii_case("asc") {
            SortDirection::Asc
        } else if direction.eq_ignore_ascii_case("desc") {
            SortDirection::Desc
        } else {
            return Err(ValidationError(format!(
                "Invalid sort direction in segment: {safe_segment}"
            ))
            .into());
        };
        if !seen_columns.insert(column.to_string()) {
            return Err(ValidationError(format!(
                "Duplicate sort column in segment: {safe_segment}"
            ))
            .into());
        }

        parsed.push(SortEntry {
            column: column.to_string(),
            direction,
        });
    }

    Ok(parsed)
}

async fn load_table_columns(
    state: &AppState,
    schema: &str,
    table: &str,
) -> Result<Vec<ColumnInfo>, AppError> {
    let columns = state
        .db
        .get_columns(schema, table)
        .await
        .map_err(|e| map_table_query_error(e, table))?;
    if columns.is_empty() {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }
    Ok(columns)
}

async fn get_rows(
    Extension(mode): Extension<SharedAppMode>,
    Path((schema, table)): Path<(String, String)>,
    Query(params): Query<RowsQuery>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }
    let columns = load_table_columns(&state, &schema, &table).await?;
    reject_legacy_sort_params(&all_params)?;
    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, MAX_PAGE_SIZE);
    let filters = parse_filters(&all_params);
    let sort = parse_sort_param(params.sort.as_deref(), &columns)?;
    let result = state
        .db
        .query_rows(&RowQueryParams {
            schema: &schema,
            table: &table,
            page,
            page_size,
            sort: &sort,
            search: params.search.as_deref(),
            filters: &filters,
        })
        .await
        .map_err(|e| map_table_query_error(e, &table))?;
    Ok(Json(serde_json::json!(result)))
}

async fn export_csv(
    Extension(mode): Extension<SharedAppMode>,
    Path((schema, table)): Path<(String, String)>,
    Query(params): Query<RowsQuery>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }

    let pg_pool = state
        .db
        .pg_pool()
        .ok_or_else(|| AppError::bad_request("CSV export not supported for this database type"))?
        .clone();

    reject_legacy_sort_params(&all_params)?;
    let filters = parse_filters(&all_params);
    let columns = load_table_columns(&state, &schema, &table).await?;
    let sort = parse_sort_param(params.sort.as_deref(), &columns)?;

    // Fetch columns eagerly so we can build headers before spawning
    let display_headers: Vec<String> = columns
        .iter()
        .map(|c| display_name_column(&schema, &table, &c.name, &state.config.display))
        .collect();

    let display_table = display_name_table(&schema, &table, &state.config.display)
        .replace(' ', "_")
        .to_lowercase();
    let sanitized: String = display_table
        .replace(['"', '\\', ';', '\r', '\n'], "")
        .chars()
        .filter(|c| c.is_ascii())
        .collect();
    let filename = if sanitized.is_empty() {
        "export.csv".to_string()
    } else {
        format!("{sanitized}.csv")
    };

    // Owned values for the spawned task
    let search = params.search.clone();
    let schema_owned = schema.clone();
    let sort_owned = sort;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(32);

    tokio::spawn(async move {
        use futures::StreamExt;

        // Write CSV header
        let mut header_buf = Vec::new();
        {
            let mut wtr = csv::Writer::from_writer(&mut header_buf);
            if wtr.write_record(&display_headers).is_err() {
                return;
            }
            if wtr.flush().is_err() {
                return;
            }
        }
        if tx.send(Ok(bytes::Bytes::from(header_buf))).await.is_err() {
            return;
        }

        // Build export params with owned data
        let export_params = ExportQueryParams {
            schema: &schema_owned,
            table: &table,
            sort: &sort_owned,
            search: search.as_deref(),
            filters: &filters,
        };

        let stream_result = crate::db::postgres::export_rows_stream(&pg_pool, &export_params).await;

        let (_cols, mut row_stream) = match stream_result {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "CSV export: failed to open row stream");
                return;
            }
        };

        let mut wtr = csv::Writer::from_writer(Vec::with_capacity(8192));
        let mut batch_count = 0u32;
        let mut stream_error = false;

        'rows: while let Some(row_result) = row_stream.next().await {
            match row_result {
                Ok(row) => {
                    let fields: Vec<String> = columns
                        .iter()
                        .map(|col| pg_value_to_csv_string(&row, &col.name, &col.data_type))
                        .collect();

                    if wtr.write_record(&fields).is_err() {
                        stream_error = true;
                        break;
                    }
                    batch_count += 1;

                    if batch_count >= 100 {
                        if wtr.flush().is_err() {
                            stream_error = true;
                            break 'rows;
                        }
                        let chunk = wtr.into_inner().unwrap_or_default();
                        if tx.send(Ok(bytes::Bytes::from(chunk))).await.is_err() {
                            return; // Client disconnected — no error to signal
                        }
                        wtr = csv::Writer::from_writer(Vec::with_capacity(8192));
                        batch_count = 0;
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "CSV export: row stream error mid-export");
                    stream_error = true;
                    break;
                }
            }
        }

        // Flush any remaining buffered rows
        if !stream_error && wtr.flush().is_ok() {
            let remaining = wtr.into_inner().unwrap_or_default();
            if !remaining.is_empty() {
                let _ = tx.send(Ok(bytes::Bytes::from(remaining))).await;
            }
        }

        if stream_error {
            tracing::warn!("CSV export: stream ended with error, output may be truncated");
            let _ = tx
                .send(Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "CSV export interrupted: not all rows were exported",
                )))
                .await;
        }
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let body = Body::from_stream(stream);

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    ))
}

fn pg_value_to_csv_string(row: &sqlx::postgres::PgRow, col: &str, data_type: &str) -> String {
    use sqlx::Row;
    match data_type {
        "smallint" => row
            .try_get::<i16, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "integer" => row
            .try_get::<i32, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "bigint" => row
            .try_get::<i64, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "real" => row
            .try_get::<f32, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "double precision" => row
            .try_get::<f64, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "numeric" => row
            .try_get::<rust_decimal::Decimal, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "boolean" => row
            .try_get::<bool, _>(col)
            .map(|v| if v { "Yes" } else { "No" }.to_string())
            .unwrap_or_default(),
        "json" | "jsonb" => row
            .try_get::<serde_json::Value, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "timestamp without time zone" => row
            .try_get::<chrono::NaiveDateTime, _>(col)
            .map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default(),
        "timestamp with time zone" => row
            .try_get::<chrono::DateTime<chrono::Utc>, _>(col)
            .map(|v| v.to_rfc3339())
            .unwrap_or_default(),
        "date" => row
            .try_get::<chrono::NaiveDate, _>(col)
            .map(|v| v.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        "time without time zone" | "time with time zone" => row
            .try_get::<chrono::NaiveTime, _>(col)
            .map(|v| v.format("%H:%M:%S").to_string())
            .unwrap_or_default(),
        "uuid" => row
            .try_get::<uuid::Uuid, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        _ => row.try_get::<String, _>(col).unwrap_or_default(),
    }
}

// Simple error type for API responses
pub(super) struct AppError {
    status: axum::http::StatusCode,
    message: String,
}

impl AppError {
    pub(super) fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub(super) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub(super) fn internal(message: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    pub(super) fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        // Map ValidationError from the DB layer to HTTP 400
        if let Some(ve) = err.downcast_ref::<ValidationError>() {
            return Self::bad_request(ve.0.clone());
        }
        // Log the real error for debugging; return a generic message to the client
        tracing::error!(error = %err, "Internal server error");
        Self {
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal server error".to_string(),
        }
    }
}

/// Map a DB query error to an AppError, converting PostgreSQL "undefined_table"
/// (error code 42P01) into a 404 that includes the table name.
fn map_table_query_error(err: anyhow::Error, table: &str) -> AppError {
    if let Some(sqlx::Error::Database(db_err)) = err.downcast_ref::<sqlx::Error>()
        && db_err.code().as_deref() == Some("42P01")
    {
        return AppError::not_found(format!("Table '{table}' not found"));
    }
    AppError::from(err)
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::json!({
            "error": self.message,
        });
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request};
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use crate::app_mode::initial_mode;
    use crate::config::{AppConfig, DatabaseConfig, ServerConfig, TablesConfig};
    use crate::store::testutil::ephemeral_store;

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
    fn display_config_response_serializes_with_branding() {
        let response = DisplayConfigResponse {
            branding: BrandingResponse {
                title: Some("My Database".into()),
                subtitle: Some("Fleet Telemetry".into()),
            },
            tables: HashMap::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["branding"]["title"], "My Database");
        assert_eq!(json["branding"]["subtitle"], "Fleet Telemetry");
        assert!(json["tables"].as_object().unwrap().is_empty());
    }

    #[test]
    fn display_config_response_serializes_null_branding() {
        let response = DisplayConfigResponse {
            branding: BrandingResponse {
                title: None,
                subtitle: None,
            },
            tables: HashMap::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert!(json["branding"]["title"].is_null());
        assert!(json["branding"]["subtitle"].is_null());
    }

    #[test]
    fn display_config_response_serializes_table_with_columns() {
        let mut columns = HashMap::new();
        columns.insert(
            "posn_lat".into(),
            ColumnDisplayConfig {
                display_name: "Latitude".into(),
            },
        );
        columns.insert(
            "supervisor_id".into(),
            ColumnDisplayConfig {
                display_name: "Supervisor".into(),
            },
        );

        let mut tables = HashMap::new();
        tables.insert(
            "vehicles_log".into(),
            TableDisplayConfig {
                display_name: "Fleet Log".into(),
                columns,
            },
        );

        let response = DisplayConfigResponse {
            branding: BrandingResponse {
                title: Some("My Database".into()),
                subtitle: None,
            },
            tables,
        };

        let json = serde_json::to_value(&response).unwrap();
        let vl = &json["tables"]["vehicles_log"];
        assert_eq!(vl["display_name"], "Fleet Log");
        assert_eq!(vl["columns"]["posn_lat"]["display_name"], "Latitude");
        assert_eq!(vl["columns"]["supervisor_id"]["display_name"], "Supervisor");
    }

    #[test]
    fn overlay_branding_settings_prefers_saved_values() {
        let settings = HashMap::from([
            (
                "branding.title".to_string(),
                serde_json::Value::String("Fleet DB".into()),
            ),
            (
                "branding.subtitle".to_string(),
                serde_json::Value::String("Operations".into()),
            ),
        ]);

        let branding = overlay_branding_settings(
            BrandingResponse {
                title: Some("SeeKi".into()),
                subtitle: Some("Viewer".into()),
            },
            &settings,
        );

        assert_eq!(branding.title.as_deref(), Some("Fleet DB"));
        assert_eq!(branding.subtitle.as_deref(), Some("Operations"));
    }

    #[test]
    fn overlay_branding_settings_falls_back_for_invalid_value_types() {
        let settings = HashMap::from([("branding.title".to_string(), serde_json::json!(false))]);

        let branding = overlay_branding_settings(
            BrandingResponse {
                title: Some("SeeKi".into()),
                subtitle: None,
            },
            &settings,
        );

        assert_eq!(branding.title.as_deref(), Some("SeeKi"));
    }

    #[tokio::test]
    async fn connection_status_route_returns_sanitized_runtime_metadata() {
        let (store, _dir) = ephemeral_store().await;
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://user:pass@db.internal:5544/fleet")
            .unwrap();
        let config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 3141,
            },
            database: DatabaseConfig {
                url: "postgres://user:pass@db.internal:5544/fleet".into(),
                kind: crate::config::DatabaseKind::Postgres,
                max_connections: 5,
                schemas: Some(vec!["public".into(), "reporting".into()]),
            },
            tables: TablesConfig::default(),
            display: crate::config::DisplayConfig::default(),
            branding: crate::config::BrandingConfig::default(),
            ssh: Some(crate::config::SshConfig {
                host: "jumpbox.internal".into(),
                port: 22,
                username: "tester".into(),
                auth_method: crate::config::SshAuthMethod::Agent,
                key_path: None,
                known_hosts: crate::config::KnownHostsPolicy::Add,
            }),
        };
        let mode = initial_mode(Some(crate::AppState {
            db: crate::db::DatabasePool::Postgres(pool, None),
            config,
        }));
        let app: Router = router(mode, store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/connection-status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["database_kind"], "postgres");
        assert_eq!(json["host"], "db.internal");
        assert_eq!(json["port"], 5544);
        assert_eq!(json["database"], "fleet");
        assert_eq!(json["schemas"], serde_json::json!(["public", "reporting"]));
        assert_eq!(json["ssh_enabled"], true);
        assert_eq!(json["ssh_connected"], false);
        assert!(json.get("url").is_none());
        assert!(json.get("username").is_none());
        assert!(json.get("password").is_none());
    }

    #[test]
    fn version_response_serializes_embedded_build_metadata() {
        let response = serde_json::to_value(VersionResponse {
            version: env!("SEEKI_VERSION"),
            commit: env!("SEEKI_GIT_COMMIT"),
            built_at: env!("SEEKI_BUILT_AT"),
        })
        .unwrap();

        assert!(response["version"].as_str().unwrap().len() >= 1);
        assert!(response["commit"].as_str().unwrap().len() >= 1);
        assert!(response["built_at"].as_str().unwrap().len() >= 1);
    }

    #[test]
    fn parse_filters_extracts_filter_prefixed_params() {
        let mut params = HashMap::new();
        params.insert("filter.vehicle_id".into(), "ADT3".into());
        params.insert("filter.supervisor".into(), "Local".into());
        params.insert("page".into(), "1".into());
        params.insert("search".into(), "test".into());

        let filters = parse_filters(&params);
        assert_eq!(filters.len(), 2);
        assert_eq!(filters["vehicle_id"], "ADT3");
        assert_eq!(filters["supervisor"], "Local");
    }

    #[test]
    fn parse_filters_returns_empty_when_no_filters() {
        let mut params = HashMap::new();
        params.insert("page".into(), "1".into());
        params.insert("page_size".into(), "50".into());

        let filters = parse_filters(&params);
        assert!(filters.is_empty());
    }

    #[test]
    fn parse_filters_handles_empty_params() {
        let params = HashMap::new();
        let filters = parse_filters(&params);
        assert!(filters.is_empty());
    }

    #[test]
    fn parse_filters_preserves_filter_value_exactly() {
        let mut params = HashMap::new();
        params.insert("filter.name".into(), "Hello World".into());

        let filters = parse_filters(&params);
        assert_eq!(filters["name"], "Hello World");
    }

    #[test]
    fn parse_sort_param_accepts_multi_column_sort() {
        let sort = parse_sort_param(
            Some(" vehicle_id : asc , logged_at : DESC "),
            &test_columns(),
        )
        .unwrap();

        assert_eq!(
            sort,
            vec![
                SortEntry {
                    column: "vehicle_id".into(),
                    direction: SortDirection::Asc,
                },
                SortEntry {
                    column: "logged_at".into(),
                    direction: SortDirection::Desc,
                },
            ]
        );
    }

    #[test]
    fn parse_sort_param_treats_empty_sort_as_unsorted() {
        assert!(parse_sort_param(None, &test_columns()).unwrap().is_empty());
        assert!(
            parse_sort_param(Some(""), &test_columns())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn parse_sort_param_rejects_malformed_direction() {
        let err = parse_sort_param(Some("id:sideways"), &test_columns()).unwrap_err();
        assert!(err.to_string().contains("Invalid sort direction"));
        assert!(err.to_string().contains("id:sideways"));
    }

    #[test]
    fn parse_sort_param_rejects_malformed_column() {
        let err = parse_sort_param(Some("id;drop table:asc"), &test_columns()).unwrap_err();
        assert!(err.to_string().contains("Invalid sort column name"));
    }

    #[test]
    fn parse_sort_param_rejects_duplicate_column() {
        let err = parse_sort_param(Some("id:asc,id:desc"), &test_columns()).unwrap_err();
        assert!(err.to_string().contains("Duplicate sort column"));
    }

    #[test]
    fn parse_sort_param_rejects_trailing_comma() {
        let err = parse_sort_param(Some("id:asc,"), &test_columns()).unwrap_err();
        assert!(err.to_string().contains("empty segment"));
    }

    #[test]
    fn parse_sort_param_truncates_long_segment_in_error_message() {
        let long_column = "a".repeat(500);
        let sort = format!("{long_column}:asc");
        let err = parse_sort_param(Some(&sort), &test_columns()).unwrap_err();
        let msg = err.to_string();
        // Error should contain truncated value (64 chars + ellipsis), not the full 500-char input
        assert!(msg.contains("…"), "expected ellipsis in: {msg}");
        assert!(
            msg.len() < 200,
            "error message should be capped, got {} chars",
            msg.len()
        );
    }

    #[test]
    fn reject_legacy_sort_params_flags_sort_column() {
        let mut params = HashMap::new();
        params.insert("sort_column".into(), "id".into());
        let err = reject_legacy_sort_params(&params).unwrap_err();
        assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
        assert!(err.message.contains("sort_column"));
    }

    #[test]
    fn reject_legacy_sort_params_flags_sort_direction() {
        let mut params = HashMap::new();
        params.insert("sort_direction".into(), "asc".into());
        assert!(reject_legacy_sort_params(&params).is_err());
    }

    #[test]
    fn reject_legacy_sort_params_passes_modern_params() {
        let mut params = HashMap::new();
        params.insert("sort".into(), "id:asc".into());
        params.insert("page".into(), "1".into());
        assert!(reject_legacy_sort_params(&params).is_ok());
    }

    #[test]
    fn validation_error_maps_to_bad_request() {
        let err = anyhow::Error::from(ValidationError("bad column".into()));
        let app_err = AppError::from(err);
        assert_eq!(app_err.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(app_err.message, "bad column");
    }

    #[test]
    fn generic_error_maps_to_internal_server_error() {
        let err = anyhow::anyhow!("something broke");
        let app_err = AppError::from(err);
        assert_eq!(
            app_err.status,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn csv_header_uses_display_names() {
        use crate::config::DisplayConfig;
        use crate::db::ColumnInfo;

        let columns = [
            ColumnInfo {
                name: "supervisor_id".into(),
                data_type: "integer".into(),
                display_type: "Number".into(),
                is_nullable: false,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "posn_lat".into(),
                data_type: "double precision".into(),
                display_type: "Decimal".into(),
                is_nullable: true,
                is_primary_key: false,
            },
        ];

        let mut col_overrides = HashMap::new();
        col_overrides.insert("posn_lat".to_string(), "Latitude".to_string());
        let mut columns_map = HashMap::new();
        columns_map.insert("vehicles_log".to_string(), col_overrides);

        let config = DisplayConfig {
            tables: HashMap::new(),
            columns: columns_map,
        };

        let headers: Vec<String> = columns
            .iter()
            .map(|c| display_name_column("public", "vehicles_log", &c.name, &config))
            .collect();

        assert_eq!(headers, vec!["Supervisor", "Latitude"]);
    }

    #[test]
    fn csv_writes_valid_output() {
        let headers = vec!["Name", "Age", "Active"];
        let rows = vec![vec!["Alice", "30", "Yes"], vec!["Bob", "25", "No"]];

        let mut buf = Vec::new();
        {
            let mut wtr = csv::Writer::from_writer(&mut buf);
            wtr.write_record(&headers).unwrap();
            for row in &rows {
                wtr.write_record(row).unwrap();
            }
            wtr.flush().unwrap();
        }

        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("Name,Age,Active"));
        assert!(output.contains("Alice,30,Yes"));
        assert!(output.contains("Bob,25,No"));
    }

    #[test]
    fn csv_export_filename_uses_display_name() {
        use crate::config::DisplayConfig;

        let mut tables = HashMap::new();
        tables.insert("vehicles_log".to_string(), "Fleet Log".to_string());
        let config = DisplayConfig {
            tables,
            columns: HashMap::new(),
        };

        let display = display_name_table("public", "vehicles_log", &config)
            .replace(' ', "_")
            .to_lowercase();
        assert_eq!(display, "fleet_log");
        assert_eq!(format!("{display}.csv"), "fleet_log.csv");
    }
}
