use std::str::FromStr;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::http::header::{CACHE_CONTROL, HOST, ORIGIN, REFERER};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::{Extension, Json, extract::Request};
use serde::{Deserialize, Serialize};

use crate::update::UpdateState;
use crate::update::github;
use crate::update::swap;
use crate::update::version::SeekiVersion;

use super::AppError;

// ── Auth middleware ───────────────────────────────────────────────────────────

/// Axum middleware that verifies the `Authorization: Bearer <token>` header
/// against the server-held update token.  Applied only to the mutating update
/// endpoints: `/update/apply`, `/update/wip`, `/update/rollback`.
pub async fn require_update_token(
    Extension(update): Extension<Arc<UpdateState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token_value = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token_value {
        Some(candidate) if update.token.verify(candidate) => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

// ── GET /api/version ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct VersionResponse {
    pub version: &'static str,
    pub commit: &'static str,
    pub built_at: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateStatusResponse {
    pub current: String,
    pub latest: Option<String>,
    pub pre_release_channel: bool,
    pub poll_interval_hours: u8,
    pub update_available: bool,
    pub previous_exists: bool,
    pub last_checked: Option<String>,
    pub release_notes: Option<String>,
    pub available_builds: Vec<AvailableBuild>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableBuild {
    pub tag: String,
    pub published_at: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateTokenResponse {
    token: String,
}

pub async fn get_version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("SEEKI_VERSION"),
        commit: env!("SEEKI_GIT_COMMIT"),
        built_at: env!("SEEKI_BUILT_AT"),
    })
}

async fn build_update_status_response(update: &UpdateState) -> UpdateStatusResponse {
    let settings = {
        let guard = update.settings.lock().await;
        guard.clone()
    };
    let cached = update.cache.latest(settings.pre_release_channel).await;
    let current = SeekiVersion::current();
    let current_path = swap::current_exe_path().ok();
    let previous_exists = current_path
        .as_ref()
        .is_some_and(|path| swap::has_previous(path));

    let (latest, update_available, release_notes) = release_metadata(&current, cached.as_ref());

    // Expose the list of recent prereleases only when the prerelease channel
    // is enabled — stable users should not see prerelease builds in the picker.
    let available_builds = if settings.pre_release_channel {
        update
            .cache
            .prerelease_list()
            .await
            .map(|list| {
                list.into_iter()
                    .map(|rel| AvailableBuild {
                        tag: rel.tag_name.trim_start_matches('v').to_string(),
                        published_at: rel.published_at,
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    UpdateStatusResponse {
        current: current.to_string(),
        latest,
        pre_release_channel: settings.pre_release_channel,
        poll_interval_hours: settings.poll_interval_hours,
        update_available,
        previous_exists,
        last_checked: settings.last_checked,
        release_notes,
        available_builds,
    }
}

fn release_metadata(
    current: &SeekiVersion,
    release: Option<&github::GitHubRelease>,
) -> (Option<String>, bool, Option<String>) {
    let Some(release) = release else {
        return (None, false, None);
    };

    let tag = release.tag_name.trim_start_matches('v').to_string();
    let update_available = match SeekiVersion::from_str(&tag) {
        Ok(version) => {
            // Nightly build suffixes encode `{date}g{sha}`, which do not sort
            // meaningfully against each other. GitHub's API already returns
            // releases newest-first, so any different nightly tag is a valid
            // forward update — fall back to tag identity in that case.
            if is_nightly_suffix(&current.suffix) && is_nightly_suffix(&version.suffix) {
                tag != current.to_string()
            } else {
                version > *current
            }
        }
        Err(error) => {
            tracing::warn!(tag = %tag, error = %error, "Failed to parse release version tag");
            false
        }
    };
    (Some(tag), update_available, Some(release.body.clone()))
}

fn is_nightly_suffix(suffix: &str) -> bool {
    suffix.starts_with('n')
}

async fn persist_last_checked(update: &UpdateState) -> Result<(), AppError> {
    let mut settings = update.settings.lock().await;
    settings.last_checked = Some(chrono::Utc::now().to_rfc3339());
    settings.save()?;
    Ok(())
}

fn request_origin(headers: &HeaderMap) -> Option<url::Url> {
    let host = header_value(headers, HOST)?;
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("http");
    url::Url::parse(&format!("{scheme}://{host}")).ok()
}

fn header_value(headers: &HeaderMap, name: axum::http::header::HeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn same_origin(left: &url::Url, right: &url::Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn is_same_origin_request(headers: &HeaderMap) -> bool {
    let Some(expected_origin) = request_origin(headers) else {
        return false;
    };

    if let Some(origin) = header_value(headers, ORIGIN) {
        if origin == "null" {
            return false;
        }
        return url::Url::parse(origin)
            .map(|candidate| same_origin(&candidate, &expected_origin))
            .unwrap_or(false);
    }

    let Some(referer) = header_value(headers, REFERER) else {
        return false;
    };
    url::Url::parse(referer)
        .map(|candidate| same_origin(&candidate, &expected_origin))
        .unwrap_or(false)
}

// ── GET /api/update/status ───────────────────────────────────────────────────

pub async fn get_update_status(
    Extension(update): Extension<Arc<UpdateState>>,
) -> Result<Json<UpdateStatusResponse>, AppError> {
    Ok(Json(build_update_status_response(update.as_ref()).await))
}

// ── POST /api/update/check ───────────────────────────────────────────────────

pub async fn check_update(
    Extension(update): Extension<Arc<UpdateState>>,
) -> Result<Json<UpdateStatusResponse>, AppError> {
    let pre = {
        let s = update.settings.lock().await;
        s.pre_release_channel
    };

    let result = github::check_latest(&update.cache, pre, true).await?;
    if result.fetched {
        persist_last_checked(update.as_ref()).await?;
    }

    Ok(Json(build_update_status_response(update.as_ref()).await))
}

// ── GET /api/update/token ────────────────────────────────────────────────────

pub async fn get_update_token(
    Extension(update): Extension<Arc<UpdateState>>,
    headers: HeaderMap,
) -> Result<(axum::http::HeaderMap, Json<UpdateTokenResponse>), AppError> {
    if !is_same_origin_request(&headers) {
        return Err(AppError::forbidden(
            "Cross-origin update token requests are not allowed",
        ));
    }

    let mut response_headers = axum::http::HeaderMap::new();
    response_headers.insert(
        CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    Ok((
        response_headers,
        Json(UpdateTokenResponse {
            token: update.token.expose().to_string(),
        }),
    ))
}

// ── POST /api/update/apply ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ApplyRequest {
    source: String,
    wip_upload_id: Option<String>,
    /// Optional: install a specific release tag from the cached prerelease list
    /// instead of the newest available. Used by the build picker UI.
    release_tag: Option<String>,
}

pub async fn apply_update(
    Extension(update): Extension<Arc<UpdateState>>,
    Json(req): Json<ApplyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let current_path = swap::current_exe_path()?;

    match req.source.as_str() {
        "release" => {
            // Get cached release — either the newest one, or a specific tag
            // from the prerelease list when the build picker has selected one.
            let pre = {
                let s = update.settings.lock().await;
                s.pre_release_channel
            };

            let release = if let Some(requested_tag) = req.release_tag.as_deref() {
                let normalized = requested_tag.trim_start_matches('v');
                let list = update.cache.prerelease_list().await.ok_or_else(|| {
                    AppError::bad_request(
                        "No build list cached — run check for updates first",
                    )
                })?;
                list.into_iter()
                    .find(|r| r.tag_name.trim_start_matches('v') == normalized)
                    .ok_or_else(|| {
                        AppError::bad_request(format!(
                            "Build '{requested_tag}' not found in cached list"
                        ))
                    })?
            } else {
                github::check_latest(&update.cache, pre, false)
                    .await?
                    .release
                    .ok_or_else(|| {
                        AppError::bad_request("No release available — run check first")
                    })?
            };

            let asset = github::select_asset(&release.assets).ok_or_else(|| {
                AppError::bad_request("No compatible binary found in release assets")
            })?;

            // Require the SHA256 sidecar to be present in the release manifest
            // before downloading anything — fail fast if the release is incomplete.
            let sha256_asset_name = format!("{}.sha256", &asset.name);
            let sha256_asset = release
                .assets
                .iter()
                .find(|a| a.name == sha256_asset_name)
                .ok_or_else(|| {
                    AppError::bad_request(
                        "Release is missing a SHA256 sidecar — refusing to apply an unverified binary",
                    )
                })?;

            // Download the binary into memory and hash it in one streaming pass.
            // This eliminates both the TOCTOU window (no double-read of a temp
            // file) and any temp-file leak (no file to clean up on error).
            // Done BEFORE acquiring swap_lock so /update/rollback is not blocked
            // during the network download (which can take up to 300 s).
            let (bytes, actual_sha256) = github::download_asset_bytes(&asset.browser_download_url)
                .await
                .map_err(|e| AppError::bad_request(format!("Download failed: {e}")))?;

            // Download and compare the expected SHA256 sidecar.
            // Use the actual URL from the manifest rather than constructing it.
            let sha256_url = &sha256_asset.browser_download_url;
            let expected_sha256 = github::download_sha256(sha256_url).await.map_err(|e| {
                AppError::bad_request(format!(
                    "SHA256 sidecar exists but download failed — aborting for safety: {e}"
                ))
            })?;
            let expected_sha256 = expected_sha256.trim().to_ascii_lowercase();

            if actual_sha256 != expected_sha256 {
                return Err(AppError::bad_request(format!(
                    "SHA256 mismatch: expected {expected_sha256}, got {actual_sha256}"
                )));
            }
            tracing::info!(sha256 = %actual_sha256, "Release binary SHA256 verified");

            // Narrow critical section: lock only covers the rename sequence.
            // Install: the bytes we verified are the exact bytes installed — no
            // second read, no TOCTOU.
            let _lock = update.swap_lock.lock().await;
            swap::apply_binary_bytes(&bytes, &current_path).await?;
            swap::schedule_exit(&update.shutdown);

            Ok(Json(serde_json::json!({
                "status": "applied",
                "message": "Update applied — server will restart shortly",
            })))
        }
        "wip" => {
            let upload_id = req.wip_upload_id.ok_or_else(|| {
                AppError::bad_request("wip_upload_id is required when source is 'wip'")
            })?;

            // F01: Validate upload_id — must be exactly 8 lowercase hex characters
            let is_valid_id =
                upload_id.len() == 8 && upload_id.chars().all(|c| c.is_ascii_hexdigit());
            if !is_valid_id {
                return Err(AppError::bad_request(
                    "Invalid upload ID — expected 8 hex characters",
                ));
            }

            // F4 (server-persisted): look up the SHA256 we recorded at upload
            // time. The client never supplies the expected digest — a
            // same-user attacker who could tamper with the staged file could
            // also recompute a matching hash, so round-tripping the digest
            // through the client would be defense-theatre rather than
            // defense-in-depth.
            //
            // We clone the digest rather than removing the manifest entry so
            // that transient apply failures do not burn the upload_id — the
            // client can retry without re-uploading. The manifest is only
            // removed after apply succeeds, or after verify determines the
            // staged file is tampered/missing (both terminal for this upload).
            let expected_sha256 = {
                let manifests = update.wip_manifests.lock().await;
                manifests.get(&upload_id).cloned().ok_or_else(|| {
                    AppError::not_found(format!(
                        "WIP upload '{upload_id}' not found (no server manifest)"
                    ))
                })?
            };

            // Read, hash, and compare in one pass so the bytes we install are
            // the exact bytes we verified — closes TOCTOU between verify and
            // apply. Done BEFORE acquiring swap_lock so /update/rollback is
            // not blocked during file I/O.
            let tmp_dir = std::env::temp_dir();
            let wip_path = tmp_dir.join(format!("seeki-wip-{upload_id}"));
            let bytes = match crate::update::wip::verify_staged_wip(&wip_path, &expected_sha256) {
                crate::update::wip::VerifyWipOutcome::Ok(b) => b,
                crate::update::wip::VerifyWipOutcome::Missing => {
                    // Staged file is gone — the upload_id is unusable regardless.
                    update.wip_manifests.lock().await.remove(&upload_id);
                    return Err(AppError::not_found(format!(
                        "WIP upload '{upload_id}' staged file is missing"
                    )));
                }
                crate::update::wip::VerifyWipOutcome::Mismatch { expected, actual } => {
                    // verify_staged_wip has already deleted the tampered file.
                    update.wip_manifests.lock().await.remove(&upload_id);
                    return Err(AppError::bad_request(format!(
                        "SHA256 mismatch for WIP upload: staged file has been tampered with — expected {expected}, got {actual} — file discarded"
                    )));
                }
            };

            // Narrow critical section: lock only covers the rename sequence +
            // post-apply cleanup.
            let _lock = update.swap_lock.lock().await;
            let apply_result = swap::apply_binary_bytes(&bytes, &current_path).await;
            // Always clean up the staged file, regardless of apply outcome
            let _ = std::fs::remove_file(&wip_path);
            if apply_result.is_ok() {
                // Apply succeeded — the upload has served its purpose.
                update.wip_manifests.lock().await.remove(&upload_id);
            }
            // On apply error, leave the manifest in place so the client can
            // retry by re-uploading (which overwrites the entry). The staged
            // file has already been removed above, so retry always requires a
            // fresh upload body — this is expected.
            apply_result?;

            swap::schedule_exit(&update.shutdown);

            Ok(Json(serde_json::json!({
                "status": "applied",
                "message": "WIP binary applied — server will restart shortly",
            })))
        }
        other => Err(AppError::bad_request(format!(
            "Unknown source '{other}' — expected 'release' or 'wip'"
        ))),
    }
}

// ── POST /api/update/wip ────────────────────────────────────────────────────

pub async fn upload_wip(
    Extension(update): Extension<Arc<UpdateState>>,
    body: bytes::Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    let upload = crate::update::wip::stage_upload(body).await?;

    // Record the server-computed SHA256 so the apply handler can verify the
    // staged file without trusting any client-supplied digest.
    {
        let mut manifests = update.wip_manifests.lock().await;
        manifests.insert(upload.upload_id.clone(), upload.sha256.clone());
    }

    Ok(Json(serde_json::json!({
        "upload_id": upload.upload_id,
        "sha256": upload.sha256,
        "size": upload.size,
    })))
}

// ── POST /api/update/rollback ────────────────────────────────────────────────

pub async fn rollback(
    Extension(update): Extension<Arc<UpdateState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _lock = update.swap_lock.lock().await;
    let current_path = swap::current_exe_path()?;
    swap::rollback(&current_path).await?;
    swap::schedule_exit(&update.shutdown);

    Ok(Json(serde_json::json!({
        "status": "rolled_back",
        "message": "Rolled back to previous binary — server will restart shortly",
    })))
}

// ── PATCH /api/update/settings ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pre_release_channel: Option<bool>,
    poll_interval_hours: Option<u8>,
}

pub async fn update_settings(
    Extension(update): Extension<Arc<UpdateState>>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<UpdateStatusResponse>, AppError> {
    let mut should_notify = false;
    {
        let mut settings = update.settings.lock().await;

        if let Some(pre) = req.pre_release_channel
            && settings.pre_release_channel != pre
        {
            settings.pre_release_channel = pre;
            should_notify = true;
        }

        if let Some(hours) = req.poll_interval_hours {
            if !crate::update::is_valid_poll_interval_hours(hours) {
                return Err(AppError::bad_request(
                    "poll_interval_hours must be one of 0, 1, 6, or 24",
                ));
            }

            if settings.poll_interval_hours != hours {
                settings.poll_interval_hours = hours;
                should_notify = true;
            }
        }

        settings.save()?;
    }

    if should_notify {
        update.settings_changed.notify_one();
    }

    Ok(Json(build_update_status_response(update.as_ref()).await))
}

/// Helper to construct the WIP upload route with a 100 MB body limit.
pub fn wip_route() -> axum::routing::MethodRouter {
    axum::routing::post(upload_wip).layer(DefaultBodyLimit::max(100 * 1024 * 1024))
}

/// Build an `axum::Router` containing the three mutating update endpoints
/// protected by the `require_update_token` middleware:
///
/// - `POST /update/apply`
/// - `POST /update/wip`  (100 MB body limit)
/// - `POST /update/rollback`
///
/// The returned router is intended to be merged into the main API router.
pub fn protected_update_router() -> axum::Router {
    use axum::middleware;
    use axum::routing::post;

    axum::Router::new()
        .route("/update/apply", post(apply_update))
        .route("/update/wip", wip_route())
        .route("/update/rollback", post(rollback))
        .layer(middleware::from_fn(require_update_token))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use axum::response::IntoResponse;
    use tokio::sync::{Mutex, Notify};

    fn test_update_state() -> Arc<UpdateState> {
        Arc::new(UpdateState {
            cache: github::ReleaseCache::new(),
            settings: Mutex::new(crate::update::UpdateSettings::default()),
            swap_lock: Mutex::new(()),
            shutdown: Arc::new(Notify::new()),
            settings_changed: Arc::new(Notify::new()),
            wip_manifests: Mutex::new(HashMap::new()),
            token: crate::update::UpdateToken::generate(),
        })
    }

    fn header_map(entries: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in entries {
            headers.insert(
                axum::http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                axum::http::HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    #[test]
    fn same_origin_accepts_matching_origin_header() {
        let headers = header_map(&[
            ("host", "127.0.0.1:3141"),
            ("origin", "http://127.0.0.1:3141"),
        ]);
        assert!(is_same_origin_request(&headers));
    }

    #[test]
    fn same_origin_rejects_mismatched_origin_header() {
        let headers = header_map(&[
            ("host", "127.0.0.1:3141"),
            ("origin", "http://evil.example"),
        ]);
        assert!(!is_same_origin_request(&headers));
    }

    #[test]
    fn same_origin_accepts_matching_referer_when_origin_is_absent() {
        let headers = header_map(&[
            ("host", "127.0.0.1:3141"),
            ("referer", "http://127.0.0.1:3141/settings"),
        ]);
        assert!(is_same_origin_request(&headers));
    }

    #[test]
    fn same_origin_rejects_missing_origin_and_referer() {
        let headers = header_map(&[("host", "127.0.0.1:3141")]);
        assert!(!is_same_origin_request(&headers));
    }

    #[tokio::test]
    async fn update_settings_accepts_valid_poll_interval() {
        let update = test_update_state();
        let response = match update_settings(
            Extension(update.clone()),
            Json(UpdateSettingsRequest {
                pre_release_channel: Some(true),
                poll_interval_hours: Some(24),
            }),
        )
        .await
        {
            Ok(response) => response,
            Err(_) => panic!("expected settings update to succeed"),
        };

        assert!(response.0.pre_release_channel);
        assert_eq!(response.0.poll_interval_hours, 24);
    }

    #[tokio::test]
    async fn update_settings_rejects_invalid_poll_interval() {
        let update = test_update_state();
        let error = match update_settings(
            Extension(update),
            Json(UpdateSettingsRequest {
                pre_release_channel: None,
                poll_interval_hours: Some(12),
            }),
        )
        .await
        {
            Ok(_) => panic!("expected invalid poll interval to be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.into_response().status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_update_token_sets_no_store_cache_control() {
        let update = test_update_state();
        let (headers, body) = match get_update_token(
            Extension(update.clone()),
            header_map(&[
                ("host", "127.0.0.1:3141"),
                ("origin", "http://127.0.0.1:3141"),
            ]),
        )
        .await
        {
            Ok(response) => response,
            Err(_) => panic!("expected token request to succeed"),
        };

        assert_eq!(headers.get(CACHE_CONTROL).unwrap(), "no-store");
        assert_eq!(body.0.token, update.token.expose());
    }
}
