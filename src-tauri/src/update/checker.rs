use super::netprobe;
use crate::update::AppUpdateInfo;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct LatestJson {
    version: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    pub_date: String,
    #[serde(default)]
    platforms: HashMap<String, PlatformEntry>,
}

#[derive(Debug, Deserialize)]
struct PlatformEntry {
    #[serde(default)]
    url: String,
    #[serde(default)]
    signature: String,
    #[serde(default)]
    sha256: String,
}

/// Checks the stable update channel. GitHub is tried first; the verified
/// mirrors are only used when GitHub is unreachable or too slow.
pub async fn check_update() -> Result<AppUpdateInfo, String> {
    let platform = current_platform_key()?;
    let mut last_error = None;

    for endpoint in netprobe::latest_json_candidates() {
        match fetch_latest_json(&endpoint).await {
            Ok(json) => {
                let entry = json.platforms.get(platform).ok_or_else(|| {
                    format!("No updater entry for platform {platform} in {endpoint}")
                })?;
                if entry.url.is_empty() || entry.signature.is_empty() || entry.sha256.is_empty() {
                    return Err(format!(
                        "Incomplete updater metadata for {platform} in {endpoint}"
                    ));
                }
                if !netprobe::is_official_download_url(&entry.url) {
                    return Err(format!(
                        "Refusing update URL outside the official GitHub release in {endpoint}"
                    ));
                }

                let current = env!("CARGO_PKG_VERSION");
                let has_update = compare_versions(&json.version, current);
                return Ok(AppUpdateInfo {
                    available: has_update,
                    version: json.version,
                    notes: json.notes,
                    date: json.pub_date,
                    download_url: entry.url.clone(),
                    sha256: entry.sha256.clone(),
                    source: endpoint,
                });
            }
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "No update endpoint available".into()))
}

async fn fetch_latest_json(endpoint: &str) -> Result<LatestJson, String> {
    let agent = netprobe::http_agent(Some(Duration::from_secs(30)));
    let mut resp = agent
        .get(endpoint)
        .header("User-Agent", "dsh-tauri-gui")
        .call()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.body_mut().read_json().map_err(|e| e.to_string())
}

fn current_platform_key() -> Result<&'static str, String> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Ok("windows-x86_64")
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        Ok("windows-aarch64")
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        Ok("darwin-x86_64")
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Ok("darwin-aarch64")
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok("linux-x86_64")
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Ok("linux-aarch64")
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64")
    )))]
    {
        Err("unsupported update platform (missing updater metadata)".to_string())
    }
}

fn compare_versions(latest: &str, current: &str) -> bool {
    let l_base = latest.split('-').next().unwrap_or(latest);
    let c_base = current.split('-').next().unwrap_or(current);
    let l: Vec<u32> = l_base.split('.').filter_map(|s| s.parse().ok()).collect();
    let c: Vec<u32> = c_base.split('.').filter_map(|s| s.parse().ok()).collect();
    for i in 0..std::cmp::max(l.len(), c.len()) {
        let lv = l.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if lv > cv {
            return true;
        }
        if lv < cv {
            return false;
        }
    }
    // Numeric parts are equal; compare the prerelease suffixes if any.
    // A stable release (no suffix) is newer than any prerelease.
    let l_pre = latest.split('-').nth(1).unwrap_or("");
    let c_pre = current.split('-').nth(1).unwrap_or("");
    match (l_pre.is_empty(), c_pre.is_empty()) {
        (true, true) => false,
        (true, false) => true,
        (false, true) => false,
        (false, false) => prerelease_parts(l_pre) > prerelease_parts(c_pre),
    }
}

fn prerelease_parts(pre: &str) -> (String, u32) {
    match pre.rsplit_once('.') {
        Some((name, number)) if number.parse::<u32>().is_ok() => {
            (name.to_string(), number.parse::<u32>().unwrap())
        }
        _ => (pre.to_string(), 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_numeric_versions() {
        assert!(compare_versions("0.1.1", "0.1.0"));
        assert!(!compare_versions("0.1.0", "0.1.0"));
        assert!(!compare_versions("0.1.0", "0.1.1"));
    }

    #[test]
    fn compares_prerelease_versions() {
        assert!(compare_versions("0.1.0-rc.7", "0.1.0-rc.6"));
        assert!(compare_versions("0.1.0-rc.10", "0.1.0-rc.9"));
        assert!(compare_versions("0.1.0", "0.1.0-rc.6"));
        assert!(!compare_versions("0.1.0-rc.6", "0.1.0"));
        assert!(!compare_versions("0.1.0-rc.6", "0.1.0-rc.7"));
    }
}
