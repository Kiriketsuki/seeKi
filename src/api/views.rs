use std::collections::HashMap;

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Path, Query},
    http::header,
    response::IntoResponse,
    routing::{get, post},
};
use bytes::Bytes;
use serde::Deserialize;

use crate::app_mode::SharedAppMode;
use crate::db::{
    SavedViewDefinition, SortDirection, SortEntry, ValidationError, ViewColumn, ViewDraft,
    ViewExportQueryParams, ViewRowsQueryParams,
};
use crate::store::{Store, connection_id, views};

pub fn router() -> Router {
    Router::new()
        .route("/", get(list_saved_views).post(create_saved_view))
        .route("/preview", post(preview_saved_view))
        .route("/fk-path", get(get_fk_path))
        .route(
            "/{id}",
            get(get_saved_view)
                .patch(rename_saved_view)
                .delete(delete_saved_view),
        )
        .route("/{id}/rows", get(get_saved_view_rows))
        .route("/{id}/csv", get(export_saved_view_csv))
}

#[derive(Debug, Deserialize)]
struct CreateSavedViewBody {
    name: String,
    base_schema: String,
    base_table: String,
    columns: Vec<ViewColumn>,
    #[serde(default)]
    filters: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct PreviewSavedViewBody {
    base_schema: String,
    base_table: String,
    columns: Vec<ViewColumn>,
    #[serde(default)]
    filters: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RenameSavedViewBody {
    name: String,
}

#[derive(Debug, Deserialize)]
struct FkPathQuery {
    base_schema: String,
    base_table: String,
    target_schema: String,
    target_table: String,
}

fn trim_name(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

async fn current_state_and_conn_id(
    mode: &SharedAppMode,
) -> Result<(std::sync::Arc<crate::AppState>, String), super::AppError> {
    let state = super::require_state(mode).await?;
    let conn_id = connection_id(&state.config.database.url);
    Ok((state, conn_id))
}

fn validate_allowed_table(
    state: &crate::AppState,
    schema: &str,
    table: &str,
) -> Result<(), super::AppError> {
    if !state
        .config
        .database
        .effective_schemas()
        .iter()
        .any(|candidate| candidate == schema)
    {
        return Err(super::AppError::bad_request(format!(
            "Schema '{schema}' is not exposed in this SeeKi connection"
        )));
    }
    if !state.config.tables.allows(schema, table) {
        return Err(super::AppError::bad_request(format!(
            "Table '{schema}.{table}' is not exposed in this SeeKi connection"
        )));
    }
    Ok(())
}

fn validate_view_definition(
    state: &crate::AppState,
    base_schema: &str,
    base_table: &str,
    columns: &[ViewColumn],
) -> Result<(), super::AppError> {
    validate_allowed_table(state, base_schema, base_table)?;
    for column in columns {
        validate_allowed_table(state, &column.source_schema, &column.source_table)?;
    }
    Ok(())
}

fn map_view_store_error(err: anyhow::Error) -> super::AppError {
    if let Some(sqlx::Error::Database(db_err)) = err.downcast_ref::<sqlx::Error>() {
        let message = db_err.message().to_lowercase();
        if message.contains("unique") && message.contains("saved_views") {
            return super::AppError::bad_request(
                "A saved view with that name already exists for this connection",
            );
        }
        if message.contains("unique") {
            return super::AppError::bad_request(
                "A saved view with that name already exists for this connection",
            );
        }
    }
    super::AppError::from(err)
}

fn parse_sort_without_validation(sort: Option<&str>) -> anyhow::Result<Vec<SortEntry>> {
    let Some(sort) = sort else {
        return Ok(Vec::new());
    };
    if sort.is_empty() {
        return Ok(Vec::new());
    }

    let mut parsed = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for segment in sort.split(',') {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            return Err(ValidationError("Malformed sort segment: empty segment".into()).into());
        }

        let (column_raw, direction_raw) = trimmed
            .split_once(':')
            .ok_or_else(|| ValidationError(format!("Malformed sort segment: {trimmed}")))?;
        let column = column_raw.trim();
        let direction = direction_raw.trim();

        if column.is_empty() || direction.is_empty() {
            return Err(ValidationError(format!("Malformed sort segment: {trimmed}")).into());
        }
        if !crate::db::postgres::is_valid_identifier(column) {
            return Err(
                ValidationError(format!("Invalid sort column name in segment: {trimmed}")).into(),
            );
        }
        if !seen.insert(column.to_string()) {
            return Err(
                ValidationError(format!("Duplicate sort column in segment: {trimmed}")).into(),
            );
        }

        let direction = if direction.eq_ignore_ascii_case("asc") {
            SortDirection::Asc
        } else if direction.eq_ignore_ascii_case("desc") {
            SortDirection::Desc
        } else {
            return Err(
                ValidationError(format!("Invalid sort direction in segment: {trimmed}")).into(),
            );
        };

        parsed.push(SortEntry {
            column: column.to_string(),
            direction,
        });
    }

    Ok(parsed)
}

async fn load_saved_view_definition(
    store: &Store,
    conn_id: &str,
    id: i64,
) -> Result<SavedViewDefinition, super::AppError> {
    views::get_view(store.pool(), conn_id, id)
        .await?
        .ok_or_else(|| super::AppError::not_found(format!("Saved view '{id}' not found")))
}

async fn list_saved_views(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
) -> Result<Json<serde_json::Value>, super::AppError> {
    let (_state, conn_id) = current_state_and_conn_id(&mode).await?;
    let items = views::list_views(store.pool(), &conn_id)
        .await
        .map_err(map_view_store_error)?;
    Ok(Json(serde_json::json!({ "views": items })))
}

async fn create_saved_view(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Json(body): Json<CreateSavedViewBody>,
) -> Result<Json<serde_json::Value>, super::AppError> {
    let (state, conn_id) = current_state_and_conn_id(&mode).await?;
    let name = trim_name(&body.name)
        .ok_or_else(|| super::AppError::bad_request("Saved view name must not be empty"))?;
    if body.columns.is_empty() {
        return Err(super::AppError::bad_request(
            "Saved views must include at least one selected column",
        ));
    }
    validate_view_definition(&state, &body.base_schema, &body.base_table, &body.columns)?;

    // Validate SQL planning before persistence.
    state
        .db
        .preview_view(
            &ViewDraft {
                base_schema: &body.base_schema,
                base_table: &body.base_table,
                columns: &body.columns,
                filters: &body.filters,
            },
            1,
        )
        .await?;

    let created = views::create_view(
        store.pool(),
        &conn_id,
        name,
        &body.base_schema,
        &body.base_table,
        &body.columns,
        &body.filters,
    )
    .await
    .map_err(map_view_store_error)?;

    Ok(Json(serde_json::json!({
        "view": created.summary()
    })))
}

async fn get_saved_view(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, super::AppError> {
    let (_state, conn_id) = current_state_and_conn_id(&mode).await?;
    let view = load_saved_view_definition(&store, &conn_id, id).await?;
    Ok(Json(serde_json::json!({ "view": view })))
}

async fn rename_saved_view(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path(id): Path<i64>,
    Json(body): Json<RenameSavedViewBody>,
) -> Result<Json<serde_json::Value>, super::AppError> {
    let (_state, conn_id) = current_state_and_conn_id(&mode).await?;
    let name = trim_name(&body.name)
        .ok_or_else(|| super::AppError::bad_request("Saved view name must not be empty"))?;
    let renamed = views::rename_view(store.pool(), &conn_id, id, name)
        .await
        .map_err(map_view_store_error)?;
    if !renamed {
        return Err(super::AppError::not_found(format!(
            "Saved view '{id}' not found"
        )));
    }

    let view = load_saved_view_definition(&store, &conn_id, id).await?;
    Ok(Json(serde_json::json!({ "view": view.summary() })))
}

async fn delete_saved_view(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path(id): Path<i64>,
) -> Result<axum::http::StatusCode, super::AppError> {
    let (_state, conn_id) = current_state_and_conn_id(&mode).await?;
    let deleted = views::delete_view(store.pool(), &conn_id, id)
        .await
        .map_err(map_view_store_error)?;
    if !deleted {
        return Err(super::AppError::not_found(format!(
            "Saved view '{id}' not found"
        )));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn preview_saved_view(
    Extension(mode): Extension<SharedAppMode>,
    Json(body): Json<PreviewSavedViewBody>,
) -> Result<Json<serde_json::Value>, super::AppError> {
    let (state, _conn_id) = current_state_and_conn_id(&mode).await?;
    validate_view_definition(&state, &body.base_schema, &body.base_table, &body.columns)?;
    let result = state
        .db
        .preview_view(
            &ViewDraft {
                base_schema: &body.base_schema,
                base_table: &body.base_table,
                columns: &body.columns,
                filters: &body.filters,
            },
            100,
        )
        .await?;
    Ok(Json(serde_json::json!(result)))
}

async fn get_fk_path(
    Extension(mode): Extension<SharedAppMode>,
    Query(query): Query<FkPathQuery>,
) -> Result<Json<serde_json::Value>, super::AppError> {
    let (state, _conn_id) = current_state_and_conn_id(&mode).await?;
    validate_allowed_table(&state, &query.base_schema, &query.base_table)?;
    validate_allowed_table(&state, &query.target_schema, &query.target_table)?;
    let path = state
        .db
        .lookup_fk_path(
            &query.base_schema,
            &query.base_table,
            &query.target_schema,
            &query.target_table,
        )
        .await?;
    Ok(Json(serde_json::json!({ "path": path })))
}

async fn get_saved_view_rows(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path(id): Path<i64>,
    Query(params): Query<super::RowsQuery>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, super::AppError> {
    let (state, conn_id) = current_state_and_conn_id(&mode).await?;
    let view = load_saved_view_definition(&store, &conn_id, id).await?;
    validate_view_definition(&state, &view.base_schema, &view.base_table, &view.columns)?;
    super::reject_legacy_sort_params(&all_params)?;
    let sort = parse_sort_without_validation(params.sort.as_deref())?;
    let filters = super::parse_filters(&all_params);
    let result = state
        .db
        .query_view_rows(&ViewRowsQueryParams {
            draft: ViewDraft {
                base_schema: &view.base_schema,
                base_table: &view.base_table,
                columns: &view.columns,
                filters: &view.filters,
            },
            page: params.page.max(1),
            page_size: params.page_size.clamp(1, super::MAX_PAGE_SIZE),
            sort: &sort,
            search: params.search.as_deref(),
            filters: &filters,
        })
        .await?;
    Ok(Json(serde_json::json!(result)))
}

async fn export_saved_view_csv(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path(id): Path<i64>,
    Query(params): Query<super::RowsQuery>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, super::AppError> {
    let (state, conn_id) = current_state_and_conn_id(&mode).await?;
    let view = load_saved_view_definition(&store, &conn_id, id).await?;
    validate_view_definition(&state, &view.base_schema, &view.base_table, &view.columns)?;
    super::reject_legacy_sort_params(&all_params)?;
    let filters = super::parse_filters(&all_params);
    let sort = parse_sort_without_validation(params.sort.as_deref())?;

    let pg_pool = state
        .db
        .pg_pool()
        .ok_or_else(|| {
            super::AppError::bad_request("CSV export is not supported for this database type")
        })?
        .clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(32);
    let filename = {
        let sanitized: String = view
            .name
            .replace(['"', '\\', ';', '\r', '\n'], "")
            .chars()
            .filter(|c| c.is_ascii())
            .collect();
        if sanitized.is_empty() {
            "saved-view.csv".to_string()
        } else {
            format!("{}.csv", sanitized.replace(' ', "_").to_lowercase())
        }
    };
    let base_schema = view.base_schema.clone();
    let base_table = view.base_table.clone();
    let view_columns = view.columns.clone();
    let definition_filters = view.filters.clone();
    let sort_owned = sort;
    let search_owned = params.search.clone();
    let filters_owned = filters;

    tokio::spawn(async move {
        use futures::StreamExt;

        let export_params = ViewExportQueryParams {
            draft: ViewDraft {
                base_schema: &base_schema,
                base_table: &base_table,
                columns: &view_columns,
                filters: &definition_filters,
            },
            sort: &sort_owned,
            search: search_owned.as_deref(),
            filters: &filters_owned,
        };
        let export = crate::db::postgres::export_view_rows_stream(&pg_pool, &export_params).await;

        let (columns, mut row_stream) = match export {
            Ok(value) => value,
            Err(err) => {
                tracing::error!(error = %err, "Saved-view CSV export failed to open row stream");
                let _ = tx
                    .send(Err(std::io::Error::other(
                        "Saved-view CSV export failed to start",
                    )))
                    .await;
                return;
            }
        };
        let display_headers: Vec<String> =
            columns.iter().map(|column| column.name.clone()).collect();

        let mut header_buf = Vec::new();
        {
            let mut writer = csv::Writer::from_writer(&mut header_buf);
            if writer.write_record(&display_headers).is_err() {
                return;
            }
            if writer.flush().is_err() {
                return;
            }
        }
        if tx.send(Ok(Bytes::from(header_buf))).await.is_err() {
            return;
        }

        let mut writer = csv::Writer::from_writer(Vec::with_capacity(8192));
        let mut batch_count = 0u32;
        let mut stream_error = false;

        while let Some(row_result) = row_stream.next().await {
            match row_result {
                Ok(row) => {
                    let fields: Vec<String> = columns
                        .iter()
                        .map(|column| {
                            super::pg_value_to_csv_string(&row, &column.name, &column.data_type)
                        })
                        .collect();
                    if writer.write_record(&fields).is_err() {
                        stream_error = true;
                        break;
                    }
                    batch_count += 1;
                    if batch_count >= 100 {
                        if writer.flush().is_err() {
                            stream_error = true;
                            break;
                        }
                        let chunk = writer.into_inner().unwrap_or_default();
                        if tx.send(Ok(Bytes::from(chunk))).await.is_err() {
                            return;
                        }
                        writer = csv::Writer::from_writer(Vec::with_capacity(8192));
                        batch_count = 0;
                    }
                }
                Err(err) => {
                    tracing::error!(error = %err, "Saved-view CSV export failed while streaming rows");
                    stream_error = true;
                    break;
                }
            }
        }

        if !stream_error && writer.flush().is_ok() {
            let remaining = writer.into_inner().unwrap_or_default();
            if !remaining.is_empty() {
                let _ = tx.send(Ok(Bytes::from(remaining))).await;
            }
        }

        if stream_error {
            let _ = tx
                .send(Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Saved-view CSV export interrupted",
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::app_mode::initial_mode;
    use crate::config::{
        AppConfig, BrandingConfig, DatabaseConfig, DatabaseKind, DisplayConfig, ServerConfig,
        TablesConfig,
    };
    use crate::store::testutil::ephemeral_store;
    use tempfile::TempDir;

    fn test_app_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 3141,
            },
            database: DatabaseConfig {
                url: "postgres://user:pass@localhost:5432/seeki".into(),
                kind: DatabaseKind::Postgres,
                max_connections: 5,
                schemas: Some(vec!["public".into()]),
            },
            tables: TablesConfig::default(),
            display: DisplayConfig::default(),
            branding: BrandingConfig::default(),
            ssh: None,
        }
    }

    async fn test_router(config: AppConfig) -> (Router, TempDir) {
        let (store, dir) = ephemeral_store().await;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://user:pass@localhost:5432/seeki")
            .unwrap();
        let mode = initial_mode(Some(crate::AppState {
            db: crate::db::DatabasePool::Postgres(pool, None),
            config,
        }));

        (
            Router::new().nest("/api", crate::api::router(mode, store)),
            dir,
        )
    }

    async fn seeded_router() -> (Router, i64, TempDir) {
        let (store, dir) = ephemeral_store().await;
        let created = views::create_view(
            store.pool(),
            "localhost:5432/seeki",
            "Orders",
            "public",
            "orders",
            &[ViewColumn {
                source_schema: "public".into(),
                source_table: "orders".into(),
                column_name: "id".into(),
                alias: None,
                aggregate: None,
            }],
            &HashMap::new(),
        )
        .await
        .unwrap();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://user:pass@localhost:5432/seeki")
            .unwrap();
        let mode = initial_mode(Some(crate::AppState {
            db: crate::db::DatabasePool::Postgres(pool, None),
            config: test_app_config(),
        }));
        let app = Router::new().nest("/api", crate::api::router(mode, store));
        (app, created.id, dir)
    }

    #[tokio::test]
    async fn list_saved_views_route_matches_without_trailing_slash() {
        let (app, _dir) = test_router(test_app_config()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/views")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!({ "views": [] }));
    }

    #[tokio::test]
    async fn get_saved_view_route_returns_seeded_definition() {
        let (app, id, _dir) = seeded_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/views/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["view"]["name"], "Orders");
        assert_eq!(json["view"]["base_table"], "orders");
    }

    #[tokio::test]
    async fn rename_saved_view_route_returns_summary_shape() {
        let (app, id, _dir) = seeded_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/views/{id}"))
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"name":"Renamed"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["view"]["name"], "Renamed");
        assert!(json["view"]["columns"].is_null());
        assert_eq!(json["view"]["definition_version"], 1);
    }

    #[tokio::test]
    async fn delete_saved_view_route_removes_saved_view() {
        let (app, id, _dir) = seeded_router().await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/views/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/views/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_saved_view_rejects_empty_name_before_touching_postgres() {
        let (app, _dir) = test_router(test_app_config()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/views")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"name":"   ","base_schema":"public","base_table":"orders","columns":[{"source_schema":"public","source_table":"orders","column_name":"id"}],"filters":{}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("must not be empty")
        );
    }

    #[tokio::test]
    async fn preview_saved_view_rejects_non_exposed_tables_before_touching_postgres() {
        let mut config = test_app_config();
        config.tables = TablesConfig {
            include: Some(vec!["public.orders".into()]),
            exclude: None,
        };
        let (app, _dir) = test_router(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/views/preview")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"base_schema":"public","base_table":"orders","columns":[{"source_schema":"public","source_table":"customers","column_name":"id"}],"filters":{}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("is not exposed in this SeeKi connection")
        );
    }
}
