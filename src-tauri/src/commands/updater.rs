use crate::app::AppState;
use crate::update::{checker, downloader};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn check_app_update(_app: AppHandle) -> Result<crate::update::AppUpdateInfo, String> {
    checker::check_update().await
}

#[tauri::command]
pub async fn apply_app_update(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    downloader::download_and_install(app, state).await
}
