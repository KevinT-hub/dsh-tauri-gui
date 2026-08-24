//! Setup-flow commands: environment detection, install help and the
//! "进入" gate. Business logic lives in `detection/` and `geo/`; these
//! commands only validate input, enforce the user-confirmation boundary and
//! forward to the domain.

use crate::app::state::AppState;
use crate::core::process;
use crate::detection;
use crate::detection::model::{DependencyId, DependencyInfo};
use crate::geo::model::GeoResult;
use serde::Serialize;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Full snapshot handed to the setup screen after each detection run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub app_version: String,
    pub dependencies: Vec<DependencyInfo>,
    pub all_passed: bool,
    pub source_policy: detection::SourcePolicy,
    pub geo: GeoResult,
}

fn resolve_region(state: &AppState) -> GeoResult {
    crate::geo::resolve(&state.geo_cache)
}

/// RAII guard that makes sure `SetupSession::finish` is called even when the
/// detection future returns an error or is dropped early.
struct SetupSessionGuard(std::sync::Arc<crate::detection::session::SetupSession>);

impl Drop for SetupSessionGuard {
    fn drop(&mut self) {
        self.0.finish();
    }
}

/// Run the environment detection (node/npm/pnpm/dsh), resolve the geo region
/// and return the full setup snapshot. Idempotent for the frontend: repeated
/// calls simply re-run the probes.
#[tauri::command]
pub async fn run_detection(app: AppHandle) -> Result<SetupState, String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    if !state.setup_session.begin() {
        return Err("检测正在进行中".to_string());
    }
    let _guard = SetupSessionGuard(state.setup_session.clone());
    run_detection_inner(app, state).await
}

async fn run_detection_inner(app: AppHandle, state: Arc<AppState>) -> Result<SetupState, String> {
    let region = resolve_region(&state);
    crate::app::emit_log(
        &state,
        Some(&app),
        "INFO",
        format!(
            "开始环境检测: region={:?} node/npm/pnpm/dsh 探测中…",
            region.region
        ),
    );

    let dependencies = tauri::async_runtime::spawn_blocking(detection::detect_all)
        .await
        .map_err(|err| err.to_string())?;
    *state.last_detection.lock().unwrap() = Some(dependencies.clone());

    let all_passed = detection::gate_passed(&dependencies);
    if all_passed {
        match detection::aggregate::command_spec(&dependencies) {
            Ok(spec) => {
                state.logger.info(&format!(
                    "环境检测通过: dsh={} node={:?}",
                    spec.dsh_bin.display(),
                    spec.node_version
                ));
                *state.command_spec.lock().unwrap() = Some(spec);
            }
            Err(err) => {
                state
                    .logger
                    .warn(&format!("gate passed but spec failed: {err}"));
            }
        }
    } else {
        state.logger.warn(&format!(
            "环境检测未全部通过（要求: Node {}，{}，{}）",
            crate::detection::requirement::NODE_REQUIREMENT,
            crate::detection::requirement::PACKAGE_MANAGER_REQUIREMENT,
            crate::detection::requirement::DSH_REQUIREMENT,
        ));
    }
    let source_policy = detection::resolve_sources(region.region);
    Ok(SetupState {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies,
        all_passed,
        source_policy,
        geo: region,
    })
}

/// Legacy compatibility hook for older frontends. The current flow no longer
/// uses "screen shown" as the completion signal.
#[tauri::command]
pub fn mark_setup_seen(app: AppHandle) {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    crate::app::config::mark_setup_seen(&state);
}

/// Run the user-confirmed installation command for one missing dependency
/// (e.g. `npm install -g @deepseek-ai/dsh`), streaming output to the log,
/// then re-run detection automatically. The frontend must only call this
/// after an explicit user click.
#[tauri::command]
pub async fn install_dependency(app: AppHandle, dependency: DependencyId) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let region = resolve_region(&state);
    let policy = detection::resolve_sources(region.region);
    let help = detection::installer::help_for(dependency, &policy);

    let command = help
        .command
        .ok_or_else(|| format!("{} 需要手动安装，请打开官方页面后重试", help.title))?;

    crate::app::emit_log(
        &state,
        Some(&app),
        "INFO",
        format!("[install] 执行安装: {}", command.join(" ")),
    );

    let (program, args) = split_command(&command);
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    process::hide_console(&mut cmd);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|err| format!("无法启动安装命令: {err}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "安装进程 stdout 不可用".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "安装进程 stderr 不可用".to_string())?;

    let app_out = app.clone();
    let state_out = state.clone();
    std::thread::spawn(move || pump_install_lines(app_out, state_out, stdout));
    let app_err = app.clone();
    let state_err = state.clone();
    std::thread::spawn(move || pump_install_lines(app_err, state_err, stderr));

    let status = child.wait().map_err(|err| format!("安装进程异常: {err}"))?;
    if !status.success() {
        return Err(format!("安装命令失败（退出码 {:?}）", status.code()));
    }

    crate::app::emit_log(
        &state,
        Some(&app),
        "INFO",
        format!("[install] {} 安装完成，重新检测环境…", help.title),
    );
    let _ = run_detection(app).await;
    Ok(())
}

/// Split a `Vec<String>` command into `(program, args)` where the program
/// may itself be a `.cmd`/`.bat` wrapper on Windows.
fn split_command(command: &[String]) -> (String, Vec<String>) {
    let (program, rest) = command.split_first().expect("non-empty command");
    (program.clone(), rest.to_vec())
}

fn pump_install_lines(
    app: AppHandle,
    state: Arc<AppState>,
    stream: impl std::io::Read + Send + 'static,
) {
    use std::io::BufRead;
    for line in std::io::BufReader::new(stream)
        .lines()
        .map_while(Result::ok)
    {
        crate::app::emit_log(
            &state,
            Some(&app),
            "INFO",
            format!("[install] {}", crate::core::redact::redact(&line)),
        );
    }
}

/// Trigger the engine start after the setup screen's "进入" button.
/// Completion is recorded first so the launched engine may auto-navigate;
/// if startup fails, the marker is rolled back.
#[tauri::command]
pub fn enter_harness(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let (
        previous_first_run_completed,
        previous_last_checklist_version,
        previous_setup_seen_version,
    ) = {
        let config = state.config.lock().unwrap();
        (
            config.first_run_completed,
            config.last_checklist_version.clone(),
            config.setup_seen_version.clone(),
        )
    };
    crate::app::config::mark_checklist_completed(&state, &app_version)?;
    match crate::engine::connect_existing_or_spawn(&app, &state) {
        Ok(_) => Ok(()),
        Err(err) => {
            {
                let mut config = state.config.lock().unwrap();
                config.first_run_completed = previous_first_run_completed;
                config.last_checklist_version = previous_last_checklist_version;
                config.setup_seen_version = previous_setup_seen_version;
                if let Err(save_err) = crate::app::config::save(&config, &state.config_path) {
                    state.logger.warn(&format!(
                        "failed to roll back checklist marker after enter failed: {save_err}"
                    ));
                }
            }
            state.first_run_marked.store(false, Ordering::SeqCst);
            crate::engine::stop_engine(&state);
            Err(err)
        }
    }
}

/// Re-run detection from the tray or settings (manual escape hatch when the
/// setup screen is not shown for this version).
#[tauri::command]
pub fn recheck_environment(app: AppHandle) {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let app_for_task = app.clone();
    std::thread::spawn(move || {
        let result = tauri::async_runtime::block_on(run_detection(app_for_task));
        match result {
            Ok(setup) => {
                let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
                if setup.all_passed {
                    crate::app::emit_log(
                        &state,
                        Some(&app),
                        "INFO",
                        "环境检测通过，启动引擎…".to_string(),
                    );
                    if let Err(err) = crate::engine::connect_existing_or_spawn(&app, &state) {
                        crate::app::set_status(
                            &state,
                            Some(&app),
                            "error",
                            "引擎启动失败",
                            Some(err),
                            None,
                            None,
                            None,
                        );
                    }
                } else {
                    crate::app::set_status(
                        &state,
                        Some(&app),
                        "error",
                        "环境检测未通过",
                        Some("缺少必需依赖，请打开主窗口查看详情。".to_string()),
                        None,
                        None,
                        None,
                    );
                }
            }
            Err(err) => {
                crate::app::emit_log(&state, Some(&app), "ERROR", format!("环境检测失败: {err}"));
            }
        }
    });
    let _ = Ordering::SeqCst; // keep the atomic import stable for clippy
}

/// One-shot detection used by the boot path when no engine spec exists yet
/// (replaces the old `begin_bootstrap`).
#[tauri::command]
pub fn begin_setup(app: AppHandle) {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    if state.setup_started.swap(true, Ordering::SeqCst) {
        return;
    }
    recheck_environment(app);
}
