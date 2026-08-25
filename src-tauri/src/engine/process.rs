//! Engine child-process plumbing and exit monitoring.
//!
//! Engine output is redirected to a file instead of being tied to the shell's
//! lifetime. That lets a warm `dsh web` process survive a desktop-shell
//! restart without a blocked stdout pipe.

use crate::app::state::AppState;
use crate::core::process;
use std::fs::OpenOptions;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

/// Spawn `cmd` with file-backed stdout/stderr, null stdin and hidden console.
/// The caller owns the child handle via `AppState::engine`.
pub fn spawn_with_logs(
    app: &AppHandle,
    state: &Arc<AppState>,
    cmd: &mut Command,
) -> Result<Child, String> {
    let engine_log = state.logs_dir.join("engine.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&engine_log)
        .map_err(|err| format!("cannot open engine log: {err}"))?;
    let stderr = stdout
        .try_clone()
        .map_err(|err| format!("cannot clone engine log handle: {err}"))?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    process::hide_console(cmd);

    let mut child = cmd
        .spawn()
        .map_err(|err| format!("cannot start engine process: {err}"))?;
    // Close the stop/spawn race: `stop_engine` sets `stopping` before it
    // takes the child, so a process spawned in that window must be killed
    // here instead of being orphaned.
    if state.stopping.load(Ordering::SeqCst) {
        process::kill_process_tree(&mut child);
        return Err("engine start was cancelled".to_string());
    }

    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        format!("[engine] process started: pid={}", child.id()),
    );
    Ok(child)
}

/// Start monitoring only after the caller has stored the child in AppState.
/// Starting this thread inside `spawn_with_logs` races with that assignment
/// and can make the monitor exit immediately after observing `None`.
pub fn start_monitor(app: AppHandle, state: Arc<AppState>) {
    thread::spawn(move || monitor_engine(app, state));
}

/// Watch the engine child for exit and restart it when configured to do so.
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
                *state.engine_session.lock().unwrap() = None;
                crate::app::engine_session::clear(&state.home);
                state
                    .logger
                    .warn(&format!("engine exited with status {code:?}"));
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
                    "engineStopped",
                    Some(format!("process_exit_code={code:?}")),
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
                        "engineRestartingAfterCrash",
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
                            "engineRestartFailed",
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
