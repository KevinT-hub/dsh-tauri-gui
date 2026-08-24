//! Engine lifecycle orchestration: connect-or-spawn, restart, stop and
//! crash recovery. All process mechanics live in `process.rs`; this module
//! only decides *when* things start/stop.

use crate::app::state::AppState;
use crate::core::process;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

/// Start the engine unless one is already starting. Returns `Ok` silently
/// when a spawn is already in flight.
pub fn spawn_engine(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
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
        0
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
    let port = effective_port(state);
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

    let child = crate::engine::process::spawn_with_pipes(app, state, &mut cmd)?;
    *state.engine.lock().unwrap() = Some(child);
    state.stopping.store(false, Ordering::SeqCst);
    state.ready_url.lock().unwrap().take();
    state.generation.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

/// Connect to an already-running official `dsh web` on the configured port,
/// or spawn the validated engine. Returns `true` when an existing instance
/// was reused.
pub fn connect_existing_or_spawn(app: &AppHandle, state: &Arc<AppState>) -> Result<bool, String> {
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
            "已连接到现有 Web UI",
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
    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        "[check] 未发现已有实例，启动引擎".to_string(),
    );
    spawn_engine(app, state)?;
    Ok(false)
}

/// Stop the engine process (and its whole tree on Windows).
pub fn stop_engine(state: &Arc<AppState>) {
    state.stopping.store(true, Ordering::SeqCst);
    let child = state.engine.lock().unwrap().take();
    if let Some(mut child) = child {
        let pid = child.id();
        state.logger.info(&format!("stopping engine (pid {pid})"));
        process::kill_process_tree(&mut child);
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
        "正在重启引擎…",
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
                "引擎重启失败",
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
    let url = state
        .ready_url
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Web UI 尚未就绪".to_string())?;
    crate::engine::web::navigate(app, &url);
    // The window is revealed by on_page_load after the Web UI has painted,
    // so there is no black flash during the transition.
    Ok(())
}

/// Open the running Web UI in the system default browser.
pub fn open_web_ui_browser(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let url = state
        .ready_url
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Web UI 尚未就绪".to_string())?;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|err| format!("无法打开系统浏览器: {err}"))
}
