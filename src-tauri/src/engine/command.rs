//! Construction of the official `dsh web` command from a validated
//! `CommandSpec`. No bundled-runtime knowledge here: the spec's `dsh_bin` is
//! the only thing that runs.

use crate::core::process;
use crate::detection::model::CommandSpec;
use std::process::Command;

/// Build the platform-correct `Command` for the validated dsh executable
/// (`.cmd`/`.bat` shims on Windows run through `cmd.exe /d /c`).
pub fn dsh_command(spec: &CommandSpec) -> Command {
    process::command_for(&spec.dsh_bin)
}

/// dsh opens the default browser unless this flag is present; the Tauri
/// WebView is the only UI surface owned by the desktop shell.
pub fn append_web_command_args(command: &mut Command, port: u16) {
    command
        .arg("web")
        .arg("--no-open")
        .arg("--port")
        .arg(port.to_string());
}

#[cfg(test)]
mod tests {
    use super::append_web_command_args;
    use std::process::Command;

    #[test]
    fn dsh_web_command_disables_default_browser() {
        let mut command = Command::new("dsh");
        append_web_command_args(&mut command, 3080);

        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(args, vec!["web", "--no-open", "--port", "3080"]);
    }
}
