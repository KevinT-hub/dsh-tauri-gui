//! Shell status model, phase transitions and event publishing.

use crate::app::state::AppState;
use serde::Serialize;
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
