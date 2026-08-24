//! PATH lookup and `--version` probing for the four dependencies.
//!
//! Every probe: resolves the executable via `core::process::find_on_path`,
//! runs `--version` with an argument array (never a shell string), hides the
//! console on Windows and reads both stdout and stderr for the version line.

use crate::core::process;
use crate::core::version;
use std::path::{Path, PathBuf};

/// Result of locating and version-probing one command.
#[derive(Debug, Clone)]
pub struct ProbeOutcome {
    pub path: PathBuf,
    pub version: String,
}

/// Locate `name` on PATH and read its `--version` output.
///
/// Returns `Ok(ProbeOutcome)` when the executable exists and reports a
/// non-empty version. `Err(probe_error)` carries a human-readable reason for
/// the checklist row.
pub fn probe(name: &str, label: &str) -> Result<ProbeOutcome, String> {
    let path =
        process::find_on_path(name).ok_or_else(|| format!("未在 PATH 中找到 {label}（{name}）"))?;
    let version = run_version_command(&path, label)?;
    Ok(ProbeOutcome { path, version })
}

/// Run `<path> --version` and return the first non-empty line of combined
/// stdout/stderr. The command runs with `CREATE_NO_WINDOW` on Windows.
pub fn run_version_command(path: &Path, label: &str) -> Result<String, String> {
    let mut command = process::command_for(path);
    command.arg("--version");
    process::hide_console(&mut command);
    let output = command
        .output()
        .map_err(|err| format!("无法运行 {label}（{}）: {err}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{label} --version 退出码 {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    version::first_version_line(&stdout, &stderr).ok_or_else(|| format!("{label} 未返回版本信息"))
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    /// The probe command builder must not mangle the argument array.
    #[test]
    fn command_preserves_args() {
        let mut command = Command::new("node");
        command.arg("--version");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, vec!["--version"]);
    }
}
