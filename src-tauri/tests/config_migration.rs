//! Integration test: legacy config files that carry `runtimeMode` /
//! `runtimeModeSelected` (from bundled-runtime versions) must load, round
//! trip and save without a destructive migration.

mod common;

use common::temp_home;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShellConfig {
    minimize_to_tray: bool,
    auto_start_engine: bool,
    restart_on_crash: bool,
    telemetry_disabled: bool,
    npm_registry: String,
    default_workspace: Option<std::path::PathBuf>,
    ui_theme: String,
    first_run_completed: bool,
    last_checklist_version: String,
    setup_seen_version: String,
    webui_port: u16,
    engine_home: Option<String>,
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

#[test]
fn legacy_runtime_fields_load_and_are_dropped_on_save() {
    let legacy = r#"{
        "minimizeToTray": true,
        "autoStartEngine": false,
        "restartOnCrash": true,
        "telemetryDisabled": false,
        "npmRegistry": "https://registry.npmmirror.com",
        "defaultWorkspace": null,
        "uiTheme": "dark",
        "firstRunCompleted": true,
        "lastChecklistVersion": "0.1.0",
        "setupSeenVersion": "0.1.0",
        "webuiPort": 3080,
        "engineHome": null,
        "runtimeMode": "bundled",
        "runtimeModeSelected": true
    }"#;

    let config: ShellConfig = serde_json::from_str(legacy).expect("legacy config must parse");
    assert!(config.first_run_completed);
    assert_eq!(config.last_checklist_version, "0.1.0");
    assert_eq!(config.setup_seen_version, "0.1.0");
    assert_eq!(config.ui_theme, "dark");
    assert_eq!(config.npm_registry, "https://registry.npmmirror.com");

    let saved = serde_json::to_string(&config).unwrap();
    assert!(
        !saved.contains("runtimeMode"),
        "runtimeMode must not be saved"
    );
    assert!(
        !saved.contains("bundled"),
        "legacy runtime value must not leak"
    );
    assert!(!saved.contains("runtimeModeSelected"));
}

#[test]
fn unknown_fields_never_break_loading() {
    let with_extra = r#"{
        "minimizeToTray": true,
        "autoStartEngine": true,
        "restartOnCrash": true,
        "telemetryDisabled": true,
        "npmRegistry": "https://registry.npmjs.org",
        "defaultWorkspace": null,
        "uiTheme": "system",
        "firstRunCompleted": false,
        "lastChecklistVersion": "",
        "setupSeenVersion": "",
        "webuiPort": 3080,
        "engineHome": null,
        "someFutureField": {"nested": true},
        "anotherUnknown": 42
    }"#;
    let config: ShellConfig = serde_json::from_str(with_extra).expect("extra fields are ignored");
    assert!(!config.first_run_completed);
    assert_eq!(config.webui_port, 3080);
}

#[test]
fn temp_home_roundtrip_preserves_config() {
    let home = temp_home::temp_home();
    let path = home.join("config.json");
    std::fs::write(&path, LEGACY_CONFIG).unwrap();
    let loaded: ShellConfig =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).expect("legacy loads");
    let saved = serde_json::to_string_pretty(&loaded).unwrap();
    std::fs::write(&path, &saved).unwrap();
    let reloaded: ShellConfig =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        reloaded.last_checklist_version,
        loaded.last_checklist_version
    );
    temp_home::cleanup(&home);
}

const LEGACY_CONFIG: &str = r#"{
    "minimizeToTray": true,
    "autoStartEngine": true,
    "restartOnCrash": true,
    "telemetryDisabled": true,
    "npmRegistry": "https://registry.npmmirror.com",
    "defaultWorkspace": null,
    "uiTheme": "system",
    "firstRunCompleted": true,
    "lastChecklistVersion": "0.1.0",
    "setupSeenVersion": "0.1.0",
    "webuiPort": 3080,
    "engineHome": null,
    "runtimeMode": "bundled",
    "runtimeModeSelected": true
}"#;
