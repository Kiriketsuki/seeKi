pub mod setup;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Path, Query},
    http::header,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::app_mode::{AppMode, SharedAppMode};
use crate::config::{display_name_column, display_name_table};
use crate::db::{ExportQueryParams, RowQueryParams, ValidationError};

pub fn router(mode: SharedAppMode) -> Router {
    Router::new()
        .route("/tables", get(list_tables))
        .route("/tables/{table}/columns", get(get_columns))
        .route("/tables/{table}/rows", get(get_rows))
        .route("/config/display", get(get_display_config))
        .route("/export/{table}/csv", get(export_csv))
        .route("/status", get(status))
        .route("/setup/test-connection", post(setup::test_connection))
        .route("/setup/save", post(setup::save_config))
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
) -> Result<Json<DisplayConfigResponse>, AppError> {
    let state = require_state(&mode).await?;
    let all_tables = state.db.list_tables().await?;
    let allowed_tables: Vec<_> = all_tables
        .into_iter()
        .filter(|t| state.config.tables.allows(&t.name))
        .collect();

    let table_names: Vec<&str> = allowed_tables.iter().map(|t| t.name.as_str()).collect();
    let all_columns = state.db.get_columns_bulk(&table_names).await?;

    let mut tables = HashMap::new();
    for table in &allowed_tables {
        let columns: HashMap<String, ColumnDisplayConfig> = all_columns
            .get(&table.name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|c| {
                let display = display_name_column(&table.name, &c.name, &state.config.display);
                (c.name, ColumnDisplayConfig { display_name: display })
            })
            .collect();

        let display = display_name_table(&table.name, &state.config.display);
        tables.insert(
            table.name.clone(),
            TableDisplayConfig {
                display_name: display,
                columns,
            },
        );
    }

    Ok(Json(DisplayConfigResponse {
        branding: BrandingResponse {
            title: state.config.branding.title.clone(),
            subtitle: state.config.branding.subtitle.clone(),
        },
        tables,
    }))
}

async fn list_tables(
    Extension(mode): Extension<SharedAppMode>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    let all_tables = state.db.list_tables().await?;
    let tables: Vec<serde_json::Value> = all_tables
        .into_iter()
        .filter(|t| state.config.tables.allows(&t.name))
        .map(|t| {
            let display = display_name_table(&t.name, &state.config.display);
            serde_json::json!({
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
    Path(table): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&table) {
        return Err(AppError::not_found(format!("Table '{table}' not found")));
    }
    let raw_columns = state.db.get_columns(&table).await
        .map_err(|e| map_table_query_error(e, &table))?;
    if raw_columns.is_empty() {
        return Err(AppError::not_found(format!("Table '{table}' not found")));
    }
    let columns: Vec<serde_json::Value> = raw_columns
        .into_iter()
        .map(|c| {
            let display = display_name_column(&table, &c.name, &state.config.display);
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
    sort_column: Option<String>,
    sort_direction: Option<String>,
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

async fn get_rows(
    Extension(mode): Extension<SharedAppMode>,
    Path(table): Path<String>,
    Query(params): Query<RowsQuery>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&table) {
        return Err(AppError::not_found(format!("Table '{table}' not found")));
    }
    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, MAX_PAGE_SIZE);
    let filters = parse_filters(&all_params);
    let result = state
        .db
        .query_rows(&RowQueryParams {
            table: &table,
            page,
            page_size,
            sort_column: params.sort_column.as_deref(),
            sort_direction: params.sort_direction.as_deref(),
            search: params.search.as_deref(),
            filters: &filters,
        })
        .await
        .map_err(|e| map_table_query_error(e, &table))?;
    Ok(Json(serde_json::json!(result)))
}

async fn export_csv(
    Extension(mode): Extension<SharedAppMode>,
    Path(table): Path<String>,
    Query(params): Query<RowsQuery>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&table) {
        return Err(AppError::not_found(format!("Table '{table}' not found")));
    }

    let pg_pool = state
        .db
        .pg_pool()
        .ok_or_else(|| AppError::bad_request("CSV export not supported for this database type"))?
        .clone();

    let filters = parse_filters(&all_params);

    // Fetch columns eagerly so we can build headers before spawning
    let columns = state.db.get_columns(&table).await?;
    let display_headers: Vec<String> = columns
        .iter()
        .map(|c| display_name_column(&table, &c.name, &state.config.display))
        .collect();

    let display_table = display_name_table(&table, &state.config.display)
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
    let sort_column = params.sort_column.clone();
    let sort_direction = params.sort_direction.clone();
    let search = params.search.clone();

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
            table: &table,
            sort_column: sort_column.as_deref(),
            sort_direction: sort_direction.as_deref(),
            search: search.as_deref(),
            filters: &filters,
        };

        let stream_result =
            crate::db::postgres::export_rows_stream(&pg_pool, &export_params).await;

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
        if !stream_error
            && wtr.flush().is_ok()
        {
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

fn pg_value_to_csv_string(
    row: &sqlx::postgres::PgRow,
    col: &str,
    data_type: &str,
) -> String {
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
        _ => row
            .try_get::<String, _>(col)
            .unwrap_or_default(),
    }
}

// Simple error type for API responses
struct AppError {
    status: axum::http::StatusCode,
    message: String,
}

impl AppError {
    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn service_unavailable(message: impl Into<String>) -> Self {
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
    if let Some(sqlx_err) = err.downcast_ref::<sqlx::Error>() {
        if let sqlx::Error::Database(db_err) = sqlx_err {
            if db_err.code().as_deref() == Some("42P01") {
                return AppError::not_found(format!("Table '{table}' not found"));
            }
        }
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
        assert_eq!(app_err.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn csv_header_uses_display_names() {
        use crate::config::DisplayConfig;
        use crate::db::ColumnInfo;

        let columns = vec![
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
            .map(|c| display_name_column("vehicles_log", &c.name, &config))
            .collect();

        assert_eq!(headers, vec!["Supervisor", "Latitude"]);
    }

    #[test]
    fn csv_writes_valid_output() {
        let headers = vec!["Name", "Age", "Active"];
        let rows = vec![
            vec!["Alice", "30", "Yes"],
            vec!["Bob", "25", "No"],
        ];

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

        let display = display_name_table("vehicles_log", &config)
            .replace(' ', "_")
            .to_lowercase();
        assert_eq!(display, "fleet_log");
        assert_eq!(format!("{display}.csv"), "fleet_log.csv");
    }
}
