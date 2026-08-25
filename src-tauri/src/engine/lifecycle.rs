//! Engine lifecycle orchestration: connect-or-spawn, restart, stop and
//! crash recovery. All process mechanics live in `process.rs`; this module
//! only decides *when* things start/stop.

use crate::app::state::AppState;
use crate::core::process;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

/// Start the engine unless one is already starting. Returns `Ok` silently
/// when a spawn is already in flight.
pub fn spawn_engine(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    if state.engine.lock().unwrap().is_some() {
        return Ok(());
    }
    if state
        .engine_starting
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }
    let result = spawn_engine_inner(app, state);
    state.engine_starting.store(false, Ordering::SeqCst);
    result
}

fn effective_port(state: &AppState) -> u16 {
    if state.webui_port_fallback.load(Ordering::SeqCst) {
        return state
            .engine_session
            .lock()
            .unwrap()
            .as_ref()
            .map(|session| session.port)
            .unwrap_or(0);
    } else {
        state.config.lock().unwrap().webui_port
    }
}

fn spawn_engine_inner(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let spec = state
        .command_spec
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "环境检测未通过，无法启动引擎".to_string())?;
    let config = state.config.lock().unwrap().clone();
    let requested_port = effective_port(state);
    let port = crate::engine::web::select_port(requested_port)?;
    if requested_port != 0 && requested_port != port {
        state.webui_port_fallback.store(true, Ordering::SeqCst);
    }
    let workspace = crate::engine::workspace_dir(&config);

    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        format!(
            "[check] 启动引擎: dsh={} port={} DSH_HOME={} cwd={}",
            spec.dsh_bin.display(),
            if port == 0 {
                "系统分配".to_string()
            } else {
                port.to_string()
            },
            state.engine_home.display(),
            workspace.display()
        ),
    );

    let mut cmd = crate::engine::command::dsh_command(&spec);
    crate::engine::command::append_web_command_args(&mut cmd, port);
    cmd.current_dir(workspace);
    for (key, value) in crate::engine::environment::engine_env(&config, &state.engine_home) {
        cmd.env(key, value);
    }
    cmd.env("PATH", crate::engine::environment::build_path(&spec));

    state.ready_url.lock().unwrap().take();
    let child = crate::engine::process::spawn_with_logs(app, state, &mut cmd)?;
    let session = crate::app::engine_session::EngineSession {
        pid: child.id(),
        port,
    };
    if let Err(err) = crate::app::engine_session::save(&state.home, &session) {
        state
            .logger
            .warn(&format!("failed to save engine session: {err}"));
    }
    *state.engine_session.lock().unwrap() = Some(session);
    *state.engine.lock().unwrap() = Some(child);
    state.stopping.store(false, Ordering::SeqCst);
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    crate::engine::process::start_monitor(app.clone(), state.clone());
    start_ready_probe(app.clone(), state.clone(), port, generation);
    Ok(())
}

/// Resolve readiness through a short local health probe instead of parsing
/// child stdout. This keeps engine lifetime independent from the shell and
/// also works when the process was adopted from a previous launch.
fn start_ready_probe(app: AppHandle, state: Arc<AppState>, port: u16, generation: u64) {
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if state.stopping.load(Ordering::SeqCst)
                || state.generation.load(Ordering::SeqCst) != generation
                || state.engine.lock().unwrap().is_none()
            {
                return;
            }
            if crate::engine::web::is_dsh_web_alive(port) {
                let url = format!("http://127.0.0.1:{port}/");
                let checklist = crate::app::config::checklist_required_full(&state);
                let version = state
                    .command_spec
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|spec| spec.dsh_version.clone());
                *state.ready_url.lock().unwrap() = Some(url.clone());
                state.logger.info(&format!("web UI ready at {url}"));
                crate::app::set_status(
                    &state,
                    Some(&app),
                    "engine-ready",
                    "engineReady",
                    None,
                    Some(url.clone()),
                    None,
                    version,
                );
                crate::ui::windows::sync_update_overlay(&app);
                if !checklist {
                    crate::engine::web::navigate(&app, &url);
                }
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if state.generation.load(Ordering::SeqCst) == generation
            && !state.stopping.load(Ordering::SeqCst)
        {
            state.logger.error("engine readiness probe timed out");
            crate::app::set_status(
                &state,
                Some(&app),
                "error",
                "engineStartFailed",
                None,
                None,
                None,
                None,
            );
        }
    });
}

/// Connect to an already-running official `dsh web` on the configured port,
/// or spawn the validated engine. Returns `true` when an existing instance
/// was reused.
pub fn connect_existing_or_spawn(app: &AppHandle, state: &Arc<AppState>) -> Result<bool, String> {
    // A cached/direct startup or another caller may already be spawning dsh.
    // Return immediately instead of performing the expensive HTTP probe.
    if state.engine.lock().unwrap().is_some() || state.engine_starting.load(Ordering::SeqCst) {
        return Ok(false);
    }
    let port = effective_port(state);
    let port_label = if port == 0 {
        "随机(系统分配)".to_string()
    } else {
        port.to_string()
    };
    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        format!("[check] 端口与实例检测: 探测 127.0.0.1:{port_label}"),
    );
    if port != 0 && crate::engine::web::is_dsh_web_alive(port) {
        let url = format!("http://127.0.0.1:{port}");
        let checklist = crate::app::config::checklist_required_full(state);
        state
            .logger
            .info(&format!("connecting to existing dsh web at {url}"));
        crate::app::emit_log(
            state,
            Some(app),
            "INFO",
            format!("[check] 发现已有 WebUI 实例，直接连接: {url}"),
        );
        let version = state
            .command_spec
            .lock()
            .unwrap()
            .as_ref()
            .map(|spec| spec.dsh_version.clone());
        *state.ready_url.lock().unwrap() = Some(url.clone());
        crate::app::set_status(
            state,
            Some(app),
            "engine-ready",
            "engineReadyExisting",
            None,
            Some(url.clone()),
            None,
            version,
        );
        crate::ui::windows::sync_update_overlay(app);
        if !checklist {
            crate::engine::web::navigate(app, &url);
            // on_page_load reveals the window once the Web UI has painted.
        }
        return Ok(true);
    }
    // No healthy instance is available. Clear stale handoff state before the
    // new process chooses a port.
    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        "[check] 未发现已有实例，启动引擎".to_string(),
    );
    crate::app::engine_session::clear(&state.home);
    *state.engine_session.lock().unwrap() = None;
    spawn_engine(app, state)?;
    Ok(false)
}

/// Navigate to an engine that may already be running or still starting.
///
/// The first-run checklist can finish after the warm engine has become ready.
/// In that case the readiness probe intentionally did not navigate while the
/// checklist was visible, so entering the harness must explicitly hand off
/// the WebView once the same engine is ready.
pub fn navigate_when_ready(app: &AppHandle, state: &Arc<AppState>) {
    let app = app.clone();
    let state = state.clone();
    std::thread::spawn(move || {
        // A process can exit between the enter action and the crash monitor's
        // restart. Keep the handoff alive through that short gap instead of
        // abandoning navigation as soon as the Child handle disappears.
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if state.stopping.load(Ordering::SeqCst) {
                return;
            }
            if let Some(url) = live_ready_url(&state) {
                state.logger.info(&format!("navigating WebView to {url}"));
                crate::engine::web::navigate(&app, &url);
                return;
            }
            let running = state.engine.lock().unwrap().is_some()
                || state.engine_starting.load(Ordering::SeqCst);
            if !running {
                // With crash recovery enabled, monitor_engine owns the
                // restart. It briefly clears `engine` before spawning the new
                // process, so continue waiting for that replacement.
                if !state.config.lock().unwrap().restart_on_crash {
                    crate::app::set_status(
                        &state,
                        Some(&app),
                        "error",
                        "engineStartFailed",
                        Some("engine stopped before the Web UI became ready".to_string()),
                        None,
                        None,
                        None,
                    );
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !state.stopping.load(Ordering::SeqCst) {
            state.logger.error("engine navigation wait timed out");
            crate::app::set_status(
                &state,
                Some(&app),
                "error",
                "engineStartFailed",
                None,
                None,
                None,
                None,
            );
        }
    });
}

/// Stop the engine process owned by this shell instance.
///
/// A detached engine adopted from a previous shell has no child handle here;
/// it is intentionally left running so the next shell launch can reuse it.
pub fn stop_engine(state: &Arc<AppState>) {
    state.stopping.store(true, Ordering::SeqCst);
    state.ready_url.lock().unwrap().take();
    let child = state.engine.lock().unwrap().take();
    let session = state.engine_session.lock().unwrap().take();
    crate::app::engine_session::clear(&state.home);
    if let Some(mut child) = child {
        let pid = child.id();
        state.logger.info(&format!("stopping engine (pid {pid})"));
        process::kill_process_tree(&mut child);
    } else if let Some(session) = session {
        if crate::engine::web::is_dsh_web_alive(session.port) {
            state
                .logger
                .info(&format!("stopping detached engine (pid {})", session.pid));
            process::kill_process_id(session.pid);
        } else {
            state
                .logger
                .warn("detached engine session is stale; refusing to terminate its pid");
        }
    }
}

/// Release the shell's child handle without terminating the engine. Engine
/// stdout/stderr are file-backed, so the detached process can safely remain
/// available for the next desktop-shell launch.
pub fn detach_engine(state: &Arc<AppState>) {
    state.stopping.store(true, Ordering::SeqCst);
    state.ready_url.lock().unwrap().take();
    if let Some(child) = state.engine.lock().unwrap().take() {
        let session = crate::app::engine_session::EngineSession {
            pid: child.id(),
            port: state
                .engine_session
                .lock()
                .unwrap()
                .as_ref()
                .map(|session| session.port)
                .unwrap_or_else(|| state.config.lock().unwrap().webui_port),
        };
        let _ = crate::app::engine_session::save(&state.home, &session);
        *state.engine_session.lock().unwrap() = Some(session);
        state
            .logger
            .info(&format!("detaching warm engine (pid {})", child.id()));
        drop(child);
    }
}

pub fn restart_engine(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    state.logger.info("restarting engine");
    stop_engine(state);
    state.stopping.store(false, Ordering::SeqCst);
    let version = state
        .command_spec
        .lock()
        .unwrap()
        .as_ref()
        .map(|spec| spec.dsh_version.clone());
    crate::app::set_status(
        state,
        Some(app),
        "engine-starting",
        "engineRestarting",
        None,
        None,
        None,
        version,
    );
    match connect_existing_or_spawn(app, state) {
        Ok(_) => Ok(()),
        Err(err) => {
            crate::app::set_status(
                state,
                Some(app),
                "error",
                "engineRestartFailed",
                Some(err.clone()),
                None,
                None,
                None,
            );
            Err(err)
        }
    }
}

/// Navigate the main WebView to the running Web UI.
pub fn open_web_ui(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let url = ensure_web_ui_ready(app, state)?;
    crate::engine::web::navigate(app, &url);
    // The window is revealed by on_page_load after the Web UI has painted,
    // so there is no black flash during the transition.
    Ok(())
}

/// Open the running Web UI in the system default browser.
pub fn open_web_ui_browser(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let url = ensure_web_ui_ready(app, state)?;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|err| format!("无法打开系统浏览器: {err}"))
}

/// Treat `ready_url` as a cache and verify that the server is still alive
/// before handing its address to a browser or WebView. A stale address is
/// common after a crash or a manual engine shutdown.
fn ensure_web_ui_ready(app: &AppHandle, state: &Arc<AppState>) -> Result<String, String> {
    if let Some(url) = live_ready_url(state) {
        return Ok(url);
    }

    state.logger.info("Web UI 地址已失效，尝试重新启动引擎");
    if state.command_spec.lock().unwrap().is_none() {
        return Err("环境检测未通过，无法启动 Web UI".to_string());
    }

    restart_engine(app, state)?;
    wait_for_live_ready_url(state, Duration::from_secs(20))
        .ok_or_else(|| "Web UI 启动超时，请稍后重试或查看日志".to_string())
}

fn live_ready_url(state: &Arc<AppState>) -> Option<String> {
    let url = state.ready_url.lock().unwrap().clone()?;
    let parsed = url::Url::parse(&url).ok()?;
    let port = parsed.port()?;
    if crate::engine::web::is_dsh_web_alive(port) {
        Some(url)
    } else {
        state.ready_url.lock().unwrap().take();
        None
    }
}

fn wait_for_live_ready_url(state: &Arc<AppState>, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(url) = live_ready_url(state) {
            return Some(url);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
}
