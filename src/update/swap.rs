use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Resolve the path to the currently running executable.
pub fn current_exe_path() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let canonical = std::fs::canonicalize(exe)?;
    Ok(canonical)
}

/// Return the `.prev` companion path for the given binary.
/// e.g. `/usr/local/bin/seeki` → `/usr/local/bin/seeki.prev`
pub fn prev_path(current: &Path) -> PathBuf {
    let mut p = current.as_os_str().to_owned();
    p.push(".prev");
    PathBuf::from(p)
}

/// Check whether a `.prev` rollback binary exists.
pub fn has_previous(current: &Path) -> bool {
    prev_path(current).exists()
}

/// Replace the running binary with `new_binary`:
///
/// 1. Copy `new_binary` → staging path (`.new`) — current untouched
/// 2. Set executable permission on the staged copy
/// 3. Rename current → `.prev`
/// 4. Rename `.new` → current (with recovery on failure)
///
/// The caller is responsible for triggering a process exit/restart.
///
/// Note: the release apply path now uses [`apply_binary_bytes`] to avoid a
/// TOCTOU window. This function is retained for cases where the source is
/// already on disk (e.g. a pre-downloaded file).
#[allow(dead_code)]
pub async fn apply_binary(new_binary: &Path, current: &Path) -> anyhow::Result<()> {
    let prev = prev_path(current);
    let staging = {
        let mut s = current.as_os_str().to_owned();
        s.push(".new");
        PathBuf::from(s)
    };

    // 1. Copy new binary to staging path (safe — current untouched)
    std::fs::copy(new_binary, &staging)?;

    // 2. Mark executable on the staged copy
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&staging, perms)?;
    }

    // 3. Move current aside
    std::fs::rename(current, &prev)?;

    // 4. Move staged binary into place
    if let Err(e) = std::fs::rename(&staging, current) {
        // Recovery: restore the original binary
        if let Err(restore_err) = std::fs::rename(&prev, current) {
            tracing::error!(
                error = %restore_err,
                "CRITICAL: failed to restore original binary from .prev"
            );
        }
        return Err(e.into());
    }

    tracing::info!(
        new_sha256 = %compute_sha256(current)?,
        "Binary updated successfully"
    );

    Ok(())
}

/// Like [`apply_binary`], but consumes an already-verified in-memory buffer
/// rather than a source path. Writing the staged binary directly from bytes
/// closes the TOCTOU window that would otherwise exist between a
/// hash-by-path and a subsequent copy-by-path (an attacker with write access
/// to the source path could swap the file between the two reads).
pub async fn apply_binary_bytes(bytes: &[u8], current: &Path) -> anyhow::Result<()> {
    let prev = prev_path(current);
    let staging = {
        let mut s = current.as_os_str().to_owned();
        s.push(".new");
        PathBuf::from(s)
    };

    // 1. Write bytes to staging path (current untouched)
    std::fs::write(&staging, bytes)?;

    // 2. Mark executable on the staged copy
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&staging, perms)?;
    }

    // 3. Move current aside
    std::fs::rename(current, &prev)?;

    // 4. Move staged binary into place
    if let Err(e) = std::fs::rename(&staging, current) {
        if let Err(restore_err) = std::fs::rename(&prev, current) {
            tracing::error!(
                error = %restore_err,
                "CRITICAL: failed to restore original binary from .prev"
            );
        }
        return Err(e.into());
    }

    tracing::info!(
        new_sha256 = %compute_sha256(current)?,
        "Binary updated successfully (from verified bytes)"
    );

    Ok(())
}

/// Swap the current binary back to the previous version:
///
/// 1. Rename current → `.tmp`
/// 2. Rename `.prev` → current (with recovery on failure)
/// 3. Rename `.tmp` → `.prev` (non-critical cleanup)
pub async fn rollback(current: &Path) -> anyhow::Result<()> {
    let prev = prev_path(current);
    if !prev.exists() {
        anyhow::bail!("No previous binary found at {}", prev.display());
    }

    let tmp = {
        let mut t = current.as_os_str().to_owned();
        t.push(".tmp");
        PathBuf::from(t)
    };

    // Step 1: move current aside
    std::fs::rename(current, &tmp)?;

    // Step 2: move prev into current
    if let Err(e) = std::fs::rename(&prev, current) {
        // Recovery: restore current from tmp
        if let Err(restore_err) = std::fs::rename(&tmp, current) {
            tracing::error!(
                error = %restore_err,
                "CRITICAL: failed to restore binary from .tmp during rollback"
            );
        }
        return Err(e.into());
    }

    // Step 3: move tmp to prev (non-critical — current is already restored)
    if let Err(e) = std::fs::rename(&tmp, &prev) {
        tracing::warn!(error = %e, "Failed to move old binary to .prev — cleaning up");
        let _ = std::fs::remove_file(&tmp);
    }

    tracing::info!("Rolled back to previous binary");
    Ok(())
}

/// Schedule a graceful server shutdown after a short delay (500 ms) so the
/// in-flight HTTP response has time to flush. Signals the axum server to
/// shut down cleanly rather than hard-killing the process.
pub fn schedule_exit(shutdown: &std::sync::Arc<tokio::sync::Notify>) {
    let shutdown = std::sync::Arc::clone(shutdown);
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        tracing::info!("Initiating graceful shutdown for update/rollback…");
        shutdown.notify_one();
    });
}

/// Compute the SHA-256 digest of a file and return it as a lowercase hex string.
pub fn compute_sha256(path: &Path) -> anyhow::Result<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let result = hasher.finalize();
    Ok(format!("{result:x}"))
}
