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

pub fn router() -> Router {
    Router::new()
        // Settings — available in both Setup and Normal mode
        .route("/settings", get(get_settings).post(set_settings))
        // Last-used table state — Normal mode only
        .route(
            "/presets/last-used/{schema}/{table}",
            get(get_last_used).post(set_last_used),
        )
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
    let pairs = settings::get_all(store.pool()).await.map_err(Err::internal)?;
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
        let json = serde_json::to_string(v)
            .map_err(|e| Err::internal(anyhow::Error::from(e)))?;
        if json.len() > MAX_VALUE_BYTES {
            return Err(Err::bad_request("value exceeds maximum size"));
        }
        validate_known_setting(k, v)?;
    }
    let pairs: Vec<(&str, &Value)> = entries
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
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
    let filter_json = serde_json::to_string(&body.filters)
        .map_err(|e| Err::internal(anyhow::Error::from(e)))?;
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

// ── Sort presets ──────────────────────────────────────────────────────────────

async fn list_sort_presets(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
) -> Result<Json<Value>, Err> {
    let conn_id = require_conn_id(&mode).await?;
    let items = presets::list_sort_presets(store.pool(), &conn_id, &schema, &table).await?;
    Ok(Json(serde_json::to_value(items).map_err(|e| Err::internal(anyhow::Error::from(e)))?))
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
    let columns_json = serde_json::to_string(&body.columns)
        .map_err(|e| Err::internal(anyhow::Error::from(e)))?;
    if columns_json.len() > MAX_VALUE_BYTES {
        return Err(Err::bad_request("value exceeds maximum size"));
    }
    let conn_id = require_conn_id(&mode).await?;
    let id =
        presets::save_sort_preset(store.pool(), &conn_id, &schema, &table, &body.name, &body.columns)
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
    Ok(Json(serde_json::to_value(items).map_err(|e| Err::internal(anyhow::Error::from(e)))?))
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
    let filters_json = serde_json::to_string(&body.filters)
        .map_err(|e| Err::internal(anyhow::Error::from(e)))?;
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
    let value_json = serde_json::to_string(&body.value)
        .map_err(|e| Err::internal(anyhow::Error::from(e)))?;
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
// Mirrors AppError in src/api/mod.rs; kept local to avoid exposing a pub type
// across modules for what is a small handler set.

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
        (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request};
    use http_body_util::BodyExt;

    use crate::app_mode::initial_mode;
    use crate::store::testutil::ephemeral_store;

    fn setup_router(mode: crate::app_mode::SharedAppMode, store: Store) -> Router {
        Router::new()
            .nest("/preferences", router())
            .layer(Extension(store))
            .layer(Extension(mode))
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    // ── Settings in Setup mode ─────────────────────────────────────────────

    #[tokio::test]
    async fn settings_work_in_setup_mode() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None); // Setup mode
        let app = setup_router(mode, store);

        // POST settings
        let req = Request::builder()
            .method("POST")
            .uri("/preferences/settings")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"theme":"dark"}"#))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // GET settings
        let req = Request::builder()
            .uri("/preferences/settings")
            .body(Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["theme"], "dark");
    }

    // ── Presets return 503 in Setup mode ───────────────────────────────────

    #[tokio::test]
    async fn presets_unavailable_in_setup_mode() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let req = Request::builder()
            .uri("/preferences/presets/sort/public/vehicles")
            .body(Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    // ── HTTP 400 for oversized values ──────────────────────────────────────

    #[tokio::test]
    async fn settings_reject_oversized_value() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        // Value serialises to > MAX_VALUE_BYTES (64 KiB)
        let big = "x".repeat(65 * 1024);
        let body = serde_json::json!({ "theme": big }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/settings")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn settings_reject_oversized_key() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let long_key = "k".repeat(201);
        let body = serde_json::json!({ long_key: "v" }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/settings")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn settings_reject_empty_branding_title() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/settings")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"branding.title":"   "}"#))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn settings_reject_invalid_date_format() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/settings")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"appearance.date_format":"RFC3339"}"#))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn settings_reject_invalid_row_density() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/settings")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"appearance.row_density":"spacious"}"#))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn sort_preset_reject_oversized_name() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let long_name = "k".repeat(201);
        let body = serde_json::json!({ "name": long_name, "columns": {} }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/presets/sort/public/vehicles")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn filter_preset_reject_oversized_name() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let long_name = "k".repeat(201);
        let body = serde_json::json!({ "name": long_name, "filters": {} }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/presets/filter/public/vehicles")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn ui_state_reject_oversized_key() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let long_key = "k".repeat(201);
        let body = serde_json::json!({ "value": "x" }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/preferences/ui-state/{long_key}"))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn last_used_reject_oversized_sort_columns() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let body =
            serde_json::json!({ "sort_columns": "x".repeat(65 * 1024), "filters": {} })
                .to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/presets/last-used/public/vehicles")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn ui_state_reject_oversized_value() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let big_value = "x".repeat(65 * 1024);
        let body = serde_json::json!({ "value": big_value }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/ui-state/my_key")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn last_used_reject_oversized_filters() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let body =
            serde_json::json!({ "sort_columns": [], "filters": "x".repeat(65 * 1024) })
                .to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/presets/last-used/public/vehicles")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn last_used_reject_oversized_search_term() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let big_term = "x".repeat(65 * 1024);
        let body =
            serde_json::json!({ "sort_columns": [], "filters": {}, "search_term": big_term })
                .to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/presets/last-used/public/vehicles")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn sort_preset_reject_oversized_columns() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let big_columns = "x".repeat(65 * 1024);
        let body = serde_json::json!({ "name": "my_preset", "columns": big_columns }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/presets/sort/public/vehicles")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn filter_preset_reject_oversized_filters() {
        let (store, _dir) = ephemeral_store().await;
        let mode = initial_mode(None);
        let app = setup_router(mode, store);

        let big_filters = "x".repeat(65 * 1024);
        let body =
            serde_json::json!({ "name": "my_preset", "filters": big_filters }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/preferences/presets/filter/public/vehicles")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Delete non-existent preset returns 404 ─────────────────────────────
    // The HTTP 404 path for delete_sort_preset (preferences.rs:138-142) is
    // exercised indirectly: the store layer returns `false` for a missing row
    // (covered by `delete_sort_preset_delete_nonexistent_returns_false` in
    // store/presets.rs), and the handler converts that to 404. A full HTTP-level
    // test would require a running Postgres to build a real AppState; that is out
    // of scope for unit tests. See integration tests if added in future.
}
