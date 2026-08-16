use crate::app::AppState;
use crate::ui::theme;
use std::sync::Arc;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

pub fn setup_tray(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let initial_mode = app
        .state::<Arc<AppState>>()
        .config
        .lock()
        .unwrap()
        .ui_theme
        .clone();
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let open_web = MenuItem::with_id(app, "open-web", "打开 Web UI", true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart", "重启引擎", true, None::<&str>)?;
    let check_update = MenuItem::with_id(app, "check-update", "检查更新", true, None::<&str>)?;
    let theme_light = CheckMenuItem::with_id(
        app,
        "theme-light",
        "亮色",
        true,
        initial_mode == "light",
        None::<&str>,
    )?;
    let theme_dark = CheckMenuItem::with_id(
        app,
        "theme-dark",
        "暗色",
        true,
        initial_mode == "dark",
        None::<&str>,
    )?;
    let theme_system = CheckMenuItem::with_id(
        app,
        "theme-system",
        "跟随系统",
        true,
        initial_mode == "system",
        None::<&str>,
    )?;
    let theme_menu = Submenu::with_items(
        app,
        "外观",
        true,
        &[&theme_light, &theme_dark, &theme_system],
    )?;
    let theme_menu_handle = theme_menu.clone();
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &open_web,
            &restart,
            &check_update,
            &theme_menu,
            &quit,
        ],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("missing default window icon")?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
                crate::ui::windows::sync_update_overlay(app);
            }
            "open-web" => {
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                let _ = crate::engine::open_web_ui_browser(app, &state);
            }
            "restart" => {
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                let app = app.clone();
                std::thread::spawn(move || {
                    let _ = crate::engine::restart_engine(&app, &state);
                });
            }
            "check-update" => {
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                let app = app.clone();
                std::thread::spawn(move || {
                    crate::engine::runtime_update::check_and_notify(&app, &state);
                });
            }
            "theme-light" | "theme-dark" | "theme-system" => {
                let mode = event.id().as_ref().trim_start_matches("theme-").to_string();
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                if let Err(err) = theme::set_ui_theme(app, &state, mode) {
                    state.logger.error(&err);
                }
                sync_theme_checks(&state);
            }
            "quit" => {
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                crate::engine::stop_engine(&state);
                app.cleanup_before_exit();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
                crate::ui::windows::sync_update_overlay(app);
            }
        })
        .build(app)?;

    let state = app.state::<Arc<AppState>>();
    *state.tray_menu.lock().unwrap() = Some(menu);
    *state.tray_theme_menu.lock().unwrap() = Some(theme_menu_handle);
    sync_theme_checks(&state);
    *state.tray.lock().unwrap() = Some(tray);
    Ok(())
}

fn sync_theme_checks(state: &Arc<AppState>) {
    let mode = state.config.lock().unwrap().ui_theme.clone();
    for (id, value) in [
        ("theme-light", mode == "light"),
        ("theme-dark", mode == "dark"),
        ("theme-system", mode == "system"),
    ] {
        if let Some(menu) = state.tray_theme_menu.lock().unwrap().as_ref() {
            if let Some(item) = menu.get(id) {
                if let Some(check) = item.as_check_menuitem() {
                    let _ = check.set_checked(value);
                }
            }
        }
    }
}
