//! Global application state. Owned as `Arc<AppState>` and managed by Tauri
//! so every command, tray handler and background task can share it.
//!
//! The shell no longer knows about bundled runtimes: the only engine-related
//! state is the validated external `CommandSpec` produced by detection.

use crate::app::config::ShellConfig;
use crate::core::logging::Logger;
use crate::detection::model::{CommandSpec, DependencyInfo};
use crate::update::AppUpdateInfo;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;
use tauri::menu::{Menu, Submenu};
use tauri::tray::TrayIcon;
use tauri::Wry;

pub struct AppState {
    /// Desktop-shell data home (`~/.dsh-tauri-gui`).
    pub home: PathBuf,
    /// Engine data home (`$DSH_HOME`, defaults to `~/.dsh`).
    pub engine_home: PathBuf,
    pub logs_dir: PathBuf,
    pub logger: Logger,
    pub config: Mutex<ShellConfig>,
    pub config_path: PathBuf,
    /// The shell URL loaded before the WebView is navigated to dsh.
    pub shell_url: String,
    pub status: Mutex<crate::app::status::ShellStatus>,
    pub log_tail: Mutex<VecDeque<String>>,
    /// Running `dsh web` child process, when owned by this shell.
    pub engine: Mutex<Option<Child>>,
    /// Handoff information for an engine kept alive across shell restarts.
    pub engine_session: Mutex<Option<crate::app::engine_session::EngineSession>>,
    pub ready_url: Mutex<Option<String>>,
    /// Validated external toolchain from the last successful detection.
    pub command_spec: Mutex<Option<CommandSpec>>,
    /// Latest detection rows (for diagnostics and the setup screen).
    pub last_detection: Mutex<Option<Vec<DependencyInfo>>>,
    /// Current setup-flow session (cancel/active flags for installs).
    pub setup_session: std::sync::Arc<crate::detection::session::SetupSession>,
    pub app_update: Mutex<Option<AppUpdateInfo>>,
    pub dsh_update: Mutex<Option<crate::update::DshUpdateInfo>>,
    pub update_notice: Mutex<Option<crate::app::status::UpdateNotice>>,
    /// In-process geo cache (region/country only, short TTL).
    pub geo_cache: crate::geo::GeoCache,
    pub stopping: AtomicBool,
    pub engine_starting: AtomicBool,
    pub generation: AtomicU64,
    pub first_run_marked: AtomicBool,
    pub webui_port_fallback: AtomicBool,
    /// Set the first time `begin_setup` runs so the frontend can safely
    /// (re)trigger detection without spawning duplicate tasks.
    pub setup_started: AtomicBool,
    /// Forces a fresh frontend mount when the tray returns from dsh to shell.
    pub setup_revision: AtomicU64,
    pub tray: Mutex<Option<TrayIcon<Wry>>>,
    pub tray_menu: Mutex<Option<Menu<Wry>>>,
    pub tray_theme_menu: Mutex<Option<Submenu<Wry>>>,
}

impl AppState {
    pub fn new(
        home: PathBuf,
        engine_home: PathBuf,
        logs_dir: PathBuf,
        logger: Logger,
        config: ShellConfig,
        config_path: PathBuf,
        shell_url: String,
    ) -> Self {
        let engine_session = crate::app::engine_session::load(&home);
        let has_detached_port = engine_session
            .as_ref()
            .is_some_and(|session| session.port != config.webui_port);
        Self {
            home,
            engine_home,
            logs_dir,
            logger,
            config: Mutex::new(config),
            config_path,
            shell_url,
            status: Mutex::new(crate::app::status::ShellStatus {
                phase: "idle",
                code: "initializing",
                detail: None,
                url: None,
                progress: None,
                engine_version: None,
            }),
            log_tail: Mutex::new(VecDeque::new()),
            engine: Mutex::new(None),
            engine_session: Mutex::new(engine_session),
            ready_url: Mutex::new(None),
            command_spec: Mutex::new(None),
            last_detection: Mutex::new(None),
            setup_session: crate::detection::session::SetupSession::new(),
            app_update: Mutex::new(None),
            dsh_update: Mutex::new(None),
            update_notice: Mutex::new(None),
            geo_cache: crate::geo::GeoCache::default(),
            stopping: AtomicBool::new(false),
            engine_starting: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            first_run_marked: AtomicBool::new(false),
            webui_port_fallback: AtomicBool::new(has_detached_port),
            setup_started: AtomicBool::new(false),
            setup_revision: AtomicU64::new(0),
            tray: Mutex::new(None),
            tray_menu: Mutex::new(None),
            tray_theme_menu: Mutex::new(None),
        }
    }
}
