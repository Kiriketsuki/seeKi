use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// ELF magic bytes at offset 0: `\x7fELF`
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
/// ELF64 header is 64 bytes.
const ELF64_HEADER_LEN: usize = 64;
/// EI_CLASS offset (byte 4): 2 = ELFCLASS64
const EI_CLASS_OFFSET: usize = 4;
const ELFCLASS64: u8 = 2;
/// EI_DATA offset (byte 5): 1 = ELFDATA2LSB (little-endian)
const EI_DATA_OFFSET: usize = 5;
const ELFDATA2LSB: u8 = 1;
/// e_machine offset (bytes 18-19, LE): 0x003E = x86_64
const E_MACHINE_OFFSET: usize = 18;
const EM_X86_64: u16 = 0x003E;

/// Result of staging a WIP binary upload.
pub struct WipUpload {
    pub upload_id: String,
    pub sha256: String,
    pub size: u64,
    #[allow(dead_code)]
    pub path: PathBuf,
}

/// Validate that `data` is a valid x86_64 ELF64 little-endian binary.
///
/// Checks performed (in order):
/// - Length ≥ 64 (full ELF64 header must be present)
/// - Magic `0x7F 'E' 'L' 'F'` at offset 0
/// - EI_CLASS = 2 (ELFCLASS64) at offset 4
/// - EI_DATA = 1 (ELFDATA2LSB, little-endian) at offset 5
/// - e_machine = 0x003E (x86_64) at offsets 18-19 (LE)
pub fn validate_elf(data: &[u8]) -> Result<(), String> {
    if data.len() < ELF64_HEADER_LEN {
        return Err(format!(
            "File is too small to contain a full ELF64 header: {} bytes (need at least {ELF64_HEADER_LEN})",
            data.len()
        ));
    }
    if data[..4] != ELF_MAGIC {
        return Err(format!(
            "Bad ELF magic: expected [7f 45 4c 46], got [{:02x} {:02x} {:02x} {:02x}]",
            data[0], data[1], data[2], data[3]
        ));
    }
    let ei_class = data[EI_CLASS_OFFSET];
    if ei_class != ELFCLASS64 {
        return Err(format!(
            "EI_CLASS is {ei_class}, expected {ELFCLASS64} (ELFCLASS64) — only 64-bit ELF binaries are supported"
        ));
    }
    let ei_data = data[EI_DATA_OFFSET];
    if ei_data != ELFDATA2LSB {
        return Err(format!(
            "EI_DATA is {ei_data}, expected {ELFDATA2LSB} (ELFDATA2LSB / little-endian) — only little-endian binaries are supported"
        ));
    }
    let e_machine = u16::from_le_bytes([data[E_MACHINE_OFFSET], data[E_MACHINE_OFFSET + 1]]);
    if e_machine != EM_X86_64 {
        return Err(format!(
            "e_machine is 0x{e_machine:04x}, expected 0x{EM_X86_64:04x} (x86_64) — only x86_64 binaries are supported"
        ));
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

    /// Build a minimal valid ELF64 LE x86_64 header (64 zero bytes with
    /// fields set appropriately).
    fn valid_elf64_header() -> Vec<u8> {
        let mut buf = vec![0u8; 64];
        // ELF magic
        buf[0] = 0x7f;
        buf[1] = b'E';
        buf[2] = b'L';
        buf[3] = b'F';
        // EI_CLASS = ELFCLASS64
        buf[4] = 2;
        // EI_DATA = ELFDATA2LSB
        buf[5] = 1;
        // e_machine at offsets 18-19, LE: 0x003E = x86_64
        buf[18] = 0x3E;
        buf[19] = 0x00;
        buf
    }

    #[test]
    fn validates_elf_valid_header() {
        let hdr = valid_elf64_header();
        assert!(validate_elf(&hdr).is_ok(), "valid ELF64 x86_64 header must pass");
    }

    #[test]
    fn validates_elf_too_small() {
        // Less than 64 bytes
        let short = vec![0x7f, b'E', b'L', b'F', 2, 1, 0, 0];
        let err = validate_elf(&short).unwrap_err();
        assert!(err.contains("too small"), "got: {err}");
    }

    #[test]
    fn validates_elf_bad_magic() {
        let mut hdr = valid_elf64_header();
        hdr[0] = 0x4d; // 'M' — Windows PE
        let err = validate_elf(&hdr).unwrap_err();
        assert!(err.contains("magic") || err.contains("ELF"), "got: {err}");
    }

    #[test]
    fn validates_elf_wrong_class() {
        let mut hdr = valid_elf64_header();
        hdr[4] = 1; // ELFCLASS32
        let err = validate_elf(&hdr).unwrap_err();
        assert!(err.contains("EI_CLASS"), "got: {err}");
    }

    #[test]
    fn validates_elf_wrong_data_encoding() {
        let mut hdr = valid_elf64_header();
        hdr[5] = 2; // ELFDATA2MSB (big-endian)
        let err = validate_elf(&hdr).unwrap_err();
        assert!(err.contains("EI_DATA"), "got: {err}");
    }

    #[test]
    fn validates_elf_wrong_machine() {
        let mut hdr = valid_elf64_header();
        // Set e_machine to ARM64 (0x00B7)
        hdr[18] = 0xB7;
        hdr[19] = 0x00;
        let err = validate_elf(&hdr).unwrap_err();
        assert!(err.contains("e_machine"), "got: {err}");
    }

    #[test]
    fn validates_elf_non_elf_bytes() {
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
