//! Environment for the dsh engine process: `PATH`, `DSH_HOME`, registry,
//! telemetry switch and working directory.
//!
//! The engine receives the validated `CommandSpec` and the shell config; it
//! never touches plugin content — it only hands the official CLI the same
//! data home and registry the user would use from a terminal.

use crate::app::config::ShellConfig;
use crate::detection::model::CommandSpec;
use std::path::{Path, PathBuf};

/// Prepend the tool directories (node, npm/pnpm) to the existing `PATH` so
/// dsh child processes resolve the same toolchain the detection validated.
pub fn build_path(spec: &CommandSpec) -> String {
    let mut parts = Vec::new();
    if let Some(node_bin) = spec.node_bin.as_ref().and_then(|p| p.parent()) {
        parts.push(node_bin.display().to_string());
    }
    if let Some(pm_bin) = spec.package_manager_bin.as_ref().and_then(|p| p.parent()) {
        let node_dir = spec.node_bin.as_ref().and_then(|p| p.parent());
        if node_dir != Some(pm_bin) {
            parts.push(pm_bin.display().to_string());
        }
    }
    if let Some(existing) = std::env::var_os("PATH") {
        parts.push(existing.to_string_lossy().to_string());
    }
    let separator = if cfg!(windows) { ";" } else { ":" };
    parts.join(separator)
}

/// Resolve the engine working directory: configured workspace when it
/// exists, otherwise the OS user home.
pub fn workspace_dir(config: &ShellConfig) -> PathBuf {
    config
        .default_workspace
        .as_ref()
        .filter(|path| path.is_dir())
        .cloned()
        .or_else(|| Some(crate::core::paths::user_home_dir()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Environment variables shared with the official dsh web process.
pub fn engine_env(config: &ShellConfig, engine_home: &Path) -> Vec<(String, String)> {
    vec![
        ("DSH_HOME".to_string(), engine_home.display().to_string()),
        (
            "DSH_TELEMETRY_DISABLED".to_string(),
            if config.telemetry_disabled { "1" } else { "0" }.to_string(),
        ),
        (
            "npm_config_registry".to_string(),
            config.npm_registry.clone(),
        ),
        ("NO_COLOR".to_string(), "1".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::model::CommandSpec;

    fn spec() -> CommandSpec {
        CommandSpec {
            dsh_bin: PathBuf::from("C:/node/dsh.cmd"),
            node_bin: Some(PathBuf::from("C:/node/node.exe")),
            package_manager_bin: Some(PathBuf::from("C:/node/npm.cmd")),
            dsh_version: "0.9.0".to_string(),
            node_version: Some("22.19.0".to_string()),
        }
    }

    #[test]
    fn path_prepends_tool_directories() {
        let path = build_path(&spec());
        assert!(path.contains("C:/node"));
    }

    #[test]
    fn env_includes_dsh_home_and_registry() {
        let config = ShellConfig::default();
        let home = PathBuf::from("C:/Users/test/.dsh");
        let env = engine_env(&config, &home);
        let map: std::collections::HashMap<_, _> = env.into_iter().collect();
        assert_eq!(map.get("DSH_HOME").unwrap(), "C:/Users/test/.dsh");
        assert!(map.contains_key("npm_config_registry"));
        assert_eq!(map.get("DSH_TELEMETRY_DISABLED").unwrap(), "1");
    }
}
