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
/// 1. Rename current → `.prev`
/// 2. Copy `new_binary` → current path
/// 3. Set executable permission on the new binary
///
/// The caller is responsible for triggering a process exit/restart.
pub async fn apply_binary(new_binary: &Path, current: &Path) -> anyhow::Result<()> {
    let prev = prev_path(current);

    // 1. Move the current binary aside
    std::fs::rename(current, &prev)?;

    // 2. Copy the new binary into place
    std::fs::copy(new_binary, current)?;

    // 3. Mark executable (rwxr-xr-x)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(current, perms)?;
    }

    tracing::info!(
        new_sha256 = %compute_sha256(current)?,
        "Binary updated successfully"
    );

    Ok(())
}

/// Swap the current binary back to the previous version:
///
/// 1. Rename current → `.tmp`
/// 2. Rename `.prev` → current
/// 3. Rename `.tmp` → `.prev`
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

    // Atomic three-step swap
    std::fs::rename(current, &tmp)?;
    std::fs::rename(&prev, current)?;
    std::fs::rename(&tmp, &prev)?;

    tracing::info!("Rolled back to previous binary");
    Ok(())
}

/// Schedule a graceful process exit after a short delay (500 ms) so the
/// in-flight HTTP response has time to flush.
pub fn schedule_exit() {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        tracing::info!("Exiting for update/rollback…");
        std::process::exit(0);
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
