pub mod auth;
pub mod github;
pub mod poller;
pub mod swap;
pub mod version;
pub mod wip;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub use auth::UpdateToken;
pub use github::ReleaseCache;
#[allow(unused_imports)]
pub use version::SeekiVersion;

fn default_poll_interval_hours() -> u8 {
    6
}

pub fn is_valid_poll_interval_hours(value: u8) -> bool {
    matches!(value, 0 | 1 | 6 | 24)
}

// ── Persistent update settings ───────────────────────────────────────────────

/// User-facing update preferences, persisted to `~/.seeki/update.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    pub pre_release_channel: bool,
    #[serde(default = "default_poll_interval_hours")]
    pub poll_interval_hours: u8,
    pub last_checked: Option<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            pre_release_channel: false,
            poll_interval_hours: default_poll_interval_hours(),
            last_checked: None,
        }
    }
}

impl UpdateSettings {
    /// Load settings from disk, returning defaults if the file is missing or
    /// unparseable.
    pub fn load() -> Self {
        let path = Self::settings_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Persist current settings to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn settings_path() -> PathBuf {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".seeki")
            .join("update.json")
    }
}

// ── Shared state ─────────────────────────────────────────────────────────────

/// Shared state for the update subsystem, stored as an `Extension` on the
/// router so it is available in both setup and normal mode.
pub struct UpdateState {
    pub cache: ReleaseCache,
    pub settings: Mutex<UpdateSettings>,
    pub swap_lock: Mutex<()>,
    pub shutdown: std::sync::Arc<tokio::sync::Notify>,
    pub settings_changed: std::sync::Arc<tokio::sync::Notify>,
    /// Server-side record of each staged WIP upload's SHA256, keyed by
    /// `upload_id`. Populated by the upload handler and consumed (removed) by
    /// the apply handler so the expected digest is never round-tripped through
    /// the client — this closes the same-user tampering loophole that would
    /// otherwise let an attacker swap the staged file and send the matching
    /// recomputed hash on apply.
    pub wip_manifests: Mutex<HashMap<String, String>>,
    /// Bearer token that must be supplied in the `Authorization` header when
    /// calling the mutating update endpoints (`/update/apply`, `/update/wip`,
    /// `/update/rollback`).
    pub token: UpdateToken,
}

impl UpdateState {
    pub fn new() -> Self {
        let token = UpdateToken::load_or_create(&UpdateToken::default_path())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to load/create update token from disk — generating ephemeral token");
                UpdateToken::generate()
            });
        Self {
            cache: ReleaseCache::new(),
            settings: Mutex::new(UpdateSettings::load()),
            swap_lock: Mutex::new(()),
            shutdown: std::sync::Arc::new(tokio::sync::Notify::new()),
            settings_changed: std::sync::Arc::new(tokio::sync::Notify::new()),
            wip_manifests: Mutex::new(HashMap::new()),
            token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_settings_defaults_poll_interval_to_six_hours() {
        let settings = UpdateSettings::default();
        assert_eq!(settings.poll_interval_hours, 6);
        assert!(!settings.pre_release_channel);
        assert!(settings.last_checked.is_none());
    }

    #[test]
    fn update_settings_deserialize_old_shape_with_default_interval() {
        let settings: UpdateSettings = serde_json::from_str(
            r#"{"pre_release_channel":true,"last_checked":"2026-04-16T09:00:00Z"}"#,
        )
        .unwrap();

        assert!(settings.pre_release_channel);
        assert_eq!(settings.poll_interval_hours, 6);
        assert_eq!(
            settings.last_checked.as_deref(),
            Some("2026-04-16T09:00:00Z")
        );
    }

    #[test]
    fn update_settings_roundtrip_preserves_poll_interval() {
        let settings = UpdateSettings {
            pre_release_channel: true,
            poll_interval_hours: 24,
            last_checked: Some("2026-04-16T09:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&settings).unwrap();
        let restored: UpdateSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.poll_interval_hours, 24);
        assert!(restored.pre_release_channel);
        assert_eq!(
            restored.last_checked.as_deref(),
            Some("2026-04-16T09:00:00Z")
        );
    }

    #[test]
    fn poll_interval_validation_accepts_allowed_values() {
        assert!(is_valid_poll_interval_hours(0));
        assert!(is_valid_poll_interval_hours(1));
        assert!(is_valid_poll_interval_hours(6));
        assert!(is_valid_poll_interval_hours(24));
        assert!(!is_valid_poll_interval_hours(12));
    }
}
