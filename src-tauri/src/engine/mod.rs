pub mod bootstrap;
pub mod runtime_update;

use crate::app::config::ShellConfig;
use crate::app::{self, AppState};
use crate::core::redact::redact;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

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

fn workspace_dir(config: &ShellConfig) -> PathBuf {
    config
        .default_workspace
        .as_ref()
        .filter(|path| path.is_dir())
        .cloned()
        .or_else(|| Some(crate::core::paths::user_home_dir()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn build_path(runtime: &crate::app::RuntimeInfo, state: &AppState) -> String {
    let node_dir = runtime
        .node_exe
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| state.runtime_dir.join("node"));
    let app_bin = state.runtime_dir.join("app/node_modules/.bin");
    let tools_bin = state.runtime_dir.join("tools/node_modules/.bin");
    let mut parts = vec![
        node_dir.display().to_string(),
        app_bin.display().to_string(),
        tools_bin.display().to_string(),
    ];
    if let Some(existing) = std::env::var_os("PATH") {
        parts.push(existing.to_string_lossy().to_string());
    }
    let separator = if cfg!(windows) { ";" } else { ":" };
    parts.join(separator)
}

fn effective_port(state: &AppState) -> u16 {
    if state.webui_port_fallback.load(Ordering::SeqCst) {
        0
    } else {
        state.config.lock().unwrap().webui_port
    }
}

/// Probe whether an official `dsh web` instance is already serving the
/// configured port. If it is, the desktop app connects to it instead of
/// spawning a second engine, so config/sessions stay in one service area.
fn is_dsh_web_alive(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(800)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1200)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));
    let request = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut head = String::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                head.push_str(&String::from_utf8_lossy(&buf[..n]));
                if head.contains("__DSH_BOOT__") {
                    return true;
                }
            }
        }
    }
    head.starts_with("HTTP/") && head.contains("__DSH_BOOT__")
}

fn spawn_engine_inner(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let runtime = state
        .runtime
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "运行时尚未就绪".to_string())?;
    let config = state.config.lock().unwrap().clone();
    let port = effective_port(state);
    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        format!(
            "[check] 启动引擎: node={} dsh={} port={} DSH_HOME={} cwd={}",
            runtime.node_exe.display(),
            runtime.dsh_bin.display(),
            if port == 0 {
                "系统分配".to_string()
            } else {
                port.to_string()
            },
            state.engine_home.display(),
            workspace_dir(&config).display()
        ),
    );

    let mut cmd = Command::new(&runtime.node_exe);
    cmd.arg(&runtime.dsh_bin)
        .arg("web")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(workspace_dir(&config))
        .env("DSH_HOME", &state.engine_home)
        .env(
            "DSH_TELEMETRY_DISABLED",
            if config.telemetry_disabled { "1" } else { "0" },
        )
        .env("npm_config_registry", &config.npm_registry)
        .env("NO_COLOR", "1")
        .env("PATH", build_path(&runtime, state))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|err| format!("无法启动引擎进程: {err}"))?;
    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        format!("[check] 引擎进程已启动 pid={}", child.id()),
    );
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout 管道不可用".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "stderr 管道不可用".to_string())?;

    *state.engine.lock().unwrap() = Some(child);
    state.stopping.store(false, Ordering::SeqCst);
    state.ready_url.lock().unwrap().take();
    state.generation.fetch_add(1, Ordering::SeqCst);

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
    Ok(())
}

/// Connect to an already-running official `dsh web` on the configured port,
/// or spawn the bundled engine. Returns `true` when an existing instance was
/// reused.
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
    if port != 0 && is_dsh_web_alive(port) {
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
            .runtime
            .lock()
            .unwrap()
            .as_ref()
            .map(|info| info.dsh_version.clone());
        *state.ready_url.lock().unwrap() = Some(url.clone());
        app::set_status(
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
            navigate(app, &url);
            // on_page_load reveals the window once the Web UI has painted.
        }
        return Ok(true);
    }
    crate::app::emit_log(
        state,
        Some(app),
        "INFO",
        "[check] 未发现已有实例，启动内置引擎".to_string(),
    );
    spawn_engine(app, state)?;
    Ok(false)
}

fn parse_web_url(line: &str) -> Option<String> {
    const MARKER: &str = "dsh web: ";
    let start = line.find(MARKER)? + MARKER.len();
    let rest = &line[start..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let url = &rest[..end];
    let parsed = url::Url::parse(url).ok()?;
    let port = parsed.port()?;
    if parsed.scheme() == "http" && parsed.host_str() == Some("127.0.0.1") && port != 0 {
        Some(parsed.to_string())
    } else {
        None
    }
}

/// Only the origin recorded in `ready_url` may be loaded by the main
/// WebView. Everything else is refused, so a stray local service or a
/// polluted engine log line cannot hijack the window.
pub fn is_allowed_web_url(state: &AppState, url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "http" || parsed.host_str() != Some("127.0.0.1") {
        return false;
    }
    let Some(port) = parsed.port() else {
        return false;
    };
    let ready = state.ready_url.lock().unwrap().clone();
    let Some(ready) = ready else {
        return false;
    };
    url::Url::parse(&ready)
        .map(|expected| {
            expected.scheme() == "http"
                && expected.host_str() == Some("127.0.0.1")
                && expected.port() == Some(port)
        })
        .unwrap_or(false)
}

/// The shell's own pages (packaged assets or the Vite dev server).
pub fn is_shell_url(url: &str) -> bool {
    url.starts_with("tauri://localhost")
        || url.starts_with("https://tauri.localhost")
        || url.starts_with("http://localhost:1420")
}

fn navigate(app: &AppHandle, url: &str) {
    let url = url.to_string();
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
        if !is_allowed_web_url(&state, &url) {
            state.logger.warn(&format!(
                "refusing navigation to non-whitelisted url: {url}"
            ));
            return;
        }
        if let Some(window) = app.get_webview_window("main") {
            if let Ok(parsed) = url::Url::parse(&url) {
                let _ = window.navigate(parsed);
            }
        }
    });
}

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
        app::push_log_tail(&state, line.clone());
        let _ = app.emit(
            "shell://log",
            serde_json::json!({
                "level": if is_stderr { "warn" } else { "info" },
                "line": line,
            }),
        );
        if !is_stderr {
            if let Some(url) = parse_web_url(&line) {
                let checklist = crate::app::config::checklist_required_full(&state);
                state.logger.info(&format!("web UI ready at {url}"));
                let version = state
                    .runtime
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|info| info.dsh_version.clone());
                *state.ready_url.lock().unwrap() = Some(url.clone());
                app::set_status(
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
                    navigate(&app, &url);
                }
            }
        }
    }
}

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
                app::set_status(
                    &state,
                    Some(&app),
                    "engine-stopped",
                    format!("引擎已退出（退出码 {code:?}）"),
                    None,
                    None,
                    None,
                    state
                        .runtime
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|info| info.dsh_version.clone()),
                );
                if restart {
                    thread::sleep(Duration::from_secs(2));
                    if state.stopping.load(Ordering::SeqCst) {
                        return;
                    }
                    app::set_status(
                        &state,
                        Some(&app),
                        "engine-starting",
                        "引擎异常退出，正在自动重启…",
                        None,
                        None,
                        None,
                        state
                            .runtime
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|info| info.dsh_version.clone()),
                    );
                    if let Err(err) = connect_existing_or_spawn(&app, &state) {
                        app::set_status(
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

pub fn stop_engine(state: &Arc<AppState>) {
    state.stopping.store(true, Ordering::SeqCst);
    let child = state.engine.lock().unwrap().take();
    if let Some(mut child) = child {
        let pid = child.id();
        state.logger.info(&format!("stopping engine (pid {pid})"));
        #[cfg(windows)]
        {
            let status = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if status.map(|s| !s.success()).unwrap_or(true) {
                // Restricted tokens and sandboxes can deny taskkill; the
                // child handle still allows TerminateProcess.
                let _ = child.kill();
            }
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
            for _ in 0..50 {
                if let Ok(Some(_)) = child.try_wait() {
                    let _ = child.wait();
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            let _ = child.kill();
        }
        let _ = child.wait();
    }
}

pub fn restart_engine(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    state.logger.info("restarting engine");
    stop_engine(state);
    state.stopping.store(false, Ordering::SeqCst);
    let version = state
        .runtime
        .lock()
        .unwrap()
        .as_ref()
        .map(|info| info.dsh_version.clone());
    app::set_status(
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
            app::set_status(
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

pub fn open_web_ui(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let url = state
        .ready_url
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Web UI 尚未就绪".to_string())?;
    navigate(app, &url);
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
