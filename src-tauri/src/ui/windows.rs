use crate::app::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

const OVERLAY_MARGIN: i32 = 24;
const OVERLAY_LABEL: &str = "update-overlay";

pub fn reposition_update_overlay(app: &AppHandle) {
    let (Some(main), Some(overlay)) = (
        app.get_webview_window("main"),
        app.get_webview_window(OVERLAY_LABEL),
    ) else {
        return;
    };
    let Ok(main_pos) = main.outer_position() else {
        return;
    };
    let Ok(main_size) = main.outer_size() else {
        return;
    };
    let Ok(overlay_size) = overlay.outer_size() else {
        return;
    };
    let x = main_pos.x + main_size.width as i32 - overlay_size.width as i32 - OVERLAY_MARGIN;
    let y = main_pos.y + main_size.height as i32 - overlay_size.height as i32 - OVERLAY_MARGIN;
    let _ = overlay.set_position(tauri::PhysicalPosition::new(x.max(0), y.max(0)));
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn sync_update_overlay(app: &AppHandle) {
    let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let state = app.state::<Arc<AppState>>();
    let available = state
        .app_update
        .lock()
        .unwrap()
        .as_ref()
        .map(|info| info.available)
        .unwrap_or(false);
    let visible = match app.get_webview_window("main") {
        Some(main) => main.is_visible().unwrap_or(false) && !main.is_minimized().unwrap_or(false),
        None => false,
    };
    if available && visible {
        reposition_update_overlay(app);
        let _ = overlay.show();
    } else {
        let _ = overlay.hide();
    }
}

pub fn hide_update_overlay(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = overlay.hide();
    }
}
