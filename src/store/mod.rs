pub mod presets;
pub mod settings;
pub mod ui_state;

use std::path::PathBuf;

use anyhow::Context;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};

/// Persistent local store — SQLite at `~/.seeki/seeki.db` (or `$XDG_DATA_HOME/seeki/seeki.db`).
///
/// The pool is internally Arc'd by sqlx so `Store` is cheap to clone.
/// Sub-modules (`settings`, `presets`, `ui_state`) expose free functions taking `&SqlitePool`.
#[derive(Clone)]
pub struct Store(SqlitePool);

impl Store {
    /// Open (or create) the store at the default path.
    pub async fn open() -> anyhow::Result<Self> {
        let path = store_path();
        Self::open_at(&path).await
    }

    /// Open (or create) the store at a given path, with corrupt-recovery.
    ///
    /// On any open/migration failure, renames the existing file to
    /// `<name>.bak.<unix_timestamp>` and starts fresh rather than refusing to start.
    pub(crate) async fn open_at(path: &PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating store directory: {}", parent.display()))?;
        }

        match Self::try_open(path).await {
            Ok(store) => Ok(store),
            Err(e) if is_migration_error(&e) => {
                // A migration failure on a valid database must NOT trigger the
                // rename-and-retry path — that would silently discard user data.
                // Propagate so the caller surfaces a clear diagnostic instead.
                Err(e)
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "store open failed — renaming corrupt file and starting fresh"
                );
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let bak = path.with_extension(format!("db.bak.{ts}"));
                let _ = std::fs::rename(path, &bak);
                Self::try_open(path).await
            }
        }
    }

    pub(crate) async fn try_open(path: &PathBuf) -> anyhow::Result<Self> {
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let opts = url
            .parse::<SqliteConnectOptions>()
            .context("parsing sqlite url")?
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(opts)
            .await
            .with_context(|| format!("opening store at {}", path.display()))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("running store migrations")?;

        tracing::info!(path = %path.display(), "store ready");
        Ok(Self(pool))
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.0
    }
}

/// Returns true if the anyhow error chain contains a sqlx migration error.
/// Used in `open_at` to distinguish migration failures from file corruption so
/// we never rename (and thus discard) a valid user database on a migration bug.
fn is_migration_error(e: &anyhow::Error) -> bool {
    e.chain()
        .any(|c| c.downcast_ref::<sqlx::migrate::MigrateError>().is_some())
}

/// Build a stable, non-sensitive connection identifier from a database URL.
///
/// Format: `host:port/dbname` — credentials are never included.
/// Falls back to `"default"` if the URL can't be parsed.
///
/// **Key-stability contract**: the key is stable as long as the host, port, and
/// database name remain unchanged. Changing any of these (e.g. adding an explicit
/// port that was previously implicit, changing the hostname alias) produces a
/// different key and silently orphans previously stored presets and UI state.
/// This is intentional — data is keyed per connection — but should be documented
/// to users if the URL is reconfigured.
pub fn connection_id(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let host = parsed.host_str().unwrap_or("unknown");
        let port = parsed
            .port()
            .map(|p| format!(":{p}"))
            .unwrap_or_default();
        let path = parsed.path().trim_start_matches('/');
        format!("{host}{port}/{path}")
    } else {
        "default".to_string()
    }
}

/// Resolve the store path:
/// `$XDG_DATA_HOME/seeki/seeki.db` → `~/.seeki/seeki.db`
pub fn store_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg).join("seeki").join("seeki.db")
    } else {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".seeki")
            .join("seeki.db")
    }
}

#[cfg(test)]
pub mod testutil {
    use super::*;
    use tempfile::TempDir;

    /// Open an ephemeral store in a temporary directory.
    /// Returns both the store and the `TempDir` guard — drop the guard to clean up.
    pub async fn ephemeral_store() -> (Store, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("seeki.db");
        let store = Store::try_open(&path).await.expect("ephemeral store");
        (store, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn connection_id_host_port_dbname() {
        assert_eq!(
            connection_id("postgres://user:pass@prod-host:5432/fleet"),
            "prod-host:5432/fleet"
        );
    }

    #[test]
    fn connection_id_no_port() {
        assert_eq!(
            connection_id("postgres://user:pass@prod-host/mydb"),
            "prod-host/mydb"
        );
    }

    #[test]
    fn connection_id_different_dbnames_differ() {
        let a = connection_id("postgres://user@host:5432/alpha");
        let b = connection_id("postgres://user@host:5432/beta");
        assert_ne!(a, b);
    }

    #[test]
    fn connection_id_same_url_stable() {
        let url = "postgres://user:pass@host:5432/db";
        assert_eq!(connection_id(url), connection_id(url));
    }

    #[test]
    fn connection_id_unparseable_falls_back() {
        assert_eq!(connection_id("not a url"), "default");
    }

    #[tokio::test]
    async fn open_creates_db_and_runs_migrations() {
        let (_store, dir) = testutil::ephemeral_store().await;
        let db_path = dir.path().join("seeki.db");
        assert!(db_path.exists(), "db file should be created");
    }

    #[tokio::test]
    async fn open_recovers_from_corrupt_db() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("seeki.db");

        // Write garbage to simulate corruption
        std::fs::write(&path, b"this is not sqlite").unwrap();

        // open() should recover without panicking
        let store = Store::open_at(&path).await;
        assert!(store.is_ok(), "should recover from corrupt db");

        // The corrupt file should have been renamed to a .bak
        let bak_count = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains("db.bak")
            })
            .count();
        assert_eq!(bak_count, 1, "corrupt file should be renamed to .bak");
    }
}
