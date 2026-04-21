use std::collections::HashMap;

use axum::{
    Extension, Json, Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
};
use serde::Deserialize;
use serde_json::Value;

use crate::app_mode::{AppMode, SharedAppMode};
use crate::store::{Store, connection_id, presets, settings, ui_state};

#[cfg(test)]
mod tests;

pub fn router() -> Router {
    Router::new()
        // Settings — available in both Setup and Normal mode
        .route("/settings", get(get_settings).post(set_settings))
        // Last-used table state — Normal mode only
        .route(
            "/presets/last-used/{schema}/{table}",
            get(get_last_used).post(set_last_used),
        )
        // Clear all browsing state — Normal mode only
        .route("/presets", delete(clear_all_presets))
        // Named sort presets — Normal mode only
        .route(
            "/presets/sort/{schema}/{table}",
            get(list_sort_presets).post(save_sort_preset),
        )
        .route(
            "/presets/sort/{schema}/{table}/{name}",
            delete(delete_sort_preset),
        )
        // Named filter presets — Normal mode only
        .route(
            "/presets/filter/{schema}/{table}",
            get(list_filter_presets).post(save_filter_preset),
        )
        .route(
            "/presets/filter/{schema}/{table}/{name}",
            delete(delete_filter_preset),
        )
        // UI state — Normal mode only (schema ready; frontend migration deferred)
        .route("/ui-state/{key}", get(get_ui_state).post(set_ui_state))
}

// ── Settings ──────────────────────────────────────────────────────────────────

async fn get_settings(
    Extension(store): Extension<Store>,
) -> Result<Json<HashMap<String, Value>>, Err> {
    let pairs = settings::get_all(store.pool())
        .await
        .map_err(Err::internal)?;
    Ok(Json(pairs.into_iter().collect()))
}

async fn set_settings(
    Extension(store): Extension<Store>,
    Json(entries): Json<HashMap<String, Value>>,
) -> Result<StatusCode, Err> {
    for (k, v) in &entries {
        if k.len() > MAX_UI_STATE_KEY_LEN {
            return Err(Err::bad_request("setting key exceeds maximum length"));
        }
        let json = serde_json::to_string(v).map_err(|e| Err::internal(anyhow::Error::from(e)))?;
        if json.len() > MAX_VALUE_BYTES {
            return Err(Err::bad_request("value exceeds maximum size"));
        }
        validate_known_setting(k, v)?;
    }
    let pairs: Vec<(&str, &Value)> = entries.iter().map(|(k, v)| (k.as_str(), v)).collect();
    settings::set_many(store.pool(), &pairs)
        .await
        .map_err(Err::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

fn validate_known_setting(key: &str, value: &Value) -> Result<(), Err> {
    match key {
        "branding.title" => {
            let Some(title) = value.as_str() else {
                return Err(Err::bad_request("branding.title must be a string"));
            };
            if title.trim().is_empty() {
                return Err(Err::bad_request("branding.title must not be empty"));
            }
        }
        "branding.subtitle" => {
            if !(value.is_string() || value.is_null()) {
                return Err(Err::bad_request(
                    "branding.subtitle must be a string or null",
                ));
            }
        }
        "appearance.date_format" => {
            let Some(format) = value.as_str() else {
                return Err(Err::bad_request("appearance.date_format must be a string"));
            };
            if !matches!(
                format,
                "system" | "YYYY-MM-DD" | "DD/MM/YYYY" | "MM/DD/YYYY"
            ) {
                return Err(Err::bad_request("appearance.date_format is not supported"));
            }
        }
        "appearance.row_density" => {
            let Some(density) = value.as_str() else {
                return Err(Err::bad_request("appearance.row_density must be a string"));
            };
            if !matches!(density, "comfortable" | "compact") {
                return Err(Err::bad_request("appearance.row_density is not supported"));
            }
        }
        "page_size" => {
            let Some(n) = value.as_u64() else {
                return Err(Err::bad_request("page_size must be a number"));
            };
            if !matches!(n, 50 | 100 | 250 | 500) {
                return Err(Err::bad_request("page_size must be 50, 100, 250, or 500"));
            }
        }
        "data.pagination_mode" => {
            let Some(mode) = value.as_str() else {
                return Err(Err::bad_request("data.pagination_mode must be a string"));
            };
            if !matches!(mode, "infinite" | "paged") {
                return Err(Err::bad_request(
                    "data.pagination_mode must be \"infinite\" or \"paged\"",
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

// ── Last-used table state ─────────────────────────────────────────────────────

async fn get_last_used(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
) -> Result<Json<Value>, Err> {
    let conn_id = require_conn_id(&mode).await?;
    match presets::get_last_used(store.pool(), &conn_id, &schema, &table).await? {
        Some(state) => Ok(Json(serde_json::to_value(state).unwrap())),
        None => Err(Err::not_found("no last-used state for this table")),
    }
}

async fn set_last_used(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
    Json(body): Json<presets::LastUsedState>,
) -> Result<StatusCode, Err> {
    let sort_json = serde_json::to_string(&body.sort_columns)
        .map_err(|e| Err::internal(anyhow::Error::from(e)))?;
    if sort_json.len() > MAX_VALUE_BYTES {
        return Err(Err::bad_request("value exceeds maximum size"));
    }
    let filter_json =
        serde_json::to_string(&body.filters).map_err(|e| Err::internal(anyhow::Error::from(e)))?;
    if filter_json.len() > MAX_VALUE_BYTES {
        return Err(Err::bad_request("value exceeds maximum size"));
    }
    let search_term_json = serde_json::to_string(&body.search_term)
        .map_err(|e| Err::internal(anyhow::Error::from(e)))?;
    if search_term_json.len() > MAX_VALUE_BYTES {
        return Err(Err::bad_request("value exceeds maximum size"));
    }
    let conn_id = require_conn_id(&mode).await?;
    presets::set_last_used(store.pool(), &conn_id, &schema, &table, &body)
        .await
        .map_err(Err::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Clear all browsing state ──────────────────────────────────────────────────

async fn clear_all_presets(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
) -> Result<StatusCode, Err> {
    let conn_id = require_conn_id(&mode).await?;
    presets::clear_all(store.pool(), &conn_id)
        .await
        .map_err(Err::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Sort presets ──────────────────────────────────────────────────────────────

async fn list_sort_presets(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
) -> Result<Json<Value>, Err> {
    let conn_id = require_conn_id(&mode).await?;
    let items = presets::list_sort_presets(store.pool(), &conn_id, &schema, &table).await?;
    Ok(Json(
        serde_json::to_value(items).map_err(|e| Err::internal(anyhow::Error::from(e)))?,
    ))
}

#[derive(Deserialize)]
struct SaveSortBody {
    name: String,
    columns: Value,
}

async fn save_sort_preset(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
    Json(body): Json<SaveSortBody>,
) -> Result<Json<Value>, Err> {
    if body.name.len() > MAX_PRESET_NAME_LEN {
        return Err(Err::bad_request("preset name exceeds maximum length"));
    }
    let columns_json =
        serde_json::to_string(&body.columns).map_err(|e| Err::internal(anyhow::Error::from(e)))?;
    if columns_json.len() > MAX_VALUE_BYTES {
        return Err(Err::bad_request("value exceeds maximum size"));
    }
    let conn_id = require_conn_id(&mode).await?;
    let id = presets::save_sort_preset(
        store.pool(),
        &conn_id,
        &schema,
        &table,
        &body.name,
        &body.columns,
    )
    .await
    .map_err(Err::internal)?;
    Ok(Json(serde_json::json!({ "id": id })))
}

async fn delete_sort_preset(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table, name)): Path<(String, String, String)>,
) -> Result<StatusCode, Err> {
    let conn_id = require_conn_id(&mode).await?;
    let deleted = presets::delete_sort_preset(store.pool(), &conn_id, &schema, &table, &name)
        .await
        .map_err(Err::internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Err::not_found("sort preset not found"))
    }
}

// ── Filter presets ────────────────────────────────────────────────────────────

async fn list_filter_presets(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
) -> Result<Json<Value>, Err> {
    let conn_id = require_conn_id(&mode).await?;
    let items = presets::list_filter_presets(store.pool(), &conn_id, &schema, &table).await?;
    Ok(Json(
        serde_json::to_value(items).map_err(|e| Err::internal(anyhow::Error::from(e)))?,
    ))
}

#[derive(Deserialize)]
struct SaveFilterBody {
    name: String,
    filters: Value,
}

async fn save_filter_preset(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
    Json(body): Json<SaveFilterBody>,
) -> Result<Json<Value>, Err> {
    if body.name.len() > MAX_PRESET_NAME_LEN {
        return Err(Err::bad_request("preset name exceeds maximum length"));
    }
    let filters_json =
        serde_json::to_string(&body.filters).map_err(|e| Err::internal(anyhow::Error::from(e)))?;
    if filters_json.len() > MAX_VALUE_BYTES {
        return Err(Err::bad_request("value exceeds maximum size"));
    }
    let conn_id = require_conn_id(&mode).await?;
    let id = presets::save_filter_preset(
        store.pool(),
        &conn_id,
        &schema,
        &table,
        &body.name,
        &body.filters,
    )
    .await
    .map_err(Err::internal)?;
    Ok(Json(serde_json::json!({ "id": id })))
}

async fn delete_filter_preset(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table, name)): Path<(String, String, String)>,
) -> Result<StatusCode, Err> {
    let conn_id = require_conn_id(&mode).await?;
    let deleted = presets::delete_filter_preset(store.pool(), &conn_id, &schema, &table, &name)
        .await
        .map_err(Err::internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Err::not_found("filter preset not found"))
    }
}

// ── UI state ──────────────────────────────────────────────────────────────────

const MAX_UI_STATE_KEY_LEN: usize = 200;
const MAX_PRESET_NAME_LEN: usize = 200;
const MAX_VALUE_BYTES: usize = 64 * 1024; // 64 KiB

async fn get_ui_state(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path(key): Path<String>,
) -> Result<Json<Value>, Err> {
    if key.len() > MAX_UI_STATE_KEY_LEN {
        return Err(Err::bad_request("ui state key exceeds maximum length"));
    }
    let conn_id = require_conn_id(&mode).await?;
    match ui_state::get(store.pool(), &conn_id, &key).await? {
        Some(val) => Ok(Json(val)),
        None => Err(Err::not_found("ui state key not found")),
    }
}

#[derive(Deserialize)]
struct SetUiStateBody {
    value: Value,
}

async fn set_ui_state(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path(key): Path<String>,
    Json(body): Json<SetUiStateBody>,
) -> Result<StatusCode, Err> {
    if key.len() > MAX_UI_STATE_KEY_LEN {
        return Err(Err::bad_request("ui state key exceeds maximum length"));
    }
    let value_json =
        serde_json::to_string(&body.value).map_err(|e| Err::internal(anyhow::Error::from(e)))?;
    if value_json.len() > MAX_VALUE_BYTES {
        return Err(Err::bad_request("value exceeds maximum size"));
    }
    let conn_id = require_conn_id(&mode).await?;
    ui_state::set(store.pool(), &conn_id, &key, &body.value)
        .await
        .map_err(Err::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn require_conn_id(mode: &SharedAppMode) -> Result<String, Err> {
    let guard = mode.read().await;
    match &*guard {
        AppMode::Normal(state) => Ok(connection_id(&state.config.database.url)),
        AppMode::Setup => Err(Err::service_unavailable(
            "this endpoint is not available in setup mode",
        )),
    }
}

// ── Error type ────────────────────────────────────────────────────────────────

struct Err {
    status: StatusCode,
    message: String,
}

impl Err {
    fn internal(e: anyhow::Error) -> Self {
        tracing::error!(error = %e, "store error");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal server error".to_string(),
        }
    }
    fn bad_request(msg: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.to_string(),
        }
    }
    fn not_found(msg: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.to_string(),
        }
    }
    fn service_unavailable(msg: &str) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: msg.to_string(),
        }
    }
}

impl From<anyhow::Error> for Err {
    fn from(e: anyhow::Error) -> Self {
        Self::internal(e)
    }
}

impl IntoResponse for Err {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}
