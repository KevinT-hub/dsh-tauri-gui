use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Desktop-shell settings. The dsh engine itself keeps its official config
/// (`cordis.patch.yml`, `settings.yaml`, ...) under `$DSH_HOME`.
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
    /// already been shown. `""` means no version has been acknowledged.
    pub last_checklist_version: String,
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
            npm_registry: "https://registry.npmmirror.com".to_string(),
            default_workspace: None,
            ui_theme: "system".to_string(),
            first_run_completed: false,
            last_checklist_version: String::new(),
            webui_port: 3080,
            engine_home: None,
        }
    }
}

/// The detection/checklist screen is required on the very first launch and
/// again once after every software update (app version change).
pub fn checklist_required(config: &ShellConfig, app_version: &str) -> bool {
    !config.first_run_completed || config.last_checklist_version != app_version
}

/// The detection screen is required on first launch, once after every app
/// update, and again whenever the local runtime is missing or broken.
pub fn checklist_required_full(state: &crate::app::AppState) -> bool {
    let version_gated =
        checklist_required(&state.config.lock().unwrap(), env!("CARGO_PKG_VERSION"));
    version_gated
        || state
            .runtime_broken
            .load(std::sync::atomic::Ordering::SeqCst)
}

/// Persist that the current app version's checklist has been completed.
pub fn mark_checklist_completed(state: &crate::app::AppState, app_version: &str) {
    if state
        .first_run_marked
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return;
    }
    let mut config = state.config.lock().unwrap();
    if config.first_run_completed && config.last_checklist_version == app_version {
        state
            .first_run_marked
            .store(true, std::sync::atomic::Ordering::SeqCst);
        return;
    }
    config.first_run_completed = true;
    config.last_checklist_version = app_version.to_string();
    match save(&config, &state.config_path) {
        Ok(()) => {
            state
                .first_run_marked
                .store(true, std::sync::atomic::Ordering::SeqCst);
            state.logger.info(&format!(
                "checklist completed for app version {app_version}; boot screen will be skipped"
            ));
        }
        Err(err) => {
            config.first_run_completed = false;
            config.last_checklist_version.clear();
            state
                .logger
                .warn(&format!("failed to persist checklist marker: {err}"));
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
    let parent = path.parent().ok_or("config path has no parent")?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let nonce = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let tmp = parent.join(format!(".config-{}-{nonce}.tmp", std::process::id()));
    let backup = parent.join(format!(".config-{}-{nonce}.bak", std::process::id()));
    let json = serde_json::to_string_pretty(config).map_err(|err| err.to_string())?;
    fs::write(&tmp, json).map_err(|err| err.to_string())?;
    if path.exists() {
        fs::rename(path, &backup).map_err(|err| err.to_string())?;
    }
    match fs::rename(&tmp, path) {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
            Ok(())
        }
        Err(err) => {
            if backup.exists() && !path.exists() {
                let _ = fs::rename(&backup, path);
            }
            Err(err.to_string())
        }
    }
}
