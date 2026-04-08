use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;

use crate::AppState;
use crate::config::{display_name_column, display_name_table};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tables", get(list_tables))
        .route("/tables/{table}/columns", get(get_columns))
        .route("/tables/{table}/rows", get(get_rows))
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
struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::json!({
            "error": self.0.to_string(),
        });
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(body),
        )
            .into_response()
    }
}
