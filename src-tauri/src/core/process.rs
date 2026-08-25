//! Cross-platform external-process primitives.
//!
//! This is the single low-level entry point for launching external
//! processes from the shell. It owns two invariants:
//!
//! 1. No shell-string concatenation: every invocation goes through an
//!    argument array (`Command::arg`/`args`).
//! 2. No visible console: child processes spawned from the Windows GUI
//!    shell are created with `CREATE_NO_WINDOW` so users never see a
//!    flashing console window.
//!
//! The module only depends on `std`; domain logic lives in `detection/`,
//! `engine/` and `update/` on top of it.

use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
use std::process::Stdio;

/// Prevent console-subsystem child processes from opening a visible console
/// window when the desktop shell itself is a Windows GUI application.
/// Tauri uses the same CREATE_NO_WINDOW behavior for Windows sidecars.
pub fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// Windows command names need `.exe`/`.cmd`/`.bat` resolution; Unix shells
/// rely on the executable bit. Returns the candidate file names for a bare
/// command name on the current platform.
pub fn command_candidates(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            format!("{name}.bat"),
            name.to_string(),
        ]
    } else {
        vec![name.to_string()]
    }
}

/// Locate an executable on `PATH`. Windows resolves `.exe`/`.cmd`/`.bat` in
/// order; the first existing file wins. Returns `None` when the command is
/// not on `PATH`.
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    let names = command_candidates(name);
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for candidate in &names {
            let full = directory.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

/// Build the platform-correct `Command` for an executable path that may be a
/// `.cmd`/`.bat` wrapper on Windows (npm/pnpm/dsh shims): those must run
/// through `cmd.exe /d /c` so the shell semantics are preserved.
pub fn command_for(path: &std::path::Path) -> Command {
    let is_cmd_wrapper = cfg!(windows)
        && matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("cmd") | Some("bat")
        );
    if is_cmd_wrapper {
        let mut command = Command::new("cmd.exe");
        command.args(["/d", "/c"]).arg(path);
        command
    } else {
        Command::new(path)
    }
}

/// Kill a process and, on Windows, its whole descendant tree via
/// `taskkill /T /F`. Falls back to `Child::kill` when taskkill is denied
/// (restricted tokens and sandboxes).
pub fn kill_process_tree(child: &mut std::process::Child) {
    let pid = child.id();
    #[cfg(windows)]
    {
        let mut taskkill = Command::new("taskkill");
        taskkill
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_console(&mut taskkill);
        let status = taskkill.status();
        if status.map(|s| !s.success()).unwrap_or(true) {
            // Restricted tokens and sandboxes can deny taskkill; the child
            // handle still allows TerminateProcess.
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
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = child.kill();
    }
    let _ = child.wait();
}

/// Kill a previously detached engine by PID. The caller must only pass a PID
/// recovered from the engine handoff record after verifying its local port.
pub fn kill_process_id(pid: u32) {
    #[cfg(windows)]
    {
        let mut taskkill = Command::new("taskkill");
        taskkill
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_console(&mut taskkill);
        let _ = taskkill.status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}
