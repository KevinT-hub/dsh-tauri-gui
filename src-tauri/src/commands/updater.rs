use crate::app::AppState;
use crate::update::{checker, downloader, AppUpdateInfo};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_update_notice(app: AppHandle) -> Option<crate::app::UpdateNotice> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let notice = state.update_notice.lock().unwrap().clone();
    notice
}

#[tauri::command]
pub fn get_app_update(app: AppHandle) -> Option<AppUpdateInfo> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let update = state.app_update.lock().unwrap().clone();
    update
}

#[tauri::command]
pub async fn check_app_update(_app: AppHandle) -> Result<crate::update::AppUpdateInfo, String> {
    checker::check_update().await
}

#[tauri::command]
pub async fn apply_app_update(app: AppHandle) -> Result<(), String> {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    downloader::download_and_install(app, state).await
}
