pub mod setup;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::config::{display_name_column, display_name_table};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tables", get(list_tables))
        .route("/tables/{table}/columns", get(get_columns))
        .route("/tables/{table}/rows", get(get_rows))
        .route("/config/display", get(get_display_config))
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
    State(state): State<Arc<AppState>>,
) -> Result<Json<DisplayConfigResponse>, AppError> {
    let all_tables = state.db.list_tables().await?;
    let allowed_tables: Vec<_> = all_tables
        .into_iter()
        .filter(|t| state.config.tables.allows(&t.name))
        .collect();

    let mut tables = HashMap::new();
    for table in &allowed_tables {
        let columns_info = state.db.get_columns(&table.name).await?;
        let columns: HashMap<String, ColumnDisplayConfig> = columns_info
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
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
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
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !state.config.tables.allows(&table) {
        return Err(AppError::not_found(format!("Table '{table}' not found")));
    }
    let raw_columns = state.db.get_columns(&table).await?;
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

async fn get_rows(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
    Query(params): Query<RowsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !state.config.tables.allows(&table) {
        return Err(AppError::not_found(format!("Table '{table}' not found")));
    }
    let result = state
        .db
        .query_rows(
            &table,
            params.page,
            params.page_size,
            params.sort_column.as_deref(),
            params.sort_direction.as_deref(),
            params.search.as_deref(),
        )
        .await?;
    Ok(Json(serde_json::json!(result)))
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
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
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
                title: Some("AutoConnect".into()),
                subtitle: Some("Fleet Telemetry".into()),
            },
            tables: HashMap::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["branding"]["title"], "AutoConnect");
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
                title: Some("AutoConnect".into()),
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
}
