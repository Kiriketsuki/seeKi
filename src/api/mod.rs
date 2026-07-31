pub mod preferences;
pub mod setup;
pub mod update;
pub mod views;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Path, Query},
    http::header,
    response::IntoResponse,
    routing::{get, patch, post, put},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::app_mode::{AppMode, SharedAppMode};
use crate::config::{display_name_column, display_name_table};
use crate::db::postgres::is_valid_identifier;
use crate::db::{
    ColumnInfo, ExportQueryParams, RelationshipEdge, RowQueryParams, SortDirection, SortEntry,
    ValidationError,
};
use crate::store::{Store, display_names};

pub fn router(mode: SharedAppMode, store: Store) -> Router {
    Router::new()
        .route("/tables", get(list_tables))
        .route("/tables/display-names", get(get_table_display_names))
        .route(
            "/tables/display-names/{schema}/{table}",
            put(put_table_display_name),
        )
        .route("/tables/{schema}/{table}/columns", get(get_columns))
        .route(
            "/tables/{schema}/{table}/relationships",
            get(get_relationships),
        )
        .route("/tables/{schema}/{table}/references", get(get_references))
        .route("/tables/{schema}/{table}/samples", get(get_column_samples))
        .route("/tables/{schema}/{table}/rows", get(get_rows))
        .route("/tables/{schema}/{table}/row", get(get_single_row))
        .route("/config/display", get(get_display_config))
        .route("/connection-status", get(get_connection_status))
        .route("/export/{schema}/{table}/csv", get(export_csv))
        .route("/status", get(status))
        .route("/setup/test-connection", post(setup::test_connection))
        .route("/setup/save", post(setup::save_config))
        .route("/version", get(update::get_version))
        .route("/update/status", get(update::get_update_status))
        .route("/update/check", post(update::check_update))
        .route("/update/token", get(update::get_update_token))
        .route("/update/settings", patch(update::update_settings))
        // The three mutating update endpoints are gated by bearer-token auth.
        .merge(update::protected_update_router())
        .nest("/views", views::router())
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
    /// IANA name the frontend must format timestamps in, so the grid agrees with
    /// the offset the backend serialized and with SQL-side date bucketing.
    /// Without this the browser rendered in the viewer's own zone.
    timezone: String,
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
    // Partition children reuse the parent's column set, so the display config
    // skips them. This keeps the bulk column fetch small on heavily
    // partitioned databases.
    let allowed_tables: Vec<_> = all_tables
        .into_iter()
        .filter(|t| t.partition_parent.is_none())
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
        let table_columns = all_columns.get(&key).cloned().unwrap_or_default();
        let sibling_names_owned: Vec<String> =
            table_columns.iter().map(|c| c.name.clone()).collect();
        let sibling_names: Vec<&str> = sibling_names_owned.iter().map(|s| s.as_str()).collect();
        let columns: HashMap<String, ColumnDisplayConfig> = table_columns
            .into_iter()
            .map(|c| {
                let display = display_name_column(
                    &table.schema,
                    &table.name,
                    &c.name,
                    &sibling_names,
                    &state.config.display,
                );
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
        timezone: crate::db::timezone::display_timezone().name().to_string(),
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
        .map_err(|e| AppError::internal(e.to_string()))?;

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
    Extension(store): Extension<Store>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    let schemas = state.config.database.effective_schemas();
    let all_tables = state.db.list_tables(&schemas).await?;
    let overrides = display_names::list_display_names(store.pool()).await?;
    let override_map: HashMap<(String, String), String> = overrides
        .into_iter()
        .map(|e| ((e.schema_name, e.table_name), e.display_name))
        .collect();
    let tables: Vec<serde_json::Value> = all_tables
        .into_iter()
        .filter(|t| state.config.tables.allows(&t.schema, &t.name))
        .map(|t| {
            let key = (t.schema.clone(), t.name.clone());
            let display = override_map
                .get(&key)
                .cloned()
                .unwrap_or_else(|| display_name_table(&t.schema, &t.name, &state.config.display));
            serde_json::json!({
                "schema": t.schema,
                "name": t.name,
                "display_name": display,
                "row_count_estimate": t.row_count_estimate,
                "is_partitioned": t.is_partitioned,
                "partition_parent": t.partition_parent,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "tables": tables })))
}

/// GET /api/tables/display-names — maps "schema.table" -> stored display-name override.
async fn get_table_display_names(
    Extension(store): Extension<Store>,
) -> Result<Json<HashMap<String, String>>, AppError> {
    let entries = display_names::list_display_names(store.pool()).await?;
    let map: HashMap<String, String> = entries
        .into_iter()
        .map(|e| {
            (
                format!("{}.{}", e.schema_name, e.table_name),
                e.display_name,
            )
        })
        .collect();
    Ok(Json(map))
}

#[derive(Debug, Deserialize)]
struct SetTableDisplayNameBody {
    #[serde(default)]
    display_name: String,
}

/// PUT /api/tables/display-names/{schema}/{table} — set or (if empty) revert a table's
/// display-name override. Responds with the resolved display name after the change.
async fn put_table_display_name(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
    Json(body): Json<SetTableDisplayNameBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !is_valid_identifier(&schema) || !is_valid_identifier(&table) {
        return Err(AppError::bad_request("Invalid schema or table name"));
    }
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }

    display_names::set_display_name(store.pool(), &schema, &table, &body.display_name)
        .await
        .map_err(AppError::bad_request_from_err)?;

    let resolved = display_names::get_display_name(store.pool(), &schema, &table)
        .await?
        .unwrap_or_else(|| display_name_table(&schema, &table, &state.config.display));

    Ok(Json(serde_json::json!({ "display_name": resolved })))
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
    let sibling_names: Vec<&str> = raw_columns.iter().map(|c| c.name.as_str()).collect();
    let columns: Vec<serde_json::Value> = raw_columns
        .iter()
        .map(|c| {
            let display = display_name_column(
                &schema,
                &table,
                &c.name,
                &sibling_names,
                &state.config.display,
            );
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

/// GET /api/tables/{schema}/{table}/relationships — FK edges touching one
/// table. Non-allowed outgoing targets stay listed with `allowed: false` so
/// the grid can still highlight the column without offering a hop.
/// Non-allowed incoming sources are omitted entirely.
async fn get_relationships(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !is_valid_identifier(&schema) || !is_valid_identifier(&table) {
        return Err(AppError::bad_request("Invalid schema or table name"));
    }
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }

    let (outgoing, incoming) = state
        .db
        .table_relationships(&schema, &table)
        .await
        .map_err(|e| map_table_query_error(e, &table))?;

    let overrides = display_names::list_display_names(store.pool()).await?;
    let override_map: HashMap<(String, String), String> = overrides
        .into_iter()
        .map(|e| ((e.schema_name, e.table_name), e.display_name))
        .collect();
    let far_display_name = |far_schema: &str, far_table: &str| {
        override_map
            .get(&(far_schema.to_string(), far_table.to_string()))
            .cloned()
            .unwrap_or_else(|| display_name_table(far_schema, far_table, &state.config.display))
    };

    let outgoing: Vec<serde_json::Value> = outgoing
        .iter()
        .map(|edge| {
            serde_json::json!({
                "constraint": edge.constraint_name,
                "columns": edge.columns,
                "target": {
                    "schema": edge.other_schema,
                    "table": edge.other_table,
                    "display_name": far_display_name(&edge.other_schema, &edge.other_table),
                    "columns": edge.other_columns,
                    "allowed": state.config.tables.allows(&edge.other_schema, &edge.other_table),
                },
            })
        })
        .collect();
    let incoming: Vec<serde_json::Value> = incoming
        .iter()
        .filter(|edge| {
            state
                .config
                .tables
                .allows(&edge.other_schema, &edge.other_table)
        })
        .map(|edge| {
            serde_json::json!({
                "constraint": edge.constraint_name,
                "columns": edge.columns,
                "source": {
                    "schema": edge.other_schema,
                    "table": edge.other_table,
                    "display_name": far_display_name(&edge.other_schema, &edge.other_table),
                    "columns": edge.other_columns,
                    "allowed": true,
                },
            })
        })
        .collect();

    Ok(Json(
        serde_json::json!({ "outgoing": outgoing, "incoming": incoming }),
    ))
}

/// Upper bound on the number of `/references` count queries in flight at once.
/// Each count takes one pool connection, so an unbounded fan-out over a table
/// with many incoming foreign keys would starve every other request.
const REFERENCE_COUNT_CONCURRENCY: usize = 4;

/// Pick the incoming edges the `/references` endpoint counts, and pair each one
/// with the referenced values taken from the `eq.` filters, in constraint order.
/// An edge drops out when its source table is outside the allowlist, or when the
/// row carries no value for one of the referenced columns. A missing value means
/// the value is NULL on the row, and NULL matches no foreign key.
fn select_reference_candidates<'a>(
    incoming: &'a [RelationshipEdge],
    tables: &crate::config::TablesConfig,
    exact_filters: &HashMap<String, String>,
) -> Vec<(&'a RelationshipEdge, Vec<String>)> {
    incoming
        .iter()
        .filter(|edge| tables.allows(&edge.other_schema, &edge.other_table))
        .filter_map(|edge| {
            let values: Option<Vec<String>> = edge
                .columns
                .iter()
                .map(|col| exact_filters.get(col).cloned())
                .collect();
            values.map(|values| (edge, values))
        })
        .collect()
}

/// GET /api/tables/{schema}/{table}/references — capped count of rows in every
/// other table that references this row, one entry per matching incoming FK
/// edge. The `eq.` query parameters carry the current row's referenced column
/// values, the same namespace `get_rows` uses for exact-match filters.
async fn get_references(
    Extension(mode): Extension<SharedAppMode>,
    Extension(store): Extension<Store>,
    Path((schema, table)): Path<(String, String)>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !is_valid_identifier(&schema) || !is_valid_identifier(&table) {
        return Err(AppError::bad_request("Invalid schema or table name"));
    }
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }

    let exact_filters = parse_exact_filters(&all_params);
    if exact_filters.is_empty() {
        return Err(AppError::bad_request(
            "At least one `eq.` filter is required to look up related rows",
        ));
    }

    let (_, incoming) = state
        .db
        .table_relationships(&schema, &table)
        .await
        .map_err(|e| map_table_query_error(e, &table))?;

    let overrides = display_names::list_display_names(store.pool()).await?;
    let override_map: HashMap<(String, String), String> = overrides
        .into_iter()
        .map(|e| ((e.schema_name, e.table_name), e.display_name))
        .collect();
    let far_display_name = |far_schema: &str, far_table: &str| {
        override_map
            .get(&(far_schema.to_string(), far_table.to_string()))
            .cloned()
            .unwrap_or_else(|| display_name_table(far_schema, far_table, &state.config.display))
    };

    let candidates = select_reference_candidates(&incoming, &state.config.tables, &exact_filters);

    // Run the per-edge counts concurrently, but never more than
    // REFERENCE_COUNT_CONCURRENCY at a time. A table with many incoming edges
    // would otherwise take one pool connection per edge and starve every other
    // request while the counts run.
    let mut count_futures = Vec::with_capacity(candidates.len());
    for (edge, values) in &candidates {
        count_futures.push(state.db.count_referencing_rows(
            &edge.other_schema,
            &edge.other_table,
            &edge.other_columns,
            values,
        ));
    }
    let counts: Vec<anyhow::Result<(i64, bool)>> = futures::stream::iter(count_futures)
        .buffered(REFERENCE_COUNT_CONCURRENCY)
        .collect()
        .await;

    let mut references: Vec<serde_json::Value> = Vec::with_capacity(candidates.len());
    for ((edge, _), result) in candidates.iter().zip(counts) {
        let (count, capped) = result.map_err(|e| map_table_query_error(e, &edge.other_table))?;
        references.push(serde_json::json!({
            "schema": edge.other_schema,
            "table": edge.other_table,
            "display_name": far_display_name(&edge.other_schema, &edge.other_table),
            "columns": edge.other_columns,
            "count": count,
            "capped": capped,
        }));
    }
    // Sort on the display name, then on the source columns, so two edges from
    // the same source table keep a stable, predictable order in the menu.
    references.sort_by(|a, b| {
        let key = |v: &serde_json::Value| {
            (
                v["display_name"].as_str().unwrap_or_default().to_string(),
                v["columns"].to_string(),
            )
        };
        key(a).cmp(&key(b))
    });

    Ok(Json(serde_json::json!({ "references": references })))
}

#[derive(Deserialize)]
struct ColumnSamplesQuery {
    column: String,
}

async fn get_column_samples(
    Extension(mode): Extension<SharedAppMode>,
    Path((schema, table)): Path<(String, String)>,
    Query(query): Query<ColumnSamplesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }
    if !is_valid_identifier(&query.column) {
        return Err(AppError::bad_request(format!(
            "Invalid column name: {}",
            truncate_for_error(&query.column)
        )));
    }

    let columns = load_table_columns(&state, &schema, &table).await?;
    if !columns.iter().any(|column| column.name == query.column) {
        return Err(AppError::bad_request(format!(
            "Unknown column '{}' on table '{schema}.{table}'",
            truncate_for_error(&query.column)
        )));
    }

    let samples = state
        .db
        .sample_column_values(&schema, &table, &query.column, 5)
        .await
        .map_err(|e| map_table_query_error(e, &table))?;
    Ok(Json(serde_json::json!({ "samples": samples })))
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

/// Extract exact-match filters from query params with the `eq.` prefix.
/// e.g. `?eq.user_id=42` matches rows where user_id equals 42 exactly,
/// unlike `filter.` params which match substrings.
fn parse_exact_filters(all_params: &HashMap<String, String>) -> HashMap<String, String> {
    all_params
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix("eq.")
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
pub(crate) fn truncate_for_error(value: &str) -> String {
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
    let exact_filters = parse_exact_filters(&all_params);
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
            exact_filters: &exact_filters,
        })
        .await
        .map_err(|e| map_table_query_error(e, &table))?;
    Ok(Json(serde_json::json!(result)))
}

/// GET /api/tables/{schema}/{table}/row — one row matching an `eq.` exact-filter
/// set, for the FK peek panel. Requires at least one `eq.` filter so the query
/// cannot degrade into "the first row of the whole table".
async fn get_single_row(
    Extension(mode): Extension<SharedAppMode>,
    Path((schema, table)): Path<(String, String)>,
    Query(all_params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = require_state(&mode).await?;
    if !state.config.tables.allows(&schema, &table) {
        return Err(AppError::not_found(format!(
            "Table '{schema}.{table}' not found"
        )));
    }
    // Reject a filter-less request before the schema round trip. The check needs
    // no column metadata, so paying for it first wastes a database call.
    let exact_filters = parse_exact_filters(&all_params);
    if exact_filters.is_empty() {
        return Err(AppError::bad_request("At least one eq. filter is required"));
    }

    let columns = load_table_columns(&state, &schema, &table).await?;
    let valid_column_names: std::collections::HashSet<&str> =
        columns.iter().map(|c| c.name.as_str()).collect();
    for col_name in exact_filters.keys() {
        if !valid_column_names.contains(col_name.as_str()) {
            return Err(AppError::bad_request(format!(
                "Unknown column '{}' on table '{schema}.{table}'",
                truncate_for_error(col_name)
            )));
        }
    }

    let (row, multiple) = state
        .db
        .query_single_row(&schema, &table, &exact_filters)
        .await
        .map_err(|e| map_table_query_error(e, &table))?;

    let sibling_names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
    let response_columns: Vec<serde_json::Value> = columns
        .iter()
        .map(|c| {
            let display = display_name_column(
                &schema,
                &table,
                &c.name,
                &sibling_names,
                &state.config.display,
            );
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

    Ok(Json(serde_json::json!({
        "row": row,
        "multiple": multiple,
        "columns": response_columns,
    })))
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
    let exact_filters = parse_exact_filters(&all_params);
    let columns = load_table_columns(&state, &schema, &table).await?;
    let sort = parse_sort_param(params.sort.as_deref(), &columns)?;

    // Fetch columns eagerly so we can build headers before spawning
    let sibling_names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
    let display_headers: Vec<String> = columns
        .iter()
        .map(|c| {
            display_name_column(
                &schema,
                &table,
                &c.name,
                &sibling_names,
                &state.config.display,
            )
        })
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
            exact_filters: &exact_filters,
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
        // Same zone the grid renders in — exporting UTC while showing local time
        // meant the file disagreed with the screen it came from.
        "timestamp with time zone" => row
            .try_get::<chrono::DateTime<chrono::Utc>, _>(col)
            .map(|v| {
                crate::db::timezone::format_timestamptz(v, crate::db::timezone::display_timezone())
            })
            .unwrap_or_default(),
        "date" => row
            .try_get::<chrono::NaiveDate, _>(col)
            .map(|v| v.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        "time without time zone" => row
            .try_get::<chrono::NaiveTime, _>(col)
            .map(|v| v.format("%H:%M:%S").to_string())
            .unwrap_or_default(),
        "time with time zone" => row
            .try_get::<crate::db::postgres::PgTimeTzChrono, _>(col)
            .map(crate::db::postgres::format_timetz)
            .unwrap_or_default(),
        "uuid" => row
            .try_get::<uuid::Uuid, _>(col)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        // inet/cidr binary wire format is a struct, not text — the unchecked String
        // fallback below cannot decode it (exports empty cells).
        "inet" | "cidr" => row
            .try_get::<sqlx::types::ipnet::IpNet, _>(col)
            .map(crate::db::postgres::format_ipnet)
            .unwrap_or_default(),
        // USER-DEFINED types (Postgres enums, citext, domains, etc.) have OIDs sqlx
        // doesn't statically know, so the checked `try_get::<String, _>` rejects them
        // even though their wire format is plain text. `try_get_unchecked` skips that
        // type-compatibility check and decodes the text representation directly —
        // without it, enum/citext/domain columns export as empty cells for every row.
        _ => row.try_get_unchecked::<String, _>(col).unwrap_or_default(),
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

    /// Map a fallible store call's error to a 400. Used for user-input validation
    /// failures (e.g. display-name length) surfaced as plain `anyhow::Error`s.
    pub(super) fn bad_request_from_err(err: anyhow::Error) -> Self {
        Self::bad_request(err.to_string())
    }

    pub(super) fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::FORBIDDEN,
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

    fn test_app_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 3141,
            },
            database: DatabaseConfig {
                url: "postgres://user:pass@localhost:5432/seeki".into(),
                kind: crate::config::DatabaseKind::Postgres,
                max_connections: 5,
                schemas: Some(vec!["public".into()]),
            },
            tables: TablesConfig::default(),
            display: crate::config::DisplayConfig::default(),
            branding: crate::config::BrandingConfig::default(),
            ssh: None,
        }
    }

    async fn test_api_router(config: AppConfig) -> Router {
        let (store, _dir) = ephemeral_store().await;
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://user:pass@localhost:5432/seeki")
            .unwrap();
        let mode = initial_mode(Some(crate::AppState {
            db: crate::db::DatabasePool::Postgres(pool, None),
            config,
        }));
        Router::new().nest("/api", router(mode, store))
    }

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
            timezone: "Asia/Singapore".into(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["branding"]["title"], "My Database");
        assert_eq!(json["branding"]["subtitle"], "Fleet Telemetry");
        assert!(json["tables"].as_object().unwrap().is_empty());
        assert_eq!(json["timezone"], "Asia/Singapore");
    }

    #[test]
    fn display_config_response_serializes_null_branding() {
        let response = DisplayConfigResponse {
            branding: BrandingResponse {
                title: None,
                subtitle: None,
            },
            tables: HashMap::new(),
            timezone: "UTC".into(),
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

    #[tokio::test]
    async fn column_samples_route_rejects_invalid_identifier_before_querying_postgres() {
        let app = test_api_router(test_app_config()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tables/public/orders/samples?column=id;drop")
                    .body(Body::empty())
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
                .contains("Invalid column name")
        );
    }

    #[tokio::test]
    async fn column_samples_route_hides_non_exposed_tables() {
        let mut config = test_app_config();
        config.tables = TablesConfig {
            include: Some(vec!["public.visible_table".into()]),
            exclude: None,
        };
        let app = test_api_router(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tables/public/orders/samples?column=id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn single_row_route_hides_non_exposed_tables() {
        let mut config = test_app_config();
        config.tables = TablesConfig {
            include: Some(vec!["public.visible_table".into()]),
            exclude: None,
        };
        let app = test_api_router(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tables/public/orders/row?eq.id=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn single_row_route_requires_at_least_one_exact_filter() {
        let app = test_api_router(test_app_config()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tables/public/orders/row")
                    .body(Body::empty())
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
                .unwrap_or_default()
                .contains("eq. filter"),
            "unexpected error body: {json}"
        );
    }

    #[tokio::test]
    async fn single_row_route_ignores_non_eq_query_params() {
        // A `search=` or `page=` param carries no exact filter, so the request
        // must still fail closed rather than return an arbitrary first row.
        let app = test_api_router(test_app_config()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tables/public/orders/row?search=abc&page=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
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
        let response = serde_json::to_value(update::VersionResponse {
            version: env!("SEEKI_VERSION"),
            commit: env!("SEEKI_GIT_COMMIT"),
            built_at: env!("SEEKI_BUILT_AT"),
        })
        .unwrap();

        assert!(!response["version"].as_str().unwrap().is_empty());
        assert!(!response["commit"].as_str().unwrap().is_empty());
        assert!(!response["built_at"].as_str().unwrap().is_empty());
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
    fn parse_exact_filters_extracts_eq_prefixed_params() {
        let mut params = HashMap::new();
        params.insert("eq.user_id".into(), "42".into());
        params.insert("filter.name".into(), "Ali".into());
        params.insert("page".into(), "1".into());

        let exact = parse_exact_filters(&params);
        assert_eq!(exact.len(), 1);
        assert_eq!(exact["user_id"], "42");

        // The two namespaces stay independent.
        let filters = parse_filters(&params);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters["name"], "Ali");
    }

    #[test]
    fn parse_exact_filters_returns_empty_without_eq_params() {
        let mut params = HashMap::new();
        params.insert("page".into(), "1".into());
        assert!(parse_exact_filters(&params).is_empty());
    }

    fn incoming_edge(
        constraint: &str,
        source_table: &str,
        source_columns: &[&str],
        referenced_columns: &[&str],
    ) -> RelationshipEdge {
        RelationshipEdge {
            constraint_name: constraint.to_string(),
            columns: referenced_columns.iter().map(|c| c.to_string()).collect(),
            other_schema: "public".to_string(),
            other_table: source_table.to_string(),
            other_columns: source_columns.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn select_reference_candidates_omits_non_allowed_source_tables() {
        let incoming = vec![
            incoming_edge("items_order_fkey", "order_items", &["order_id"], &["id"]),
            incoming_edge("audit_order_fkey", "audit_log", &["order_id"], &["id"]),
        ];
        let tables = crate::config::TablesConfig {
            include: None,
            exclude: Some(vec!["audit_log".to_string()]),
        };
        let mut exact = HashMap::new();
        exact.insert("id".to_string(), "7".to_string());

        let candidates = select_reference_candidates(&incoming, &tables, &exact);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0.other_table, "order_items");
        assert_eq!(candidates[0].1, vec!["7".to_string()]);
    }

    #[test]
    fn select_reference_candidates_drops_edges_with_a_missing_referenced_value() {
        // A referenced column absent from the `eq.` filters means the value is
        // NULL on the row, so the whole edge drops out.
        let incoming = vec![
            incoming_edge("simple_fkey", "shipments", &["order_id"], &["id"]),
            incoming_edge(
                "composite_fkey",
                "stock_moves",
                &["wh_id", "region"],
                &["warehouse_id", "region_code"],
            ),
        ];
        let tables = crate::config::TablesConfig::default();
        let mut exact = HashMap::new();
        exact.insert("id".to_string(), "7".to_string());
        exact.insert("warehouse_id".to_string(), "3".to_string());

        let candidates = select_reference_candidates(&incoming, &tables, &exact);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0.other_table, "shipments");
    }

    #[test]
    fn select_reference_candidates_keeps_composite_values_in_constraint_order() {
        let incoming = vec![incoming_edge(
            "composite_fkey",
            "stock_moves",
            &["wh_id", "region"],
            &["warehouse_id", "region_code"],
        )];
        let tables = crate::config::TablesConfig::default();
        let mut exact = HashMap::new();
        // Insert in the reverse order to prove the map order does not leak in.
        exact.insert("region_code".to_string(), "EU".to_string());
        exact.insert("warehouse_id".to_string(), "7".to_string());

        let candidates = select_reference_candidates(&incoming, &tables, &exact);

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].0.other_columns,
            vec!["wh_id".to_string(), "region".to_string()]
        );
        assert_eq!(
            candidates[0].1,
            vec!["7".to_string(), "EU".to_string()],
            "values pair with other_columns by position"
        );
    }

    #[test]
    fn select_reference_candidates_keeps_two_edges_from_the_same_source_table() {
        // A message table with a sender and a recipient both pointing at users
        // yields two separate entries, one per constraint.
        let incoming = vec![
            incoming_edge("messages_sender_fkey", "messages", &["sender_id"], &["id"]),
            incoming_edge(
                "messages_recipient_fkey",
                "messages",
                &["recipient_id"],
                &["id"],
            ),
        ];
        let tables = crate::config::TablesConfig::default();
        let mut exact = HashMap::new();
        exact.insert("id".to_string(), "42".to_string());

        let candidates = select_reference_candidates(&incoming, &tables, &exact);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].0.other_columns, vec!["sender_id".to_string()]);
        assert_eq!(
            candidates[1].0.other_columns,
            vec!["recipient_id".to_string()]
        );
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
            ..DisplayConfig::default()
        };

        let sibling_names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
        let headers: Vec<String> = columns
            .iter()
            .map(|c| {
                display_name_column("public", "vehicles_log", &c.name, &sibling_names, &config)
            })
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
            ..DisplayConfig::default()
        };

        let display = display_name_table("public", "vehicles_log", &config)
            .replace(' ', "_")
            .to_lowercase();
        assert_eq!(display, "fleet_log");
        assert_eq!(format!("{display}.csv"), "fleet_log.csv");
    }
}
