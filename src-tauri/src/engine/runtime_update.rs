use crate::app::config::RuntimeMode;
use crate::app::{self, AppState, RuntimeInfo};
use crate::core::redact::redact;
use crate::engine;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeUpdateCheck {
    pub current: String,
    pub latest: String,
    pub update_available: bool,
}

fn node_and_runtime(state: &Arc<AppState>) -> Result<(RuntimeInfo, PathBuf), String> {
    let runtime = state
        .runtime
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "运行时尚未就绪".to_string())?;
    if runtime.mode != RuntimeMode::Bundled {
        return Err(
            "dsh hot updates are disabled in system runtime mode; update dsh with your system package manager"
                .to_string(),
        );
    }
    let npm_cli = crate::engine::bootstrap::npm_cli_path(&state.runtime_dir)
        .ok_or_else(|| format!("bundled npm missing under {}", state.runtime_dir.display()))?;
    Ok((runtime, npm_cli))
}

fn registry_url(state: &Arc<AppState>) -> String {
    state
        .config
        .lock()
        .unwrap()
        .npm_registry
        .trim_end_matches('/')
        .to_string()
}

/// Mirror-first, official-npm fallback: the configured registry is tried
/// first, then `registry.npmjs.org` when it is unreachable or fails.
fn registry_candidates(state: &Arc<AppState>) -> Vec<String> {
    let primary = registry_url(state);
    let fallback = "https://registry.npmjs.org".to_string();
    let mut candidates = vec![primary.clone()];
    if fallback != primary {
        candidates.push(fallback);
    }
    candidates
}

const FETCH_SCRIPT: &str = r#"
const url = process.argv[1];
fetch(url, { headers: { accept: 'application/json' }, signal: AbortSignal.timeout(15000) })
  .then((r) => { if (!r.ok) throw new Error('HTTP ' + r.status); return r.json(); })
  .then((j) => process.stdout.write(String(j.version || '')))
  .catch((e) => { console.error(String((e && e.message) || e)); process.exit(1); });
"#;

pub fn check(state: &Arc<AppState>) -> Result<RuntimeUpdateCheck, String> {
    let (runtime, _) = node_and_runtime(state)?;
    let mut last_error = None;
    let mut latest = None;

    for registry in registry_candidates(state) {
        let url = format!("{registry}/@deepseek-ai/dsh/latest");
        let mut command = Command::new(&runtime.node_exe);
        command.args(["-e", FETCH_SCRIPT, &url]);
        engine::hide_console(&mut command);
        match command.output() {
            Ok(output) if output.status.success() => {
                latest = Some(
                    String::from_utf8(output.stdout)
                        .map_err(|err| err.to_string())?
                        .trim()
                        .to_string(),
                );
                break;
            }
            Ok(output) => {
                last_error = Some(format!(
                    "查询最新版本失败 ({}): {}",
                    registry,
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            Err(err) => {
                last_error = Some(format!("无法查询 npm registry ({}): {err}", registry));
            }
        }
    }

    let latest =
        latest.ok_or_else(|| last_error.unwrap_or_else(|| "无可用 npm registry".to_string()))?;
    let current = runtime.dsh_version.clone();
    Ok(RuntimeUpdateCheck {
        current: current.clone(),
        latest: latest.clone(),
        update_available: latest != current,
    })
}

pub fn check_and_notify(app: &AppHandle, state: &Arc<AppState>) {
    match check(state) {
        Ok(result) => {
            let _ = app.emit("shell://runtime-update", &result);
            app::set_status(
                state,
                Some(app),
                state.status.lock().unwrap().phase,
                if result.update_available {
                    format!(
                        "发现新版本 dsh {}（当前 {}）",
                        result.latest, result.current
                    )
                } else {
                    format!("dsh 核心已是最新版本（{}）", result.current)
                },
                None,
                None,
                None,
                Some(result.current),
            );
        }
        Err(err) => {
            state.logger.error(&err);
            let _ = app.emit("shell://runtime-update-error", err);
        }
    }
}

fn emit_update_line(app: &AppHandle, state: &Arc<AppState>, line: String) {
    let line = redact(&line);
    state
        .logger
        .log("engine", "INFO", &format!("[updater] {line}"));
    let _ = app.emit(
        "shell://log",
        serde_json::json!({ "level": "info", "line": format!("[updater] {line}") }),
    );
}

/// Run one `npm install` against a single registry; returns the process
/// failure detail on a non-zero exit so the caller can retry another source.
fn run_npm_install(
    app: &AppHandle,
    state: &Arc<AppState>,
    runtime: &RuntimeInfo,
    npm_cli: &PathBuf,
    prefix_dir: &PathBuf,
    spec: &str,
    registry: &str,
) -> Result<(), String> {
    let separator = if cfg!(windows) { ";" } else { ":" };
    let mut cmd = Command::new(&runtime.node_exe);
    cmd.arg(npm_cli)
        .arg("install")
        .arg("--prefix")
        .arg(prefix_dir)
        .arg(spec)
        .arg("--registry")
        .arg(registry)
        .arg("--no-audit")
        .arg("--no-fund")
        .arg("--no-update-notifier")
        .current_dir(&state.home)
        .env("npm_config_registry", registry)
        .env(
            "PATH",
            format!(
                "{}{separator}{}{separator}{}",
                runtime.node_exe.parent().unwrap().display(),
                state.runtime_dir.join("app/node_modules/.bin").display(),
                state.runtime_dir.join("tools/node_modules/.bin").display(),
            ),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    engine::hide_console(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|err| format!("无法启动 npm 更新: {err}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout 管道不可用".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "stderr 管道不可用".to_string())?;

    let app_out = app.clone();
    let state_out = state.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            emit_update_line(&app_out, &state_out, line);
        }
    });
    let app_err = app.clone();
    let state_err = state.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            emit_update_line(&app_err, &state_err, line);
        }
    });

    let status = child
        .wait()
        .map_err(|err| format!("npm 更新进程异常: {err}"))?;
    if !status.success() {
        return Err(format!("npm 更新失败（退出码 {:?}）", status.code()));
    }
    Ok(())
}

pub fn apply(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let (runtime, npm_cli) = node_and_runtime(state)?;
    let app_dir = state.runtime_dir.join("app");

    // Resolve the exact version before touching anything, so the staging
    // install can never diverge from what the user was offered.
    let target_version = check(state)?.latest;
    let spec = format!("@deepseek-ai/dsh@{target_version}");

    // Native modules (node-pty etc.) are loaded by the running engine and can
    // lock files on Windows; stop the engine before replacing them.
    engine::stop_engine(state);

    app::set_status(
        state,
        Some(app),
        "updating",
        "正在下载并安装最新 dsh 核心…",
        None,
        None,
        None,
        Some(target_version.clone()),
    );

    let pid = std::process::id();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let staging = state
        .runtime_dir
        .join(format!(".app-staging-{pid}-{nonce}"));
    let backup = state.runtime_dir.join(format!(".app-old-{pid}-{nonce}"));
    let cleanup = |path: &std::path::Path| {
        let _ = std::fs::remove_dir_all(path);
    };
    cleanup(&staging);
    if staging.exists() {
        return Err("cannot clean previous staging directory; files may be locked".to_string());
    }
    std::fs::create_dir_all(&staging).map_err(|err| err.to_string())?;

    let mut last_error = None;
    let mut installed = false;
    for registry in registry_candidates(state) {
        state.logger.info(&format!(
            "installing dsh core {target_version} from {registry}"
        ));
        match run_npm_install(app, state, &runtime, &npm_cli, &staging, &spec, &registry) {
            Ok(()) => {
                installed = true;
                break;
            }
            Err(err) => {
                last_error = Some(format!("{registry}: {err}"));
            }
        }
    }

    if !installed {
        let detail = last_error.unwrap_or_else(|| "所有 npm registry 均失败".to_string());
        cleanup(&staging);
        app::set_status(
            state,
            Some(app),
            "error",
            "dsh 核心更新失败",
            Some(detail.clone()),
            None,
            None,
            Some(target_version),
        );
        let _ = engine::spawn_engine(app, state);
        return Err(detail);
    }

    let staged_package = staging.join("node_modules/@deepseek-ai/dsh");
    if !staged_package.join("package.json").exists() {
        cleanup(&staging);
        app::set_status(
            state,
            Some(app),
            "error",
            "dsh 核心更新失败",
            Some("staging 目录缺少 @deepseek-ai/dsh 包".to_string()),
            None,
            None,
            Some(target_version),
        );
        let _ = engine::spawn_engine(app, state);
        return Err("staging install did not produce @deepseek-ai/dsh".to_string());
    }
    if let Err(err) = crate::engine::bootstrap::resolve_dsh_bin(&staged_package) {
        cleanup(&staging);
        app::set_status(
            state,
            Some(app),
            "error",
            "dsh 核心更新失败",
            Some(err.clone()),
            None,
            None,
            Some(target_version),
        );
        let _ = engine::spawn_engine(app, state);
        return Err(err);
    }

    cleanup(&backup);
    if app_dir.exists() {
        std::fs::rename(&app_dir, &backup).map_err(|err| {
            cleanup(&staging);
            let _ = engine::spawn_engine(app, state);
            format!("无法移动旧 app 目录: {err}")
        })?;
    }
    if let Err(err) = std::fs::rename(&staging, &app_dir) {
        if backup.exists() && !app_dir.exists() {
            let _ = std::fs::rename(&backup, &app_dir);
        }
        cleanup(&staging);
        app::set_status(
            state,
            Some(app),
            "error",
            "dsh 核心更新失败",
            Some(err.to_string()),
            None,
            None,
            Some(target_version),
        );
        let _ = engine::spawn_engine(app, state);
        return Err(err.to_string());
    }
    cleanup(&backup);

    let version = crate::engine::bootstrap::read_dsh_version(&state.runtime_dir)?;
    let manifest = serde_json::json!({
        "dshVersion": version,
        "nodeVersion": crate::engine::bootstrap::read_node_version(&state.runtime_dir).unwrap_or_default(),
        "pnpmVersion": crate::engine::bootstrap::read_pnpm_version(&state.runtime_dir).unwrap_or_default(),
        "createdAt": chrono::Local::now().to_rfc3339(),
    });
    std::fs::write(
        state.runtime_dir.join("runtime.json"),
        serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;

    app::set_status(
        state,
        Some(app),
        "engine-starting",
        format!("dsh 核心已更新到 {version}，正在重启引擎…"),
        None,
        None,
        None,
        Some(version),
    );
    // Fresh engine from the updated runtime.
    engine::spawn_engine(app, state)?;
    Ok(())
}
