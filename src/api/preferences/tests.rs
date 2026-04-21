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
async fn settings_accept_valid_page_size() {
    let (store, _dir) = ephemeral_store().await;
    let mode = initial_mode(None);
    let app = setup_router(mode, store);

    let req = Request::builder()
        .method("POST")
        .uri("/preferences/settings")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"data.page_size":100}"#))
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn settings_reject_invalid_page_size() {
    let (store, _dir) = ephemeral_store().await;
    let mode = initial_mode(None);
    let app = setup_router(mode, store);

    let req = Request::builder()
        .method("POST")
        .uri("/preferences/settings")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"data.page_size":99}"#))
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn settings_accept_valid_pagination_mode() {
    let (store, _dir) = ephemeral_store().await;
    let mode = initial_mode(None);
    let app = setup_router(mode, store);

    let req = Request::builder()
        .method("POST")
        .uri("/preferences/settings")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"data.pagination_mode":"infinite"}"#))
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn settings_reject_invalid_pagination_mode() {
    let (store, _dir) = ephemeral_store().await;
    let mode = initial_mode(None);
    let app = setup_router(mode, store);

    let req = Request::builder()
        .method("POST")
        .uri("/preferences/settings")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"data.pagination_mode":"virtual"}"#))
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn last_used_accepts_page_size() {
    // Setup mode returns 503 — page_size:250 is valid JSON, not rejected with 400.
    let (store, _dir) = ephemeral_store().await;
    let mode = initial_mode(None);
    let app = setup_router(mode, store);

    let req = Request::builder()
        .method("POST")
        .uri("/preferences/presets/last-used/public/vehicles")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"sort_columns":[],"filters":{},"page_size":250}"#,
        ))
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn last_used_accepts_null_page_size() {
    // Setup mode returns 503 — page_size:null is valid JSON, not rejected with 400.
    let (store, _dir) = ephemeral_store().await;
    let mode = initial_mode(None);
    let app = setup_router(mode, store);

    let req = Request::builder()
        .method("POST")
        .uri("/preferences/presets/last-used/public/vehicles")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"sort_columns":[],"filters":{},"page_size":null}"#,
        ))
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
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
        serde_json::json!({ "sort_columns": "x".repeat(65 * 1024), "filters": {} }).to_string();

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
        serde_json::json!({ "sort_columns": [], "filters": "x".repeat(65 * 1024) }).to_string();

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
    let body = serde_json::json!({ "name": "my_preset", "filters": big_filters }).to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/preferences/presets/filter/public/vehicles")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = tower::ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
