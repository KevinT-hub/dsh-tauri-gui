//! Shell status model, phase transitions and event publishing.

use crate::app::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellStatus {
    pub phase: &'static str,
    pub code: &'static str,
    pub detail: Option<String>,
    pub url: Option<String>,
    pub progress: Option<f64>,
    pub engine_version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotice {
    pub target: &'static str,
    pub phase: &'static str,
    pub version: Option<String>,
    pub error: Option<String>,
}

pub fn set_update_notice(
    state: &AppState,
    app: &AppHandle,
    target: &'static str,
    phase: &'static str,
    version: Option<String>,
    error: Option<String>,
) {
    let notice = UpdateNotice {
        target,
        phase,
        version,
        error,
    };
    *state.update_notice.lock().unwrap() = Some(notice.clone());
    let _ = app.emit(crate::app::events::UPDATE_NOTICE_EVENT, notice);
    crate::ui::windows::sync_update_overlay(app);
}

pub fn clear_update_notice(state: &AppState, app: &AppHandle) {
    state.update_notice.lock().unwrap().take();
    crate::ui::windows::sync_update_overlay(app);
}

#[allow(clippy::too_many_arguments)]
pub fn set_status(
    state: &AppState,
    app: Option<&AppHandle>,
    phase: &'static str,
    code: &'static str,
    detail: Option<String>,
    url: Option<String>,
    progress: Option<f64>,
    engine_version: Option<String>,
) {
    let status = ShellStatus {
        phase,
        code,
        detail,
        url,
        progress,
        engine_version,
    };
    *state.status.lock().unwrap() = status.clone();
    if let Some(app) = app {
        let _ = app.emit(crate::app::events::STATUS_EVENT, status);
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
            crate::app::events::LOG_EVENT,
            serde_json::json!({
                "level": level.to_lowercase(),
                "line": line,
            }),
        );
    }
}
