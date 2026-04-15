use std::str::FromStr;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::{Extension, Json, extract::Request};
use serde::Deserialize;

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

pub async fn get_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("SEEKI_VERSION"),
        "commit": env!("SEEKI_COMMIT"),
        "built_at": env!("SEEKI_BUILT_AT"),
    }))
}

// ── GET /api/update/status ───────────────────────────────────────────────────

pub async fn get_update_status(
    Extension(update): Extension<Arc<UpdateState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = update.settings.lock().await;
    let cached = update.cache.latest().await;
    let current = SeekiVersion::current();
    let current_path = swap::current_exe_path().ok();
    let previous_exists = current_path.as_ref().is_some_and(|p| swap::has_previous(p));

    let (latest, update_available) = if let Some(release) = cached {
        let tag = release.tag_name.trim_start_matches('v');
        let latest_ver = SeekiVersion::from_str(tag);
        let avail = match &latest_ver {
            Ok(v) => v > &current,
            Err(e) => {
                tracing::warn!(tag = %tag, error = %e, "Failed to parse cached release version tag");
                false
            }
        };
        (Some(tag.to_string()), avail)
    } else {
        (None, false)
    };

    Ok(Json(serde_json::json!({
        "current": current.to_string(),
        "latest": latest,
        "pre_release_channel": settings.pre_release_channel,
        "update_available": update_available,
        "previous_exists": previous_exists,
        "last_checked": settings.last_checked,
    })))
}

// ── POST /api/update/check ───────────────────────────────────────────────────

pub async fn check_update(
    Extension(update): Extension<Arc<UpdateState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pre = {
        let s = update.settings.lock().await;
        s.pre_release_channel
    };

    let release = github::check_latest(&update.cache, pre, true).await?;

    // Update last_checked timestamp
    {
        let mut s = update.settings.lock().await;
        s.last_checked = Some(chrono::Utc::now().to_rfc3339());
        if let Err(e) = s.save() {
            tracing::warn!(error = %e, "Failed to persist update settings");
        }
    }

    let current = SeekiVersion::current();

    let (tag, update_available, assets, body) = if let Some(rel) = release {
        let tag = rel.tag_name.trim_start_matches('v').to_string();
        let avail = match SeekiVersion::from_str(&tag) {
            Ok(v) => v > current,
            Err(e) => {
                tracing::warn!(tag = %tag, error = %e, "Failed to parse release version tag");
                false
            }
        };
        let assets: Vec<serde_json::Value> = rel
            .assets
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "size": a.size,
                    "url": a.browser_download_url,
                })
            })
            .collect();
        (Some(tag), avail, assets, Some(rel.body))
    } else {
        (None, false, vec![], None)
    };

    Ok(Json(serde_json::json!({
        "current": current.to_string(),
        "latest": tag,
        "update_available": update_available,
        "assets": assets,
        "release_notes": body,
    })))
}

// ── POST /api/update/apply ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ApplyRequest {
    source: String,
    wip_upload_id: Option<String>,
}

pub async fn apply_update(
    Extension(update): Extension<Arc<UpdateState>>,
    Json(req): Json<ApplyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let current_path = swap::current_exe_path()?;

    match req.source.as_str() {
        "release" => {
            // Get cached release
            let pre = {
                let s = update.settings.lock().await;
                s.pre_release_channel
            };
            let release = github::check_latest(&update.cache, pre, false)
                .await?
                .ok_or_else(|| AppError::bad_request("No release available — run check first"))?;

            let asset = github::select_asset(&release.assets).ok_or_else(|| {
                AppError::bad_request("No compatible binary found in release assets")
            })?;

            // Require the SHA256 sidecar to be present in the release manifest
            // before downloading anything — fail fast if the release is incomplete.
            let sha256_asset_name = format!("{}.sha256", &asset.name);
            let has_sha256_asset = release.assets.iter().any(|a| a.name == sha256_asset_name);
            if !has_sha256_asset {
                return Err(AppError::bad_request(
                    "Release is missing a SHA256 sidecar — refusing to apply an unverified binary",
                ));
            }

            // Download the binary into memory and hash it in one streaming pass.
            // This eliminates both the TOCTOU window (no double-read of a temp
            // file) and any temp-file leak (no file to clean up on error).
            // Done BEFORE acquiring swap_lock so /update/rollback is not blocked
            // during the network download (which can take up to 300 s).
            let (bytes, actual_sha256) = github::download_asset_bytes(&asset.browser_download_url)
                .await
                .map_err(|e| AppError::bad_request(format!("Download failed: {e}")))?;

            // Download and compare the expected SHA256 sidecar.
            let sha256_url = format!("{}.sha256", &asset.browser_download_url);
            let expected_sha256 = github::download_sha256(&sha256_url).await.map_err(|e| {
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
}

pub async fn update_settings(
    Extension(update): Extension<Arc<UpdateState>>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut settings = update.settings.lock().await;

    if let Some(pre) = req.pre_release_channel {
        settings.pre_release_channel = pre;
    }

    settings.save()?;

    Ok(Json(serde_json::json!({
        "pre_release_channel": settings.pre_release_channel,
        "last_checked": settings.last_checked,
    })))
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
