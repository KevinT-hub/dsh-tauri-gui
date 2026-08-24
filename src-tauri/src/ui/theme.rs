use crate::app::AppState;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, Theme};

/// Resolved shell theme state. `effective` is `light`/`dark` when it can be
/// derived from the shell mode or the web UI preference, otherwise `system`
/// (the frontend then follows `prefers-color-scheme`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeState {
    pub mode: String,
    pub webui_preference: Option<String>,
    pub effective: String,
}

fn read_settings(home: &std::path::Path) -> Option<serde_yaml::Value> {
    let path = home.join("settings.yaml");
    let text = fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&text).ok()
}

/// Official Web UI persists `ui-theme.preference` in `$DSH_HOME/settings.yaml`
/// (light | dark | system). The shell reads it at startup and mirrors it.
pub fn read_webui_preference(home: &std::path::Path) -> Option<String> {
    let root = read_settings(home)?;
    let preference = root.get("ui-theme")?.get("preference")?.as_str()?;
    if preference == "light" || preference == "dark" || preference == "system" {
        Some(preference.to_string())
    } else {
        None
    }
}

/// Write the official `ui-theme.preference` into `$DSH_HOME/settings.yaml`
/// while preserving every other namespace and comment in the document.
pub fn write_webui_preference(home: &Path, mode: &str) -> Result<(), String> {
    let path = home.join("settings.yaml");
    fs::create_dir_all(home).map_err(|err| err.to_string())?;
    let mut lines: Vec<String> = fs::read_to_string(&path)
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default();

    let ui_theme_index = lines.iter().position(|line| {
        line.split(':')
            .next()
            .map(|key| key.trim() == "ui-theme")
            .unwrap_or(false)
    });

    let mut found_preference = false;
    if let Some(index) = ui_theme_index {
        for line in lines.iter_mut().skip(index + 1) {
            if !line.starts_with(' ') && !line.is_empty() {
                break;
            }
            if let Some(key_index) = line.find("preference:") {
                let indent = line[..key_index].to_string();
                *line = format!("{indent}preference: {mode}");
                found_preference = true;
                break;
            }
        }
        if !found_preference {
            lines.insert(index + 1, format!("  preference: {mode}"));
        }
    } else {
        lines.push("ui-theme:".to_string());
        lines.push(format!("  preference: {mode}"));
    }

    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    fs::write(&path, content).map_err(|err| err.to_string())
}

pub fn theme_state(state: &AppState) -> ThemeState {
    let mode = state.config.lock().unwrap().ui_theme.clone();
    let webui_preference = read_webui_preference(&state.engine_home);
    let effective = match mode.as_str() {
        "light" | "dark" => mode.clone(),
        _ => match webui_preference.as_deref() {
            Some("light") | Some("dark") => webui_preference.clone().unwrap(),
            _ => "system".to_string(),
        },
    };
    ThemeState {
        mode,
        webui_preference,
        effective,
    }
}

pub fn set_ui_theme(
    app: &AppHandle,
    state: &Arc<AppState>,
    mode: String,
) -> Result<ThemeState, String> {
    if !["light", "dark", "system"].contains(&mode.as_str()) {
        return Err(format!("invalid theme mode: {mode}"));
    }
    {
        let mut config = state.config.lock().unwrap();
        config.ui_theme = mode.clone();
        crate::app::config::save(&config, &state.config_path)?;
    }
    write_webui_preference(&state.engine_home, &mode)?;
    state
        .logger
        .info(&format!("theme set to {mode} (shell + web UI)"));
    let next = theme_state(state);
    let _ = app.emit(crate::app::events::THEME_EVENT, &next);
    apply_window_theme(app, state);
    Ok(next)
}

/// Apply the resolved theme to the native window chrome (title bar / frame).
/// `None` lets the OS decide, which is what "跟随系统" means.
pub fn apply_window_theme(app: &AppHandle, state: &AppState) {
    let effective = theme_state(state).effective;
    let theme = match effective.as_str() {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    };
    let background = match effective.as_str() {
        "dark" => Some(Color(13, 17, 23, 255)),
        "light" => Some(Color(246, 248, 250, 255)),
        _ => None,
    };
    for label in ["main", "update-overlay"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_theme(theme);
        }
    }
    // The overlay window is transparent; only the main window gets a solid
    // theme-matching background to avoid any pre-paint black/white flash.
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_background_color(background);
    }
}

pub fn sync_window_theme(app: &AppHandle, state: &Arc<AppState>) {
    apply_window_theme(app, state);
}

/// Watch the web UI settings document and re-emit the mapped theme when the
/// user changes the official Appearance preference.
pub fn spawn_theme_watcher(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let settings_path = state.engine_home.join("settings.yaml");
        let mut last_modified: Option<SystemTime> =
            fs::metadata(&settings_path).and_then(|m| m.modified()).ok();
        loop {
            std::thread::sleep(Duration::from_secs(3));
            let modified = fs::metadata(&settings_path).and_then(|m| m.modified()).ok();
            if modified != last_modified {
                last_modified = modified;
                let current = theme_state(&state);
                let _ = app.emit(crate::app::events::THEME_EVENT, current);
            }
            // Keep the native title bar in sync, including OS theme changes
            // while the shell is in "system" mode.
            apply_window_theme(&app, &state);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn write_preference_preserves_other_namespaces() {
        let dir =
            std::env::temp_dir().join(format!("dsh-theme-test-{}-preserve", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.yaml");
        fs::write(
            &path,
            "ui-onboarding:\n  welcomeNoticeVersion: 2026-08-13.1\n",
        )
        .unwrap();
        write_webui_preference(&dir, "dark").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("ui-theme:"));
        assert!(content.contains("preference: dark"));
        assert!(content.contains("ui-onboarding:"));
        assert!(content.contains("welcomeNoticeVersion: 2026-08-13.1"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_preference_updates_existing_value() {
        let dir =
            std::env::temp_dir().join(format!("dsh-theme-test-{}-update", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.yaml");
        fs::write(&path, "ui-theme:\n  preference: light\n").unwrap();
        write_webui_preference(&dir, "system").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("preference: system"));
        let _ = fs::remove_dir_all(&dir);
    }
}
