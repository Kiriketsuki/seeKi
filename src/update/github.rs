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
    stable: Mutex<Option<CacheEntry>>,
    prerelease: Mutex<Option<CacheEntry>>,
}

struct CacheEntry {
    release: GitHubRelease,
    fetched_at: Instant,
}

impl ReleaseCache {
    pub fn new() -> Self {
        Self {
            stable: Mutex::new(None),
            prerelease: Mutex::new(None),
        }
    }

    /// Return the cached release if the cache is still warm.
    pub async fn latest(&self, include_prerelease: bool) -> Option<GitHubRelease> {
        let guard = self.cache_slot(include_prerelease).lock().await;
        guard
            .as_ref()
            .filter(|e| e.fetched_at.elapsed() < CACHE_TTL)
            .map(|e| e.release.clone())
    }

    async fn set(&self, include_prerelease: bool, release: GitHubRelease) {
        let mut guard = self.cache_slot(include_prerelease).lock().await;
        *guard = Some(CacheEntry {
            release,
            fetched_at: Instant::now(),
        });
    }

    fn cache_slot(&self, include_prerelease: bool) -> &Mutex<Option<CacheEntry>> {
        if include_prerelease {
            &self.prerelease
        } else {
            &self.stable
        }
    }
}

pub struct ReleaseCheckResult {
    pub release: Option<GitHubRelease>,
    pub fetched: bool,
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
) -> anyhow::Result<ReleaseCheckResult> {
    // Serve from cache when possible
    if !force && let Some(cached) = cache.latest(include_prerelease).await {
        return Ok(ReleaseCheckResult {
            release: Some(cached),
            fetched: false,
        });
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
            cache.set(include_prerelease, rel.clone()).await;
            Ok(ReleaseCheckResult {
                release: Some(rel),
                fetched: true,
            })
        }
        Ok(None) => Ok(ReleaseCheckResult {
            release: None,
            fetched: true,
        }),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to check GitHub for updates");
            Ok(ReleaseCheckResult {
                release: None,
                fetched: false,
            })
        }
    }
}

/// Hard cap on release asset download size (512 MB). Prevents a malicious or
/// compromised release from exhausting local disk. Current release binaries
/// are ~20 MB, so this gives ~25x headroom without risking DoS.
const MAX_RELEASE_ASSET_BYTES: u64 = 512 * 1024 * 1024;

/// Download an asset to `dest`, streaming to avoid high memory usage.
/// Downloads to a temporary `.tmp` file first, then renames atomically on success.
/// Aborts if the asset exceeds [`MAX_RELEASE_ASSET_BYTES`].
///
/// Note: the release apply path now uses [`download_asset_bytes`] to avoid a
/// TOCTOU window. This function is retained for cases where streaming to disk
/// is necessary (e.g. very large assets).
#[allow(dead_code)]
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

        // Reject early if the server advertises a too-large Content-Length.
        if let Some(len) = resp.content_length()
            && len > MAX_RELEASE_ASSET_BYTES
        {
            anyhow::bail!(
                "Release asset is {len} bytes, exceeds cap of {MAX_RELEASE_ASSET_BYTES} bytes"
            );
        }

        let mut stream = resp.bytes_stream();
        let mut file = tokio::fs::File::create(&tmp_dest).await?;
        let mut written: u64 = 0;

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            written = written.saturating_add(chunk.len() as u64);
            if written > MAX_RELEASE_ASSET_BYTES {
                anyhow::bail!(
                    "Release asset exceeded cap of {MAX_RELEASE_ASSET_BYTES} bytes mid-download"
                );
            }
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

/// Download a release asset into memory, returning `(bytes, sha256_hex)`.
///
/// Streams the response into a `Vec<u8>`, computing SHA-256 on the fly so that
/// the digest covers the exact bytes that will be installed — no temp file,
/// no TOCTOU between verify and apply.  Aborts if the download would exceed
/// [`MAX_RELEASE_ASSET_BYTES`].
pub async fn download_asset_bytes(url: &str) -> anyhow::Result<(Vec<u8>, String)> {
    use sha2::{Digest, Sha256};

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(300))
        .build()?;

    let resp = client.get(url).send().await?.error_for_status()?;

    // Reject early if the server advertises a too-large Content-Length.
    if let Some(len) = resp.content_length()
        && len > MAX_RELEASE_ASSET_BYTES
    {
        anyhow::bail!(
            "Release asset is {len} bytes, exceeds cap of {MAX_RELEASE_ASSET_BYTES} bytes"
        );
    }

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(32 * 1024 * 1024); // 32 MB initial
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;

    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        total = total.saturating_add(chunk.len() as u64);
        if total > MAX_RELEASE_ASSET_BYTES {
            anyhow::bail!(
                "Release asset exceeded cap of {MAX_RELEASE_ASSET_BYTES} bytes mid-download"
            );
        }
        hasher.update(&chunk);
        buf.extend_from_slice(&chunk);
    }

    let sha256 = format!("{:x}", hasher.finalize());
    Ok((buf, sha256))
}

/// Download the `.sha256` sidecar file and return the hex digest (first token).
pub async fn download_sha256(url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()?;

    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
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
    select_asset_for_platform(std::env::consts::OS, std::env::consts::ARCH, assets)
}

fn select_asset_for_platform<'a>(
    os: &str,
    arch: &str,
    assets: &'a [GitHubAsset],
) -> Option<&'a GitHubAsset> {
    match (os, arch) {
        ("linux", "x86_64") => assets
            .iter()
            .find(|asset| asset.name == "seeki-x86_64-linux-musl")
            .or_else(|| {
                assets
                    .iter()
                    .find(|asset| asset.name == "seeki-x86_64-linux-gnu")
            }),
        ("macos", "x86_64") => assets
            .iter()
            .find(|asset| asset.name == "seeki-x86_64-darwin"),
        ("macos", "aarch64") => assets
            .iter()
            .find(|asset| asset.name == "seeki-aarch64-darwin"),
        _ => None,
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

async fn fetch_latest_stable(client: &reqwest::Client) -> anyhow::Result<Option<GitHubRelease>> {
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
    let resp = client
        .get(GITHUB_API_BASE)
        .send()
        .await?
        .error_for_status()?;
    let releases: Vec<GitHubRelease> = resp.json().await?;
    // The first entry from the list endpoint is the most recent release
    Ok(releases.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> GitHubAsset {
        GitHubAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.com/{name}"),
            size: 42,
        }
    }

    fn release(tag_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.to_string(),
            prerelease: false,
            assets: vec![],
            body: "notes".to_string(),
            published_at: "2026-04-16T09:00:00Z".to_string(),
        }
    }

    #[test]
    fn select_asset_prefers_linux_musl() {
        let assets = vec![
            asset("seeki-x86_64-linux-gnu"),
            asset("seeki-x86_64-linux-musl"),
        ];
        assert_eq!(
            select_asset_for_platform("linux", "x86_64", &assets).map(|value| value.name.as_str()),
            Some("seeki-x86_64-linux-musl")
        );
    }

    #[test]
    fn select_asset_falls_back_to_linux_gnu() {
        let assets = vec![asset("seeki-x86_64-linux-gnu")];
        assert_eq!(
            select_asset_for_platform("linux", "x86_64", &assets).map(|value| value.name.as_str()),
            Some("seeki-x86_64-linux-gnu")
        );
    }

    #[test]
    fn select_asset_supports_macos_intel() {
        let assets = vec![asset("seeki-x86_64-darwin")];
        assert_eq!(
            select_asset_for_platform("macos", "x86_64", &assets).map(|value| value.name.as_str()),
            Some("seeki-x86_64-darwin")
        );
    }

    #[test]
    fn select_asset_supports_macos_apple_silicon() {
        let assets = vec![asset("seeki-aarch64-darwin")];
        assert_eq!(
            select_asset_for_platform("macos", "aarch64", &assets).map(|value| value.name.as_str()),
            Some("seeki-aarch64-darwin")
        );
    }

    #[test]
    fn select_asset_returns_none_for_unsupported_platform() {
        let assets = vec![asset("seeki-x86_64-linux-musl")];
        assert!(select_asset_for_platform("windows", "x86_64", &assets).is_none());
    }

    #[tokio::test]
    async fn stable_and_prerelease_cache_entries_are_separate() {
        let cache = ReleaseCache::new();
        cache.set(false, release("v26.5.0.3")).await;

        assert_eq!(
            cache.latest(false).await.map(|value| value.tag_name),
            Some("v26.5.0.3".to_string())
        );
        assert!(cache.latest(true).await.is_none());
    }

    #[tokio::test]
    async fn prerelease_cache_entry_does_not_satisfy_stable_reads() {
        let cache = ReleaseCache::new();
        cache.set(true, release("v26.5.0.3n260416g1a2b3c4")).await;

        assert_eq!(
            cache.latest(true).await.map(|value| value.tag_name),
            Some("v26.5.0.3n260416g1a2b3c4".to_string())
        );
        assert!(cache.latest(false).await.is_none());
    }
}
