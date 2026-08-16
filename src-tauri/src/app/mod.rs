pub mod config;

use crate::core::logging::Logger;
use crate::update::AppUpdateInfo;
use config::ShellConfig;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;
use tauri::menu::{Menu, Submenu};
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellStatus {
    pub phase: &'static str,
    pub message: String,
    pub detail: Option<String>,
    pub url: Option<String>,
    pub progress: Option<f64>,
    pub engine_version: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RuntimeInfo {
    pub node_exe: PathBuf,
    pub dsh_bin: PathBuf,
    pub dsh_version: String,
}

pub struct AppState {
    pub home: PathBuf,
    pub engine_home: PathBuf,
    pub runtime_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub logger: Logger,
    pub config: Mutex<ShellConfig>,
    pub config_path: PathBuf,
    pub status: Mutex<ShellStatus>,
    pub log_tail: Mutex<VecDeque<String>>,
    pub engine: Mutex<Option<Child>>,
    pub ready_url: Mutex<Option<String>>,
    pub runtime: Mutex<Option<RuntimeInfo>>,
    pub app_update: Mutex<Option<AppUpdateInfo>>,
    pub stopping: AtomicBool,
    pub engine_starting: AtomicBool,
    pub generation: AtomicU64,
    pub first_run_marked: AtomicBool,
    pub runtime_broken: AtomicBool,
    pub webui_port_fallback: AtomicBool,
    pub tray: Mutex<Option<TrayIcon<tauri::Wry>>>,
    pub tray_menu: Mutex<Option<Menu<tauri::Wry>>>,
    pub tray_theme_menu: Mutex<Option<Submenu<tauri::Wry>>>,
}

impl AppState {
    pub fn new(
        home: PathBuf,
        engine_home: PathBuf,
        logs_dir: PathBuf,
        logger: Logger,
        config: ShellConfig,
        config_path: PathBuf,
    ) -> Self {
        let runtime_dir = home.join("runtime");
        Self {
            home,
            engine_home,
            runtime_dir,
            logs_dir,
            logger,
            config: Mutex::new(config),
            config_path,
            status: Mutex::new(ShellStatus {
                phase: "idle",
                message: "正在初始化…".to_string(),
                detail: None,
                url: None,
                progress: None,
                engine_version: None,
            }),
            log_tail: Mutex::new(VecDeque::new()),
            engine: Mutex::new(None),
            ready_url: Mutex::new(None),
            runtime: Mutex::new(None),
            app_update: Mutex::new(None),
            stopping: AtomicBool::new(false),
            engine_starting: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            first_run_marked: AtomicBool::new(false),
            runtime_broken: AtomicBool::new(false),
            webui_port_fallback: AtomicBool::new(false),
            tray: Mutex::new(None),
            tray_menu: Mutex::new(None),
            tray_theme_menu: Mutex::new(None),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn set_status(
    state: &AppState,
    app: Option<&AppHandle>,
    phase: &'static str,
    message: impl Into<String>,
    detail: Option<String>,
    url: Option<String>,
    progress: Option<f64>,
    engine_version: Option<String>,
) {
    let status = ShellStatus {
        phase,
        message: message.into(),
        detail,
        url,
        progress,
        engine_version,
    };
    *state.status.lock().unwrap() = status.clone();
    if let Some(app) = app {
        let _ = app.emit("shell://status", status);
        if phase == "error" {
            // Never leave the user with a hidden window when something fails.
            crate::ui::windows::show_main_window(app);
        }
    }
}

pub fn push_log_tail(state: &AppState, line: String) {
    let mut tail = state.log_tail.lock().unwrap();
    tail.push_back(line);
    while tail.len() > 300 {
        tail.pop_front();
    }
}

/// Write one shell/check log line to both the daily log file and the live
/// `shell://log` event so the detection page always has detailed output.
pub fn emit_log(state: &AppState, app: Option<&AppHandle>, level: &str, line: String) {
    state.logger.log("app", level, &line);
    push_log_tail(state, line.clone());
    if let Some(app) = app {
        let _ = app.emit(
            "shell://log",
            serde_json::json!({
                "level": level.to_lowercase(),
                "line": line,
            }),
        );
    }
}
