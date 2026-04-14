pub mod github;
pub mod swap;
pub mod version;
pub mod wip;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub use github::ReleaseCache;
#[allow(unused_imports)]
pub use version::SeekiVersion;

// ── Persistent update settings ───────────────────────────────────────────────

/// User-facing update preferences, persisted to `~/.seeki/update.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateSettings {
    pub pre_release_channel: bool,
    pub last_checked: Option<String>,
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
}

impl UpdateState {
    pub fn new() -> Self {
        Self {
            cache: ReleaseCache::new(),
            settings: Mutex::new(UpdateSettings::load()),
            swap_lock: Mutex::new(()),
            shutdown: std::sync::Arc::new(tokio::sync::Notify::new()),
        }
    }
}
