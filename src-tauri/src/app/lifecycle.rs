//! Application lifecycle helpers: single-instance focus, engine shutdown on
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

/// Shut the engine down cleanly before the process exits.
pub fn shutdown(state: &Arc<AppState>) {
    crate::engine::stop_engine(state);
}
