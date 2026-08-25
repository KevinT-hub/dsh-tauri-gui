//! Application lifecycle helpers: single-instance focus, engine handoff on
//! exit and the window-show policy shared by lib.rs and the tray.

use crate::app::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Bring the main window to the foreground (used by single-instance events,
/// tray clicks and the setup screen reveal).
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    crate::ui::windows::sync_update_overlay(app);
}

/// Release the desktop shell while keeping the warm engine available for the
/// next launch. Explicit restart/recheck/update actions still stop it.
pub fn shutdown(state: &Arc<AppState>) {
    crate::engine::detach_engine(state);
}
