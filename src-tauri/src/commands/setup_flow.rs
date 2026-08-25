//! Setup-flow commands with detailed live logging and the aggregated
//! three-row checklist shown in the frontend.

use crate::app::state::AppState;
use crate::core::process;
use crate::detection;
use crate::detection::model::{CheckStatus, DependencyId, DependencyInfo};
use crate::geo::model::GeoResult;
use serde::Serialize;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

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

struct SetupSessionGuard(std::sync::Arc<crate::detection::session::SetupSession>);

impl Drop for SetupSessionGuard {
    fn drop(&mut self) {
        self.0.finish();
    }
}

#[tauri::command]
pub async fn run_detection_v2(app: AppHandle) -> Result<SetupState, String> {
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

    for dependency in &dependencies {
        let status = format!("{:?}", dependency.status).to_lowercase();
        let version = dependency
            .version
            .as_deref()
            .map(|value| format!(" version={value}"))
            .unwrap_or_default();
        let path = dependency
            .path
            .as_deref()
            .map(|value| format!(" path={value}"))
            .unwrap_or_default();
        crate::app::emit_log(
            &state,
            Some(&app),
            if dependency.status == CheckStatus::Passed {
                "INFO"
            } else {
                "WARN"
            },
            format!(
                "[detect] {}: {status}{version}{path}",
                dependency.id.label()
            ),
        );
    }

    let mut all_passed = detection::gate_passed(&dependencies);
    if all_passed {
        match detection::aggregate::command_spec(&dependencies) {
            Ok(spec) => {
                crate::app::emit_log(
                    &state,
                    Some(&app),
                    "INFO",
                    format!(
                        "环境检测通过: dsh={} node={:?}",
                        spec.dsh_bin.display(),
                        spec.node_version
                    ),
                );
                *state.command_spec.lock().unwrap() = Some(spec);
                if let Some(spec) = state.command_spec.lock().unwrap().clone() {
                    if let Err(err) = crate::app::toolchain_cache::save(&state.home, &spec) {
                        state
                            .logger
                            .warn(&format!("failed to save toolchain cache: {err}"));
                    }
                }

                // Warm up dsh while the user is still viewing the completed
                // checklist. Previously the engine was first started only
                // after clicking "Enter Harness", which added a second
                // loading phase after the detection page.
                let engine_app = app.clone();
                let engine_state = state.clone();
                std::thread::spawn(move || {
                    if engine_state.stopping.load(Ordering::SeqCst) {
                        return;
                    }
                    engine_state.stopping.store(false, Ordering::SeqCst);
                    if let Err(err) =
                        crate::engine::connect_existing_or_spawn(&engine_app, &engine_state)
                    {
                        crate::app::set_status(
                            &engine_state,
                            Some(&engine_app),
                            "error",
                            "engineStartFailed",
                            Some(err),
                            None,
                            None,
                            None,
                        );
                    }
                });

                let update_app = app.clone();
                let update_state = state.clone();
                std::thread::spawn(move || {
                    let _ = crate::ui::tray::check_dsh_update_now(update_app, update_state);
                });
            }
            Err(err) => {
                *state.command_spec.lock().unwrap() = None;
                crate::app::toolchain_cache::clear(&state.home);
                crate::app::emit_log(
                    &state,
                    Some(&app),
                    "ERROR",
                    format!("environment gate passed but command spec failed: {err}"),
                );
                all_passed = false;
            }
        }
    } else {
        *state.command_spec.lock().unwrap() = None;
        crate::app::toolchain_cache::clear(&state.home);
        crate::app::emit_log(
            &state,
            Some(&app),
            "WARN",
            format!(
                "环境检测未全部通过（要求 Node {}，{}，{}）",
                crate::detection::requirement::NODE_REQUIREMENT,
                crate::detection::requirement::PACKAGE_MANAGER_REQUIREMENT,
                crate::detection::requirement::DSH_REQUIREMENT,
            ),
        );
    }
    crate::app::emit_log(
        &state,
        Some(&app),
        if all_passed { "INFO" } else { "WARN" },
        format!("[detect] completed: allPassed={all_passed}"),
    );
    let source_policy = detection::resolve_sources(region.region);
    Ok(SetupState {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies,
        all_passed,
        source_policy,
        geo: region,
    })
}

#[tauri::command]
pub async fn install_dependency_v2(
    app: AppHandle,
    dependency: DependencyId,
) -> Result<SetupState, String> {
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
        format!("[install] {} 安装完成，重新检测环境", help.title),
    );

    run_detection_v2(app).await
}

#[tauri::command]
pub fn recheck_environment_v2(app: AppHandle) {
    // The tray's recheck action stops the current engine first. Allow the
    // successful detection below to warm a fresh engine again.
    let initial_state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    initial_state.stopping.store(false, Ordering::SeqCst);
    let app_for_task = app.clone();
    std::thread::spawn(move || {
        let state: Arc<AppState> = app_for_task.state::<Arc<AppState>>().inner().clone();
        let result = tauri::async_runtime::block_on(run_detection_v2(app_for_task.clone()));
        match result {
            Ok(setup) => {
                if !setup.all_passed {
                    crate::app::set_status(
                        &state,
                        Some(&app_for_task),
                        "error",
                        "environmentFailed",
                        None,
                        None,
                        None,
                        None,
                    );
                }
            }
            Err(err) => {
                crate::app::emit_log(
                    &state,
                    Some(&app_for_task),
                    "ERROR",
                    format!("环境检测失败: {err}"),
                );
            }
        }
    });
}

#[tauri::command]
pub fn begin_setup_v2(app: AppHandle) {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    if state.setup_started.swap(true, Ordering::SeqCst) {
        return;
    }
    recheck_environment_v2(app);
}

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
