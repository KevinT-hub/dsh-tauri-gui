use crate::app::config::RuntimeMode;
use crate::app::AppState;
use crate::commands::shell::{set_shell_config, ShellConfigPatch};
use crate::ui::theme;
use std::sync::Arc;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

pub fn setup_tray(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let initial_mode = app
        .state::<Arc<AppState>>()
        .config
        .lock()
        .unwrap()
        .ui_theme
        .clone();
    let initial_runtime_is_bundled = matches!(
        app.state::<Arc<AppState>>()
            .config
            .lock()
            .unwrap()
            .runtime_mode,
        RuntimeMode::Bundled
    );
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
    let runtime_bundled = CheckMenuItem::with_id(
        app,
        "runtime-bundled",
        "内置运行时（推荐）",
        true,
        initial_runtime_is_bundled,
        None::<&str>,
    )?;
    let runtime_system = CheckMenuItem::with_id(
        app,
        "runtime-system",
        "系统运行时",
        true,
        !initial_runtime_is_bundled,
        None::<&str>,
    )?;
    let runtime_menu =
        Submenu::with_items(app, "运行时", true, &[&runtime_bundled, &runtime_system])?;
    let runtime_menu_handle = runtime_menu.clone();
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &open_web,
            &restart,
            &check_update,
            &theme_menu,
            &runtime_menu,
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
            "runtime-bundled" | "runtime-system" => {
                let want = if event.id().as_ref() == "runtime-bundled" {
                    RuntimeMode::Bundled
                } else {
                    RuntimeMode::System
                };
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                if state.config.lock().unwrap().runtime_mode == want {
                    return;
                }
                // The switch does a pre-flight check and then restarts the
                // whole app; run it off the menu event callback.
                let app = app.clone();
                std::thread::spawn(move || switch_runtime(app, state, want));
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
    *state.tray_runtime_menu.lock().unwrap() = Some(runtime_menu_handle);
    sync_theme_checks(&state);
    sync_runtime_checks(&state);
    *state.tray.lock().unwrap() = Some(tray);
    Ok(())
}

/// Switch the runtime mode from the tray. The whole flow is handled here in
/// Rust (no frontend dependency) so it works even when the webview is not
/// alive, and the order is important:
///
/// 1. Pre-flight: never restart into a runtime that cannot be resolved
///    (e.g. system node/npm/dsh missing from PATH). Abort with a log line
///    and keep the current mode if the target is unavailable.
/// 2. Persist the new mode — the restarted app reads it from disk.
/// 3. Reflect the new mode in the tray checkmarks immediately.
/// 4. Stop the engine that runs the OLD runtime. Without this, the
///    restarted app would detect the existing Web UI on port 3080, attach
///    to it and the switch would be a no-op.
/// 5. Relaunch the whole app through the normal exit path
///    (`ExitRequested` -> `stop_engine` no-op -> `Exit` -> Tauri spawns a
///    new instance), so the new bootstrap runs with the new runtime.
fn switch_runtime(app: AppHandle, state: Arc<AppState>, want: RuntimeMode) {
    // 1) Pre-flight.
    let available = match want {
        RuntimeMode::System => crate::engine::bootstrap::system_runtime_info().is_ok(),
        RuntimeMode::Bundled => {
            // Packaged builds always carry the archive; dev builds fall back
            // to an already-extracted local runtime.
            crate::engine::bootstrap::bundled_archive(&app).is_some()
                || crate::engine::bootstrap::runtime_info(&state.runtime_dir).is_ok()
        }
    };
    if !available {
        crate::app::emit_log(
            &state,
            Some(&app),
            "WARN",
            format!(
                "切换运行时被拒绝：目标运行时不可用（{}）。\
                 请确认已安装 node/npm/dsh（系统运行时）或内置运行时资源完整（内置运行时）后重试。",
                match want {
                    RuntimeMode::System => "系统运行时",
                    RuntimeMode::Bundled => "内置运行时",
                }
            ),
        );
        return;
    }
    // 2) Persist.
    if let Err(err) = set_shell_config(
        app.clone(),
        ShellConfigPatch {
            runtime_mode: Some(want),
            ..Default::default()
        },
    ) {
        state
            .logger
            .error(&format!("runtime switch: failed to persist mode: {err}"));
        return;
    }
    // 3) Reflect in the tray right away (also done inside set_shell_config,
    //    kept here so the intent of the tray path is explicit).
    sync_runtime_checks(&state);
    // 4) Stop the old engine.
    crate::engine::stop_engine(&state);
    // 5) Restart.
    app.request_restart();
}

pub(crate) fn sync_theme_checks(state: &Arc<AppState>) {
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

pub(crate) fn sync_runtime_checks(state: &Arc<AppState>) {
    let is_bundled = matches!(
        state.config.lock().unwrap().runtime_mode,
        RuntimeMode::Bundled
    );
    if let Some(menu) = state.tray_runtime_menu.lock().unwrap().as_ref() {
        for (id, value) in [
            ("runtime-bundled", is_bundled),
            ("runtime-system", !is_bundled),
        ] {
            if let Some(item) = menu.get(id) {
                if let Some(check) = item.as_check_menuitem() {
                    let _ = check.set_checked(value);
                }
            }
        }
    }
}
