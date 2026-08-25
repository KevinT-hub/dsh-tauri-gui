use crate::app::state::AppState;
use crate::core::process;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_dsh_update(app: AppHandle) -> Option<crate::update::DshUpdateInfo> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let update = state.dsh_update.lock().unwrap().clone();
    update
}

#[tauri::command]
pub async fn install_dsh_update(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let was_running =
        state.engine.lock().unwrap().is_some() || state.engine_session.lock().unwrap().is_some();
    if was_running {
        crate::engine::stop_engine(&state);
    }

    let npm =
        process::find_on_path("npm").ok_or_else(|| "未找到 npm，请先确保 npm 可用".to_string())?;
    let mut cmd = process::command_for(&npm);
    cmd.args(["install", "-g", "@deepseek-ai/dsh@latest"]);
    process::hide_console(&mut cmd);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    crate::app::emit_log(
        &state,
        Some(&app),
        "INFO",
        "开始安装 DeepSeek Harness 更新: npm install -g @deepseek-ai/dsh@latest".to_string(),
    );

    let mut child = cmd.spawn().map_err(|err| format!("无法启动 npm: {err}"))?;
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

    let status = child
        .wait()
        .map_err(|err| format!("npm 安装进程异常: {err}"))?;
    if !status.success() {
        if was_running {
            let _ = crate::engine::connect_existing_or_spawn(&app, &state);
        }
        return Err(format!("npm install 失败（退出码 {:?}）", status.code()));
    }

    crate::app::emit_log(
        &state,
        Some(&app),
        "INFO",
        "DeepSeek Harness 更新安装完成，准备重启并重新检测".to_string(),
    );
    crate::app::config::reset_checklist(&state)?;
    *state.command_spec.lock().unwrap() = None;
    *state.last_detection.lock().unwrap() = None;
    Ok(())
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
            format!("[dsh-update] {}", crate::core::redact::redact(&line)),
        );
    }
}
