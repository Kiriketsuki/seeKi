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

/// Outcome of verifying a staged WIP file against an expected SHA256.
#[derive(Debug)]
pub enum VerifyWipOutcome {
    Ok(Vec<u8>),
    Mismatch { expected: String, actual: String },
    Missing,
}

/// Read the staged WIP file at `wip_path` into memory and compare its SHA256
/// to `expected_sha256`. On mismatch (or if reading fails for non-missing
/// reasons), the staged file is deleted. Returns the verified bytes on match.
///
/// This is split out from the HTTP handler so it can be exercised by unit
/// tests without needing axum/tokio request plumbing.
pub fn verify_staged_wip(wip_path: &std::path::Path, expected_sha256: &str) -> VerifyWipOutcome {
    use sha2::{Digest, Sha256};

    let bytes = match std::fs::read(wip_path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return VerifyWipOutcome::Missing,
        Err(_) => {
            let _ = std::fs::remove_file(wip_path);
            return VerifyWipOutcome::Mismatch {
                expected: expected_sha256.to_string(),
                actual: String::from("<unreadable>"),
            };
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());

    if actual != expected_sha256 {
        let _ = std::fs::remove_file(wip_path);
        return VerifyWipOutcome::Mismatch {
            expected: expected_sha256.to_string(),
            actual,
        };
    }

    VerifyWipOutcome::Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn sha256_hex(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        format!("{:x}", h.finalize())
    }

    fn tmp_path(suffix: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("seeki-test-{nanos}-{suffix}"));
        p
    }

    #[test]
    fn validates_elf_magic() {
        assert!(validate_elf(b"\x7fELF\x02\x01\x01\x00").is_ok());
        assert!(validate_elf(b"MZ\x90\x00").is_err());
        assert!(validate_elf(b"abc").is_err());
    }

    #[test]
    fn verify_staged_wip_matches() {
        let path = tmp_path("match");
        let bytes = b"\x7fELFhello world".to_vec();
        std::fs::write(&path, &bytes).unwrap();
        let expected = sha256_hex(&bytes);

        match verify_staged_wip(&path, &expected) {
            VerifyWipOutcome::Ok(got) => assert_eq!(got, bytes),
            other => panic!("expected Ok, got {other:?}"),
        }

        // File should still be there on success; apply handler cleans up.
        assert!(path.exists(), "verify should not delete on match");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn verify_staged_wip_rejects_tampered_and_deletes() {
        let path = tmp_path("tamper");
        let original = b"\x7fELForiginal contents";
        std::fs::write(&path, original).unwrap();
        let expected = sha256_hex(original);

        // Simulate tamper: overwrite the staged file with different bytes.
        let tampered = b"\x7fELFevil contents -- longer";
        std::fs::write(&path, tampered).unwrap();

        match verify_staged_wip(&path, &expected) {
            VerifyWipOutcome::Mismatch { expected: e, actual } => {
                assert_eq!(e, expected);
                assert_ne!(actual, expected);
                assert_eq!(actual, sha256_hex(tampered));
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }

        assert!(!path.exists(), "mismatch must delete staged file");
    }

    #[test]
    fn verify_staged_wip_missing_file() {
        let path = tmp_path("missing");
        assert!(!path.exists());
        match verify_staged_wip(&path, "deadbeef") {
            VerifyWipOutcome::Missing => {}
            other => panic!("expected Missing, got {other:?}"),
        }
    }
}
