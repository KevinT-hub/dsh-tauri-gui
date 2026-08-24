use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Desktop-shell settings. The dsh engine itself keeps its official config
/// (`cordis.patch.yml`, `settings.yaml`, ...) under `$DSH_HOME`.
///
/// Legacy `runtimeMode` / `runtimeModeSelected` fields from versions that
/// shipped a bundled runtime are intentionally *ignored* by serde (no
/// `deny_unknown_fields`), so old configs load and save without a migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShellConfig {
    pub minimize_to_tray: bool,
    pub auto_start_engine: bool,
    pub restart_on_crash: bool,
    pub telemetry_disabled: bool,
    pub npm_registry: String,
    pub default_workspace: Option<PathBuf>,
    /// light | dark | system
    pub ui_theme: String,
    pub first_run_completed: bool,
    /// App version whose checklist (first-run / post-update boot screen) has
    /// actually been completed via the final "进入" action.
    pub last_checklist_version: String,
    /// App version whose setup screen was successfully acknowledged.
    /// Kept for config compatibility; the visibility gate now follows
    /// `last_checklist_version`.
    pub setup_seen_version: String,
    /// Web UI listen port (official default 3080); 0 lets the OS pick.
    pub webui_port: u16,
    /// Engine data home override; `None` uses the official `~/.dsh`.
    pub engine_home: Option<String>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            auto_start_engine: true,
            restart_on_crash: true,
            telemetry_disabled: true,
            npm_registry: "https://registry.npmjs.org".to_string(),
            default_workspace: None,
            ui_theme: "system".to_string(),
            first_run_completed: false,
            last_checklist_version: String::new(),
            setup_seen_version: String::new(),
            webui_port: 3080,
            engine_home: None,
        }
    }
}

/// The setup/detection screen stays visible until the current app version has
/// actually been completed by clicking "进入".
pub fn checklist_required(config: &ShellConfig, app_version: &str) -> bool {
    config.last_checklist_version != app_version
}

/// Version-gated setup requirement. The screen stays open until the current
/// app version is completed once through the final "进入" action.
pub fn checklist_required_full(state: &crate::app::AppState) -> bool {
    checklist_required(&state.config.lock().unwrap(), env!("CARGO_PKG_VERSION"))
}

/// Legacy compatibility marker for the old "seen" semantics.
///
/// The new gate no longer depends on this field, so the setup screen is only
/// considered complete after the user finishes the flow through
/// `mark_checklist_completed`.
pub fn mark_setup_seen(state: &crate::app::AppState) {
    let app_version = env!("CARGO_PKG_VERSION");
    let mut config = state.config.lock().unwrap();
    if config.setup_seen_version == app_version {
        return;
    }
    config.setup_seen_version = app_version.to_string();
    if let Err(err) = save(&config, &state.config_path) {
        config.setup_seen_version.clear();
        state
            .logger
            .warn(&format!("failed to persist setup_seen_version: {err}"));
    }
}

/// Persist that the current app version's checklist has been completed and
/// the user entered the harness.
pub fn mark_checklist_completed(
    state: &crate::app::AppState,
    app_version: &str,
) -> Result<(), String> {
    if state
        .first_run_marked
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return Ok(());
    }
    let mut config = state.config.lock().unwrap();
    if config.first_run_completed
        && config.last_checklist_version == app_version
        && config.setup_seen_version == app_version
    {
        state
            .first_run_marked
            .store(true, std::sync::atomic::Ordering::SeqCst);
        return Ok(());
    }
    let previous_first_run_completed = config.first_run_completed;
    let previous_last_checklist_version = config.last_checklist_version.clone();
    let previous_setup_seen_version = config.setup_seen_version.clone();
    config.first_run_completed = true;
    config.last_checklist_version = app_version.to_string();
    config.setup_seen_version = app_version.to_string();
    match save(&config, &state.config_path) {
        Ok(()) => {
            state
                .first_run_marked
                .store(true, std::sync::atomic::Ordering::SeqCst);
            state.logger.info(&format!(
                "checklist completed for app version {app_version}; boot screen will be skipped"
            ));
            Ok(())
        }
        Err(err) => {
            config.first_run_completed = previous_first_run_completed;
            config.last_checklist_version = previous_last_checklist_version;
            config.setup_seen_version = previous_setup_seen_version;
            state
                .logger
                .warn(&format!("failed to persist checklist marker: {err}"));
            Err(err)
        }
    }
}

/// Force the next launch through the setup screen again. This is used after
/// replacing the external dsh CLI so the version probe cannot remain stale.
pub fn reset_checklist(state: &crate::app::AppState) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    let previous = (
        config.first_run_completed,
        config.last_checklist_version.clone(),
        config.setup_seen_version.clone(),
    );
    config.first_run_completed = false;
    config.last_checklist_version.clear();
    config.setup_seen_version.clear();
    match save(&config, &state.config_path) {
        Ok(()) => {
            state
                .first_run_marked
                .store(false, std::sync::atomic::Ordering::SeqCst);
            state
                .logger
                .info("checklist reset; the next launch will re-run environment detection");
            Ok(())
        }
        Err(err) => {
            config.first_run_completed = previous.0;
            config.last_checklist_version = previous.1;
            config.setup_seen_version = previous.2;
            Err(err)
        }
    }
}

pub fn load(home: &Path) -> (ShellConfig, PathBuf) {
    let path = home.join("config.json");
    let config = read_config(&path)
        .or_else(|| recover_backup(&path))
        .unwrap_or_default();
    (config, path)
}

fn read_config(path: &Path) -> Option<ShellConfig> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

/// If the main config file is missing (e.g. a crash between the delete and
/// the rename in an older version), fall back to the newest `.bak` file and
/// restore it to the canonical path.
fn recover_backup(path: &Path) -> Option<ShellConfig> {
    let parent = path.parent()?;
    let mut best: Option<(PathBuf, SystemTime)> = None;
    for entry in fs::read_dir(parent).ok()?.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with(".config-") && name.ends_with(".bak") {
            let modified = entry.metadata().and_then(|m| m.modified()).ok();
            if let Some(modified) = modified {
                if best.as_ref().is_none_or(|(_, current)| modified > *current) {
                    best = Some((entry.path(), modified));
                }
            }
        }
    }
    let (backup, _) = best?;
    let config = read_config(&backup)?;
    let _ = fs::copy(&backup, path);
    Some(config)
}

pub fn save(config: &ShellConfig, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|err| err.to_string())?;
    crate::core::filesystem::atomic_write(path, &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_runtime_fields_are_ignored() {
        let json = r#"{
            "minimizeToTray": true,
            "autoStartEngine": false,
            "restartOnCrash": true,
            "telemetryDisabled": false,
            "npmRegistry": "https://registry.npmmirror.com",
            "defaultWorkspace": null,
            "uiTheme": "system",
            "firstRunCompleted": true,
            "lastChecklistVersion": "0.1.0",
            "webuiPort": 3080,
            "engineHome": null,
            "runtimeMode": "bundled",
            "runtimeModeSelected": true
        }"#;
        let config: ShellConfig = serde_json::from_str(json).expect("legacy config must load");
        // Unknown legacy fields are dropped, not stored.
        let roundtrip = serde_json::to_string(&config).unwrap();
        assert!(!roundtrip.contains("runtimeMode"));
        assert!(!roundtrip.contains("bundled"));
        // Legacy acknowledge fields survive.
        assert_eq!(config.last_checklist_version, "0.1.0");
        assert!(config.first_run_completed);
    }

    #[test]
    fn checklist_required_tracks_last_checklist_version() {
        let mut config = ShellConfig::default();
        assert!(checklist_required(&config, "0.1.1"));
        config.setup_seen_version = "0.1.1".to_string();
        assert!(checklist_required(&config, "0.1.1"));
        config.last_checklist_version = "0.1.1".to_string();
        assert!(!checklist_required(&config, "0.1.1"));
        assert!(checklist_required(&config, "0.2.0"));
    }

    #[test]
    fn default_registry_is_official() {
        assert_eq!(
            ShellConfig::default().npm_registry,
            "https://registry.npmjs.org"
        );
    }
}
