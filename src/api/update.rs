use std::str::FromStr;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::{Extension, Json};
use serde::Deserialize;

use crate::update::github;
use crate::update::swap;
use crate::update::version::SeekiVersion;
use crate::update::UpdateState;

use super::AppError;

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
    let previous_exists = current_path
        .as_ref()
        .is_some_and(|p| swap::has_previous(p));

    let (latest, update_available) = if let Some(release) = cached {
        let tag = release.tag_name.trim_start_matches('v');
        let latest_ver = SeekiVersion::from_str(tag);
        let avail = latest_ver.as_ref().is_ok_and(|v| v > &current);
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
        let avail = SeekiVersion::from_str(&tag)
            .map(|v| v > current)
            .unwrap_or(false);
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
                .ok_or_else(|| {
                    AppError::bad_request("No release available — run check first")
                })?;

            let asset = github::select_asset(&release.assets).ok_or_else(|| {
                AppError::bad_request("No compatible binary found in release assets")
            })?;

            // Download to a temp file
            let tmp_dir = std::env::temp_dir();
            let dest = tmp_dir.join(format!("seeki-update-{}", &release.tag_name));
            github::download_asset(&asset.browser_download_url, &dest).await?;

            // Try to verify SHA256 if a sidecar exists
            let sha256_url = format!("{}.sha256", &asset.browser_download_url);
            if let Ok(expected) = github::download_sha256(&sha256_url).await {
                let actual = swap::compute_sha256(&dest)?;
                if actual != expected {
                    // Clean up the downloaded file
                    let _ = std::fs::remove_file(&dest);
                    return Err(AppError::bad_request(format!(
                        "SHA256 mismatch: expected {expected}, got {actual}"
                    )));
                }
                tracing::info!("SHA256 verified: {actual}");
            } else {
                tracing::warn!("No SHA256 sidecar found — skipping verification");
            }

            swap::apply_binary(&dest, &current_path).await?;

            // Clean up temp download
            let _ = std::fs::remove_file(&dest);

            swap::schedule_exit();

            Ok(Json(serde_json::json!({
                "status": "applied",
                "message": "Update applied — server will restart shortly",
            })))
        }
        "wip" => {
            let upload_id = req.wip_upload_id.ok_or_else(|| {
                AppError::bad_request("wip_upload_id is required when source is 'wip'")
            })?;

            // Look for the staged file by naming convention
            let tmp_dir = std::env::temp_dir();
            let wip_path = tmp_dir.join(format!("seeki-wip-{upload_id}"));
            if !wip_path.exists() {
                return Err(AppError::not_found(format!(
                    "WIP upload '{upload_id}' not found"
                )));
            }

            swap::apply_binary(&wip_path, &current_path).await?;

            // Clean up staged file
            let _ = std::fs::remove_file(&wip_path);

            swap::schedule_exit();

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
    body: bytes::Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    let upload = crate::update::wip::stage_upload(body).await?;

    Ok(Json(serde_json::json!({
        "upload_id": upload.upload_id,
        "sha256": upload.sha256,
        "size": upload.size,
    })))
}

// ── POST /api/update/rollback ────────────────────────────────────────────────

pub async fn rollback() -> Result<Json<serde_json::Value>, AppError> {
    let current_path = swap::current_exe_path()?;
    swap::rollback(&current_path).await?;
    swap::schedule_exit();

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
