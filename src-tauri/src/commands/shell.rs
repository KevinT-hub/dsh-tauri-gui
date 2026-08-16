use crate::app::config::ShellConfig;
use crate::app::AppState;
use crate::ui::theme;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShellConfigPatch {
    pub minimize_to_tray: Option<bool>,
    pub auto_start_engine: Option<bool>,
    pub restart_on_crash: Option<bool>,
    pub telemetry_disabled: Option<bool>,
    pub npm_registry: Option<String>,
    pub default_workspace: Option<serde_json::Value>,
    pub webui_port: Option<u32>,
    pub engine_home: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistState {
    pub required: bool,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub app_version: String,
    pub dsh_version: Option<String>,
    pub node_version: Option<String>,
    pub shell_home: String,
    pub engine_home: String,
    pub runtime_dir: String,
    pub logs_dir: String,
    pub webui_port: u16,
    pub status: crate::app::ShellStatus,
    pub log_tail: Vec<String>,
}

#[tauri::command]
pub fn get_diagnostics(app: AppHandle) -> Diagnostics {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let dsh_version = state
        .runtime
        .lock()
        .unwrap()
        .as_ref()
        .map(|info| info.dsh_version.clone());
    let node_version = crate::engine::bootstrap::read_node_version(&state.runtime_dir).ok();
    let webui_port = state.config.lock().unwrap().webui_port;
    let status = state.status.lock().unwrap().clone();
    let log_tail: Vec<String> = state.log_tail.lock().unwrap().iter().cloned().collect();
    Diagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        dsh_version,
        node_version,
        shell_home: state.home.display().to_string(),
        engine_home: state.engine_home.display().to_string(),
        runtime_dir: state.runtime_dir.display().to_string(),
        logs_dir: state.logs_dir.display().to_string(),
        webui_port,
        status,
        log_tail,
    }
}

#[tauri::command]
pub fn checklist_state(app: AppHandle) -> ChecklistState {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let required = crate::app::config::checklist_required_full(&state);
    ChecklistState {
        required,
        app_version,
    }
}

#[tauri::command]
pub fn shell_ready(app: AppHandle) {
    // The shell page has painted; only now reveal the checklist window so
    // there is no black/blank flash on first launch.
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let checklist = crate::app::config::checklist_required_full(&state);
    if checklist {
        crate::ui::windows::show_main_window(&app);
    }
}

#[tauri::command]
pub fn enter_harness(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    crate::app::config::mark_checklist_completed(&state, &app_version);
    crate::engine::open_web_ui(&app, &state)
}

#[tauri::command]
pub fn shell_status(app: AppHandle) -> crate::app::ShellStatus {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let status = state.status.lock().unwrap().clone();
    status
}

#[tauri::command]
pub fn get_shell_config(app: AppHandle) -> ShellConfig {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let config = state.config.lock().unwrap().clone();
    config
}

#[tauri::command]
pub fn set_shell_config(app: AppHandle, patch: ShellConfigPatch) -> Result<ShellConfig, String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let mut config = state.config.lock().unwrap().clone();
    if let Some(value) = patch.minimize_to_tray {
        config.minimize_to_tray = value;
    }
    if let Some(value) = patch.auto_start_engine {
        config.auto_start_engine = value;
    }
    if let Some(value) = patch.restart_on_crash {
        config.restart_on_crash = value;
    }
    if let Some(value) = patch.telemetry_disabled {
        config.telemetry_disabled = value;
    }
    if let Some(value) = patch.npm_registry {
        if !value.is_empty() {
            config.npm_registry = value;
        }
    }
    if let Some(value) = patch.default_workspace {
        config.default_workspace = if value.is_null() {
            None
        } else if let Some(text) = value.as_str() {
            Some(std::path::PathBuf::from(text))
        } else {
            return Err("defaultWorkspace 必须是字符串或 null".to_string());
        };
    }
    if let Some(value) = patch.webui_port {
        if value > 65535 {
            return Err("webuiPort 必须在 0-65535 之间".to_string());
        }
        config.webui_port = value as u16;
        // A changed configured port invalidates a previous bind-fallback.
        state.webui_port_fallback.store(false, Ordering::SeqCst);
    }
    if let Some(value) = patch.engine_home {
        config.engine_home = if value.is_null() {
            None
        } else if let Some(text) = value.as_str() {
            Some(text.to_string())
        } else {
            return Err("engineHome 必须是字符串或 null".to_string());
        };
    }
    crate::app::config::save(&config, &state.config_path)?;
    *state.config.lock().unwrap() = config.clone();
    state
        .logger
        .info(&format!("shell config updated: {config:?}"));
    Ok(config)
}

#[tauri::command]
pub fn restart_engine(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = crate::engine::restart_engine(&app, &state);
    });
    Ok(())
}

fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let command = "explorer.exe";
    #[cfg(target_os = "macos")]
    let command = "open";
    #[cfg(target_os = "linux")]
    let command = "xdg-open";
    Command::new(command)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("无法打开 {}: {err}", path.display()))
}

#[tauri::command]
pub fn open_logs_dir(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let dir = state.logs_dir.clone();
    std::thread::spawn(move || {
        let _ = open_path(&dir);
    });
    Ok(())
}

#[tauri::command]
pub fn open_web_ui(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    crate::engine::open_web_ui(&app, &state)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    crate::engine::stop_engine(&state);
    app.cleanup_before_exit();
    app.exit(0);
}

#[tauri::command]
pub fn get_theme_state(app: AppHandle) -> theme::ThemeState {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    theme::theme_state(&state)
}

#[tauri::command]
pub fn set_ui_theme(app: AppHandle, mode: String) -> Result<theme::ThemeState, String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    theme::set_ui_theme(&app, &state, mode)
}

#[tauri::command]
pub fn hide_update_overlay(app: AppHandle) {
    crate::ui::windows::hide_update_overlay(&app);
}
