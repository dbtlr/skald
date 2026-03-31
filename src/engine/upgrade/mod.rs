use serde::Deserialize;
use std::time::Duration;

const GITHUB_API_URL: &str = "https://api.github.com/repos/dbtlr/skald/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub current: String,
    pub latest: String,
    pub update_available: bool,
    pub download_url: Option<String>,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Check the latest version from GitHub releases.
/// Returns None on any failure (network, parse, etc).
pub fn check_latest_version() -> Option<VersionInfo> {
    let response = ureq::AgentBuilder::new()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .get(GITHUB_API_URL)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "skald-cli")
        .call()
        .ok()?;

    let release: GitHubRelease = response.into_json().ok()?;
    let latest = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
    let current = env!("CARGO_PKG_VERSION");

    let update_available = version_is_newer(current, latest);
    let download_url = if update_available {
        current_target().map(|_| build_download_url(&release.tag_name))
    } else {
        None
    };

    Some(VersionInfo {
        current: current.to_string(),
        latest: latest.to_string(),
        update_available,
        download_url,
    })
}

/// Compare two semver-style version strings. Returns true if latest > current.
pub fn version_is_newer(current: &str, latest: &str) -> bool {
    let parse =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse::<u32>().ok()).collect() };

    let cur = parse(current);
    let lat = parse(latest);

    for i in 0..cur.len().max(lat.len()) {
        let c = cur.get(i).copied().unwrap_or(0);
        let l = lat.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    false
}

/// Returns the compile-time target triple, or None for unsupported platforms.
pub fn current_target() -> Option<&'static str> {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Some("aarch64-unknown-linux-gnu")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        Some("x86_64-apple-darwin")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Some("aarch64-apple-darwin")
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Some("x86_64-pc-windows-msvc")
    } else {
        None
    }
}

/// Build the download URL for a given release tag.
pub fn build_download_url(tag: &str) -> String {
    let target = current_target().unwrap_or("unknown");
    let ext = if cfg!(target_os = "windows") { "zip" } else { "tar.gz" };
    format!("https://github.com/dbtlr/skald/releases/download/{tag}/sk-{target}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_newer_detects_newer() {
        assert!(version_is_newer("0.1.0", "0.2.0"));
        assert!(version_is_newer("1.0.0", "2.0.0"));
        assert!(version_is_newer("1.2.3", "1.2.4"));
        assert!(version_is_newer("0.9.9", "1.0.0"));
    }

    #[test]
    fn version_is_newer_equal_returns_false() {
        assert!(!version_is_newer("1.0.0", "1.0.0"));
        assert!(!version_is_newer("0.1.0", "0.1.0"));
    }

    #[test]
    fn version_is_newer_older_returns_false() {
        assert!(!version_is_newer("2.0.0", "1.0.0"));
        assert!(!version_is_newer("1.2.4", "1.2.3"));
        assert!(!version_is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn current_target_returns_some() {
        assert!(current_target().is_some());
    }

    #[test]
    fn build_download_url_format() {
        let url = build_download_url("v0.2.0");
        let target = current_target().unwrap();
        assert_eq!(
            url,
            format!("https://github.com/dbtlr/skald/releases/download/v0.2.0/sk-{target}.tar.gz")
        );
        assert!(url.starts_with("https://github.com/dbtlr/skald/releases/download/v0.2.0/sk-"));
        assert!(url.ends_with(".tar.gz") || url.ends_with(".zip"));
    }
}
