use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const CACHE_TTL: Duration = Duration::from_secs(15 * 60); // 15 minutes
const GITHUB_API_BASE: &str = "https://api.github.com/repos/Kiriketsuki/seeKi/releases";
const USER_AGENT: &str = "seeki-updater";

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<GitHubAsset>,
    pub body: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

// ── Release cache ────────────────────────────────────────────────────────────

pub struct ReleaseCache {
    inner: Mutex<Option<CacheEntry>>,
}

struct CacheEntry {
    release: GitHubRelease,
    fetched_at: Instant,
}

impl ReleaseCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Return the cached release if the cache is still warm.
    pub async fn latest(&self) -> Option<GitHubRelease> {
        let guard = self.inner.lock().await;
        guard
            .as_ref()
            .filter(|e| e.fetched_at.elapsed() < CACHE_TTL)
            .map(|e| e.release.clone())
    }

    async fn set(&self, release: GitHubRelease) {
        let mut guard = self.inner.lock().await;
        *guard = Some(CacheEntry {
            release,
            fetched_at: Instant::now(),
        });
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Fetch the latest release from GitHub.
///
/// Returns `None` (and logs a warning) if GitHub is unreachable rather than
/// propagating the error.  The caller can always retry later.
///
/// When `force` is `false` the cached response is returned if it is less
/// than 15 minutes old.
pub async fn check_latest(
    cache: &ReleaseCache,
    include_prerelease: bool,
    force: bool,
) -> anyhow::Result<Option<GitHubRelease>> {
    // Serve from cache when possible
    if !force
        && let Some(cached) = cache.latest().await
    {
        return Ok(Some(cached));
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .build()?;

    let release = if include_prerelease {
        fetch_newest_prerelease(&client).await
    } else {
        fetch_latest_stable(&client).await
    };

    match release {
        Ok(Some(rel)) => {
            cache.set(rel.clone()).await;
            Ok(Some(rel))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to check GitHub for updates");
            Ok(None)
        }
    }
}

/// Download an asset to `dest`, streaming to avoid high memory usage.
/// Downloads to a temporary `.tmp` file first, then renames atomically on success.
pub async fn download_asset(url: &str, dest: &Path) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(300))
        .build()?;

    let tmp_dest = {
        let mut p = dest.as_os_str().to_owned();
        p.push(".tmp");
        std::path::PathBuf::from(p)
    };

    let result: anyhow::Result<()> = async {
        let resp = client.get(url).send().await?.error_for_status()?;
        let mut stream = resp.bytes_stream();
        let mut file = tokio::fs::File::create(&tmp_dest).await?;

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        drop(file);

        tokio::fs::rename(&tmp_dest, dest).await?;
        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&tmp_dest).await;
    }

    result
}

/// Download the `.sha256` sidecar file and return the hex digest (first token).
pub async fn download_sha256(url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()?;

    let body = client.get(url).send().await?.error_for_status()?.text().await?;
    // Format is typically "<hex>  <filename>" or just "<hex>"
    let hex = body
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty sha256 sidecar file"))?
        .to_lowercase();
    Ok(hex)
}

/// Select the best matching binary asset for the current platform.
///
/// Prefers `seeki-x86_64-linux-musl`, falls back to `seeki-x86_64-linux-gnu`.
pub fn select_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    assets
        .iter()
        .find(|a| a.name == "seeki-x86_64-linux-musl")
        .or_else(|| assets.iter().find(|a| a.name == "seeki-x86_64-linux-gnu"))
}

// ── Internal helpers ─────────────────────────────────────────────────────────

async fn fetch_latest_stable(
    client: &reqwest::Client,
) -> anyhow::Result<Option<GitHubRelease>> {
    let url = format!("{GITHUB_API_BASE}/latest");
    let resp = client.get(&url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let resp = resp.error_for_status()?;
    let release: GitHubRelease = resp.json().await?;
    Ok(Some(release))
}

async fn fetch_newest_prerelease(
    client: &reqwest::Client,
) -> anyhow::Result<Option<GitHubRelease>> {
    let resp = client.get(GITHUB_API_BASE).send().await?.error_for_status()?;
    let releases: Vec<GitHubRelease> = resp.json().await?;
    // The first entry from the list endpoint is the most recent release
    Ok(releases.into_iter().next())
}
