use crate::app::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn check_runtime_update(
    app: AppHandle,
) -> Result<crate::engine::runtime_update::RuntimeUpdateCheck, String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || crate::engine::runtime_update::check(&state))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn apply_runtime_update(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || crate::engine::runtime_update::apply(&app, &state))
        .await
        .map_err(|err| err.to_string())?
}
