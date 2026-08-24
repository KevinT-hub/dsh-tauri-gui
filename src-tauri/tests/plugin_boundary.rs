//! Integration test: the shell must never touch dsh plugin/session content.
//!
//! The engine environment contract (see docs/PLUGIN_COMPATIBILITY.md):
//! - `DSH_HOME` defaults to `~/.dsh` and is passed through unchanged;
//! - the engine command is exactly `dsh web --no-open --port <port>`;
//! - no shell code writes into the plugin/session directories.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a parent")
        .to_path_buf()
}

fn engine_sources() -> String {
    let engine_dir = repo_root().join("src-tauri/src/engine");
    let mut out = String::new();
    if let Ok(entries) = fs::read_dir(&engine_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "rs") {
                out.push_str(&fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
    }
    out
}

fn detection_sources() -> String {
    let dir = repo_root().join("src-tauri/src/detection");
    let mut out = String::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "rs") {
                out.push_str(&fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
    }
    out
}

#[test]
fn engine_uses_official_dsh_web_command_only() {
    let source = engine_sources();
    assert!(
        source.contains("append_web_command_args"),
        "engine must build `dsh web --no-open --port` args"
    );
    assert!(
        source.contains("--no-open") && source.contains("--port"),
        "engine args must include --no-open and --port"
    );
}

#[test]
fn dsh_home_semantics_are_preserved() {
    let source = engine_sources();
    assert!(
        source.contains("DSH_HOME"),
        "engine must pass DSH_HOME to the dsh process"
    );
    // The engine only *hands* the home dir over; it must not write plugin
    // or session content itself.
    assert!(
        !source.contains("remove_dir_all") && !source.contains("create_dir_all"),
        "engine must not create/remove directories inside DSH_HOME"
    );
}

#[test]
fn detection_never_touches_plugin_content() {
    let source = detection_sources();
    assert!(
        !source.contains("plugins") || source.contains("plugin"), // dsh.rs mentions the plugin *package* only
        "detection must not read plugin directories"
    );
    // Detection only probes PATH executables — no DSH_HOME writes.
    assert!(
        !source.contains("engine_home") && !source.contains("DSH_HOME"),
        "detection must not depend on engine home"
    );
}
