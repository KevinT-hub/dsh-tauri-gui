mod app;
mod commands;
mod core;
mod detection;
mod engine;
mod geo;
mod ui;
mod update;

use app::AppState;
use core::logging::Logger;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app::lifecycle::show_main(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_page_load(|webview, _payload| {
            if webview.label() != "main" {
                return;
            }
            let window = webview.window();
            let app = window.app_handle();
            let url = webview
                .url()
                .map(|value| value.to_string())
                .unwrap_or_default();
            let Some(state) = app.try_state::<Arc<AppState>>() else {
                // First load can fire before setup has managed the state; the
                // frontend calls `shell_ready` once it has painted.
                return;
            };
            if engine::is_shell_url(&url) {
                // The shell page is the loading surface for both the
                // checklist and normal startup. Do not wait for the dsh Web
                // UI to become ready before revealing it.
                ui::windows::show_main_window(app);
                return;
            }
            if engine::is_allowed_web_url(&state, &url) {
                // The official Web UI has finished loading: only now reveal
                // the window so there is no black/blank flash.
                ui::windows::show_main_window(app);
            }
        })
        .setup(|app| {
            let home = core::paths::resolve_shell_home();
            let logs_dir = home.join("logs");
            std::fs::create_dir_all(&logs_dir)?;
            let logger = Logger::new(logs_dir.clone());
            let (config, config_path) = app::config::load(&home);
            let shell_url = app
                .get_webview_window("main")
                .and_then(|window| window.url().ok())
                .map(|url| url.to_string())
                .unwrap_or_else(|| "http://tauri.localhost/".to_string());
            let engine_home = config
                .engine_home
                .as_deref()
                .map(std::path::PathBuf::from)
                .map(core::paths::absolutize)
                .unwrap_or_else(core::paths::resolve_engine_home);
            let state = Arc::new(AppState::new(
                home,
                engine_home,
                logs_dir.clone(),
                logger,
                config,
                config_path,
                shell_url,
            ));
            app.manage(state.clone());
            state.logger.info(&format!(
                "desktop shell starting, home: {}",
                state.home.display()
            ));

            app::bootstrap::start_cached_engine(app.handle(), state.clone());

            ui::tray::setup_tray(app).map_err(|err| {
                state.logger.error(&format!("tray setup failed: {err}"));
                err
            })?;
            ui::theme::spawn_theme_watcher(app.handle().clone(), state.clone());
            ui::theme::sync_window_theme(app.handle(), &state);

            // Detection is deferred until the frontend has painted the shell
            // page (see the `shell_ready` / `begin_setup` commands), so the
            // version-gated setup screen can show before probing starts.
            schedule_app_update_check(app.handle().clone(), state);
            schedule_dsh_update_check(
                app.handle().clone(),
                app.state::<Arc<AppState>>().inner().clone(),
            );
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let app = window.app_handle();
                    let state = app.state::<Arc<AppState>>();
                    if state.config.lock().unwrap().minimize_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                        ui::windows::sync_update_overlay(app);
                    }
                }
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                    let app = window.app_handle();
                    ui::windows::sync_update_overlay(app);
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::shell::shell_status,
            commands::shell::get_shell_config,
            commands::shell::set_shell_config,
            commands::shell::get_diagnostics,
            commands::shell::checklist_state,
            commands::shell::shell_ready,
            commands::shell::restart_engine,
            commands::shell::open_logs_dir,
            commands::shell::open_web_ui,
            commands::shell::quit_app,
            commands::shell::get_theme_state,
            commands::shell::set_ui_theme,
            commands::shell::hide_update_overlay,
            commands::setup_flow::run_detection_v2,
            commands::setup::mark_setup_seen,
            commands::setup_flow::install_dependency_v2,
            commands::setup::enter_harness,
            commands::setup_flow::recheck_environment_v2,
            commands::setup_flow::begin_setup_v2,
            commands::dsh_update::get_dsh_update,
            commands::dsh_update::install_dsh_update,
            commands::geo::get_geo_state,
            commands::updater::check_app_update,
            commands::updater::apply_app_update,
            commands::updater::get_app_update,
            commands::updater::get_update_notice,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app_handle.state::<Arc<AppState>>();
                app::lifecycle::shutdown(&state);
            }
        });
}

/// Background app-update probe (GitHub first, mirrors fallback). The overlay
/// button only appears when an update is actually available.
fn schedule_app_update_check(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        let _ = ui::tray::check_app_update_now(app, state);
    });
}

fn schedule_dsh_update_check(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(4));
        let _ = ui::tray::check_dsh_update_now(app, state);
    });
}
