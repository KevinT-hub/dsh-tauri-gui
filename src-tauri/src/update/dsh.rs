use crate::app::AppState;
use crate::core::http::{http_agent, USER_AGENT};
use crate::detection::{self, model::{CheckStatus, DependencyId}};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct LatestJson {
    version: String,
}

pub async fn check_update(state: &AppState) -> Result<crate::update::DshUpdateInfo, String> {
    let registry = state.config.lock().unwrap().npm_registry.clone();
    let current_version = current_version(state).unwrap_or_default();
    let latest_version = fetch_latest_version(&registry).await?;
    let available = !current_version.is_empty() && compare_versions(&latest_version, &current_version);
    Ok(crate::update::DshUpdateInfo {
        available,
        current_version,
        latest_version,
        install_command: "npm install -g @deepseek-ai/dsh@latest".to_string(),
        registry,
    })
}

fn current_version(state: &AppState) -> Option<String> {
    if let Some(spec) = state.command_spec.lock().unwrap().clone() {
        if !spec.dsh_version.trim().is_empty() {
            return Some(spec.dsh_version);
        }
    }
    if let Some(rows) = state.last_detection.lock().unwrap().clone() {
        if let Some(item) = rows.iter().find(|item| {
            item.id == DependencyId::Dsh && item.status == CheckStatus::Passed
        }) {
            if let Some(version) = &item.version {
                if !version.trim().is_empty() {
                    return Some(version.clone());
                }
            }
        }
    }
    let probe = detection::dsh::detect();
    if probe.status == CheckStatus::Passed {
        probe.version
    } else {
        None
    }
}

async fn fetch_latest_version(registry: &str) -> Result<String, String> {
    let base = registry.trim_end_matches('/');
    let endpoint = format!("{base}/@deepseek-ai%2Fdsh/latest");
    let agent = http_agent(Some(Duration::from_secs(20)));
    let mut resp = agent
        .get(&endpoint)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| err.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let latest: LatestJson = resp.body_mut().read_json().map_err(|err| err.to_string())?;
    Ok(latest.version)
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
