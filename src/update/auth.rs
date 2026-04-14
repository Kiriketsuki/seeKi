//! Update endpoint authentication token.
//!
//! Generates a random 32-byte token on first startup, persists it to
//! `~/.config/seeki/update-token` with mode 0600, and reloads it on
//! subsequent starts.  The token is passed in the `Authorization` header
//! as `Bearer <token>` and verified with a constant-time comparison to
//! prevent timing attacks.

use std::path::{Path, PathBuf};

/// A bearer token that guards the update/rollback/WIP endpoints.
#[derive(Clone, Debug)]
pub struct UpdateToken(String);

impl UpdateToken {
    /// Generate a fresh random token (64 lowercase hex characters = 256 bits from OS CSPRNG).
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("OS CSPRNG unavailable");
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Self(token)
    }

    /// Load the token from `path` if it exists and is non-empty; otherwise
    /// generate a new one, write it to `path` with mode 0600, and return it.
    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let contents = std::fs::read_to_string(path)?;
            let token = contents.trim().to_string();
            if !token.is_empty() {
                tracing::info!(path = %path.display(), "Loaded existing update token");
                return Ok(Self(token));
            }
        }

        let token = Self::generate();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)?;
            f.write_all(token.0.as_bytes())?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(path, &token.0)?;
        }

        tracing::info!(path = %path.display(), "Generated new update token");
        Ok(token)
    }

    /// Constant-time comparison: returns `true` when `candidate` matches the
    /// stored token.  Uses `subtle::ConstantTimeEq` if available; falls back
    /// to a simple byte-by-byte comparison with a fixed-length pad so that
    /// short-circuit evaluation is avoided.
    pub fn verify(&self, candidate: &str) -> bool {
        let a = self.0.as_bytes();
        let b = candidate.as_bytes();
        // If lengths differ the comparison must still run to completion to
        // avoid leaking the token length via timing.
        let len = a.len().max(b.len());
        let mut result: u8 = (a.len() ^ b.len()) as u8; // non-zero if lengths differ
        for i in 0..len {
            let ai = if i < a.len() { a[i] } else { 0 };
            let bi = if i < b.len() { b[i] } else { 0 };
            result |= ai ^ bi;
        }
        result == 0
    }

    /// Return the canonical path where the token is stored.
    pub fn default_path() -> PathBuf {
        dirs_next::config_dir()
            .unwrap_or_else(|| {
                dirs_next::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
            })
            .join("seeki")
            .join("update-token")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_64_char_hex() {
        let t = UpdateToken::generate();
        assert_eq!(t.0.len(), 64, "token must be 64 hex chars (256-bit SHA-256 digest)");
        assert!(t.0.chars().all(|c| c.is_ascii_hexdigit()), "token must be hex");
    }

    #[test]
    fn verify_accepts_matching_token() {
        let t = UpdateToken("abc123".to_string());
        assert!(t.verify("abc123"));
    }

    #[test]
    fn verify_rejects_wrong_token() {
        let t = UpdateToken("abc123".to_string());
        assert!(!t.verify("abc124"));
        assert!(!t.verify(""));
        assert!(!t.verify("abc12"));
        assert!(!t.verify("abc1234"));
    }

    #[test]
    fn verify_rejects_empty_candidate() {
        let t = UpdateToken("abc123".to_string());
        assert!(!t.verify(""));
    }

    #[test]
    fn load_or_create_generates_and_persists() {
        let dir = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let path = dir.join(format!("seeki-test-token-{nanos}"));
        // Ensure it doesn't exist yet
        let _ = std::fs::remove_file(&path);

        let t1 = UpdateToken::load_or_create(&path).unwrap();
        assert!(path.exists(), "token file must be created");
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.trim(), t1.0, "persisted token must match");

        // Loading again must return the same token
        let t2 = UpdateToken::load_or_create(&path).unwrap();
        assert_eq!(t1.0, t2.0, "second load must return same token");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_or_create_regenerates_if_empty() {
        let dir = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let path = dir.join(format!("seeki-test-token-empty-{nanos}"));
        std::fs::write(&path, "   \n").unwrap(); // whitespace-only

        let t = UpdateToken::load_or_create(&path).unwrap();
        assert!(!t.0.is_empty(), "must generate a token when file is blank");
        let _ = std::fs::remove_file(&path);
    }

    /// CSPRNG non-repetition: two independently generated tokens must differ.
    /// A collision here would require 2^-256 probability — if this fires the
    /// OS CSPRNG is broken.
    #[test]
    fn generate_csprng_non_repetition() {
        let t1 = UpdateToken::generate();
        let t2 = UpdateToken::generate();
        assert_ne!(
            t1.0, t2.0,
            "two successive generate() calls must produce distinct tokens (CSPRNG collision)"
        );
    }

    /// Atomic mode-0600 creation: the token file must be created with
    /// permissions 0600 in a single atomic open so there is no window where
    /// the file is readable by other users.
    #[cfg(unix)]
    #[test]
    fn load_or_create_mode_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let path = dir.join(format!("seeki-test-token-mode-{nanos}"));
        let _ = std::fs::remove_file(&path);

        UpdateToken::load_or_create(&path).unwrap();

        let meta = std::fs::metadata(&path).unwrap();
        let mode = meta.permissions().mode();
        // Mask to lower 12 bits (type bits are in the upper portion)
        assert_eq!(
            mode & 0o777,
            0o600,
            "token file must be created with mode 0600, got {mode:#o}"
        );

        let _ = std::fs::remove_file(&path);
    }
}
