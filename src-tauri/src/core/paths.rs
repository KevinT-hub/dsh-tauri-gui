use std::path::PathBuf;

fn user_base() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The operating-system user home directory (used as the engine's default
/// workspace when no workspace is configured).
pub fn user_home_dir() -> PathBuf {
    absolutize(user_base())
}

pub fn absolutize(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}

/// Desktop-shell data home (`~/.dsh-tauri-gui`): shell config, logs, runtime.
pub fn resolve_shell_home() -> PathBuf {
    if let Some(value) = std::env::var_os("DSH_TAURI_GUI_HOME") {
        return absolutize(PathBuf::from(value));
    }
    absolutize(user_base().join(".dsh-tauri-gui"))
}

/// Engine data home. Defaults to the official `~/.dsh` so the desktop app
/// shares settings, credentials, sessions and profiles with the official
/// DeepSeek Harness WebUI/CLI. Overridable via `DSH_TAURI_ENGINE_HOME` or
/// the shell config `engineHome`.
pub fn resolve_engine_home() -> PathBuf {
    if let Some(value) = std::env::var_os("DSH_TAURI_ENGINE_HOME") {
        return absolutize(PathBuf::from(value));
    }
    absolutize(user_base().join(".dsh"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn home_prefers_explicit_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let explicit = std::env::temp_dir().join("dsh-custom-home");
        std::env::set_var("DSH_TAURI_GUI_HOME", &explicit);
        assert_eq!(resolve_shell_home(), explicit);
        std::env::remove_var("DSH_TAURI_GUI_HOME");
    }

    #[test]
    fn home_uses_userprofile_when_dir_fails() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Simulate a broken `dirs` lookup by pointing USERPROFILE at a temp dir.
        let temp = std::env::temp_dir().join("dsh-tauri-gui-path-test");
        std::env::set_var("USERPROFILE", &temp);
        std::env::remove_var("DSH_TAURI_GUI_HOME");
        let home = resolve_shell_home();
        assert!(home.is_absolute());
        assert!(home.ends_with(".dsh-tauri-gui"));
    }

    #[test]
    fn engine_home_defaults_to_official_dsh() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp = std::env::temp_dir().join("dsh-tauri-gui-path-test-engine");
        std::env::set_var("USERPROFILE", &temp);
        std::env::remove_var("DSH_TAURI_ENGINE_HOME");
        let home = resolve_engine_home();
        assert!(home.is_absolute());
        assert!(home.ends_with(".dsh"));
    }
}
