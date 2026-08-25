//! Small, recoverable cache for the last successfully validated toolchain.
//!
//! The cache only accelerates normal startup. Every cached executable path is
//! checked again before use, and the regular environment detection still runs
//! in the background. A missing or stale cache simply falls back to detection.

use crate::detection::model::CommandSpec;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CACHE_FILE: &str = "toolchain-cache.json";
const CACHE_VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolchainCache {
    schema_version: u8,
    dsh_bin: PathBuf,
    node_bin: Option<PathBuf>,
    package_manager_bin: Option<PathBuf>,
    dsh_version: String,
    node_version: Option<String>,
}

fn cache_path(home: &Path) -> PathBuf {
    home.join(CACHE_FILE)
}

pub fn save(home: &Path, spec: &CommandSpec) -> Result<(), String> {
    let cache = ToolchainCache {
        schema_version: CACHE_VERSION,
        dsh_bin: spec.dsh_bin.clone(),
        node_bin: spec.node_bin.clone(),
        package_manager_bin: spec.package_manager_bin.clone(),
        dsh_version: spec.dsh_version.clone(),
        node_version: spec.node_version.clone(),
    };
    let text = serde_json::to_string_pretty(&cache).map_err(|err| err.to_string())?;
    crate::core::filesystem::atomic_write(&cache_path(home), &text)
}

pub fn load(home: &Path) -> Option<CommandSpec> {
    let text = std::fs::read_to_string(cache_path(home)).ok()?;
    let cache: ToolchainCache = serde_json::from_str(&text).ok()?;
    if cache.schema_version != CACHE_VERSION
        || !cache.dsh_bin.is_file()
        || !cache.node_bin.as_ref().is_none_or(|path| path.is_file())
        || !cache
            .package_manager_bin
            .as_ref()
            .is_none_or(|path| path.is_file())
    {
        return None;
    }
    Some(CommandSpec {
        dsh_bin: cache.dsh_bin,
        node_bin: cache.node_bin,
        package_manager_bin: cache.package_manager_bin,
        dsh_version: cache.dsh_version,
        node_version: cache.node_version,
    })
}

pub fn clear(home: &Path) {
    let _ = std::fs::remove_file(cache_path(home));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_home() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home = std::env::temp_dir().join(format!("dsh-toolchain-cache-{nonce}"));
        std::fs::create_dir_all(&home).unwrap();
        home
    }

    #[test]
    fn round_trips_only_existing_toolchain_paths() {
        let home = temp_home();
        let dsh = home.join("dsh.cmd");
        let node = home.join("node.exe");
        std::fs::write(&dsh, "dsh").unwrap();
        std::fs::write(&node, "node").unwrap();
        let spec = CommandSpec {
            dsh_bin: dsh.clone(),
            node_bin: Some(node.clone()),
            package_manager_bin: None,
            dsh_version: "0.9.0".to_string(),
            node_version: Some("22.0.0".to_string()),
        };

        save(&home, &spec).unwrap();
        let loaded = load(&home).expect("valid cache should load");
        assert_eq!(loaded.dsh_bin, dsh);
        assert_eq!(loaded.node_bin, Some(node));
        assert_eq!(loaded.dsh_version, "0.9.0");

        std::fs::remove_file(&loaded.dsh_bin).unwrap();
        assert!(
            load(&home).is_none(),
            "stale executable paths must be rejected"
        );
        clear(&home);
        std::fs::remove_dir_all(home).unwrap();
    }
}
