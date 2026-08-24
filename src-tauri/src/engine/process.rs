//! Engine child-process plumbing: spawn with piped streams, line pumping and
//! exit monitoring. Lifecycle orchestration lives in `lifecycle.rs`.

use crate::app::state::AppState;
use crate::core::process;
use crate::core::redact::redact;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Spawn `cmd` with piped stdout/stderr, null stdin and hidden console, then
/// start the line pumpers and the exit monitor. The caller owns the child
/// handle via `AppState::engine`.
pub fn spawn_with_pipes(
    app: &AppHandle,
    state: &Arc<AppState>,
    cmd: &mut Command,
) -> Result<Child, String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    process::hide_console(cmd);

    let mut child = cmd
        .spawn()
        .map_err(|err| format!("无法启动引擎进程: {err}"))?;
    // Close the stop/spawn race: `stop_engine` sets `stopping` before it
    // takes the child, so a process spawned in that window must be killed
    // here instead of being orphaned (an orphaned engine would keep the
    // port alive and defeat a restart on the next launch).
    if state.stopping.load(Ordering::SeqCst) {
        process::kill_process_tree(&mut child);
        return Err("引擎启动被中断（正在停止）".to_string());
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout 管道不可用".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "stderr 管道不可用".to_string())?;

    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        format!("[check] 引擎进程已启动 pid={}", child.id()),
    );

    thread::spawn({
        let app = app.clone();
        let state = state.clone();
        move || pump_lines(app, state, stdout, false)
    });
    thread::spawn({
        let app = app.clone();
        let state = state.clone();
        move || pump_lines(app, state, stderr, true)
    });
    thread::spawn({
        let app = app.clone();
        let state = state.clone();
        move || monitor_engine(app, state)
    });
    Ok(child)
}

/// Drain one stdout/stderr stream line-by-line into the log file, the live
/// log event and the ready-marker parser.
fn pump_lines(
    app: AppHandle,
    state: Arc<AppState>,
    stream: impl Read + Send + 'static,
    is_stderr: bool,
) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        let line = redact(&line);
        let level = if is_stderr { "WARN" } else { "INFO" };
        state.logger.log("engine", level, &line);
        crate::app::push_log_tail(&state, line.clone());
        let _ = app.emit(
            crate::app::events::LOG_EVENT,
            serde_json::json!({
                "level": if is_stderr { "warn" } else { "info" },
                "line": line,
            }),
        );
        if !is_stderr {
            if let Some(url) = crate::engine::protocol::parse_web_url(&line) {
                let checklist = crate::app::config::checklist_required_full(&state);
                state.logger.info(&format!("web UI ready at {url}"));
                let version = state
                    .command_spec
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|spec| spec.dsh_version.clone());
                *state.ready_url.lock().unwrap() = Some(url.clone());
                crate::app::set_status(
                    &state,
                    Some(&app),
                    "engine-ready",
                    "Web UI 已就绪",
                    None,
                    Some(url.clone()),
                    None,
                    version,
                );
                crate::ui::windows::sync_update_overlay(&app);
                if !checklist {
                    crate::engine::web::navigate(&app, &url);
                }
            }
        }
    }
}

/// Watch the engine child for exit; on abnormal exit, retry once on an
/// OS-assigned port when the configured port was taken, then restart if
/// `restartOnCrash` is enabled.
fn monitor_engine(app: AppHandle, state: Arc<AppState>) {
    loop {
        let exited = {
            let mut guard = state.engine.lock().unwrap();
            match guard.as_mut() {
                Some(child) => child.try_wait().ok().flatten(),
                None => {
                    *guard = None;
                    return;
                }
            }
        };
        match exited {
            Some(status) => {
                let code = status.code();
                *state.engine.lock().unwrap() = None;
                state
                    .logger
                    .warn(&format!("engine exited with status {code:?}"));
                if !state.webui_port_fallback.load(Ordering::SeqCst) {
                    let bind_failed = state.log_tail.lock().unwrap().iter().any(|line| {
                        line.contains("EADDRINUSE")
                            || line.contains("address already in use")
                            || line.contains("EACCES")
                    });
                    if bind_failed {
                        state.webui_port_fallback.store(true, Ordering::SeqCst);
                        state
                            .logger
                            .warn("configured port in use; switching to an OS-assigned port");
                    }
                }
                if state.stopping.load(Ordering::SeqCst) {
                    return;
                }
                let restart = state.config.lock().unwrap().restart_on_crash;
                let version = state
                    .command_spec
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|spec| spec.dsh_version.clone());
                crate::app::set_status(
                    &state,
                    Some(&app),
                    "engine-stopped",
                    format!("引擎已退出（退出码 {code:?}）"),
                    None,
                    None,
                    None,
                    version,
                );
                if restart {
                    thread::sleep(Duration::from_secs(2));
                    if state.stopping.load(Ordering::SeqCst) {
                        return;
                    }
                    crate::app::set_status(
                        &state,
                        Some(&app),
                        "engine-starting",
                        "引擎异常退出，正在自动重启…",
                        None,
                        None,
                        None,
                        state
                            .command_spec
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|spec| spec.dsh_version.clone()),
                    );
                    if let Err(err) =
                        crate::engine::lifecycle::connect_existing_or_spawn(&app, &state)
                    {
                        crate::app::set_status(
                            &state,
                            Some(&app),
                            "error",
                            "引擎重启失败",
                            Some(err),
                            None,
                            None,
                            None,
                        );
                    }
                }
                return;
            }
            None => thread::sleep(Duration::from_millis(200)),
        }
    }
}
