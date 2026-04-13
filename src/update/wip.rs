use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// ELF magic bytes: `\x7fELF`
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// Result of staging a WIP binary upload.
pub struct WipUpload {
    pub upload_id: String,
    pub sha256: String,
    pub size: u64,
    #[allow(dead_code)]
    pub path: PathBuf,
}

/// Validate that `data` begins with the ELF magic header.
pub fn validate_elf(data: &[u8]) -> Result<(), String> {
    if data.len() < 4 {
        return Err("File too small to be a valid ELF binary".to_string());
    }
    if data[..4] != ELF_MAGIC {
        return Err("File is not a valid ELF binary (bad magic bytes)".to_string());
    }
    Ok(())
}

/// Write the uploaded bytes to a temp file, validate, and compute metadata.
pub async fn stage_upload(data: bytes::Bytes) -> anyhow::Result<WipUpload> {
    validate_elf(&data).map_err(|e| anyhow::anyhow!(e))?;

    let size = data.len() as u64;

    // Compute SHA256
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let sha256 = format!("{:x}", hasher.finalize());

    // upload_id = first 8 hex chars of SHA256
    let upload_id = sha256[..8].to_string();

    // Write to a temp file
    let tmp_dir = std::env::temp_dir();
    let filename = format!("seeki-wip-{upload_id}");
    let path = tmp_dir.join(filename);

    tokio::fs::write(&path, &data).await?;

    // Set executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }

    tracing::info!(
        upload_id = %upload_id,
        sha256 = %sha256,
        size = size,
        path = %path.display(),
        "WIP binary staged"
    );

    Ok(WipUpload {
        upload_id,
        sha256,
        size,
        path,
    })
}
