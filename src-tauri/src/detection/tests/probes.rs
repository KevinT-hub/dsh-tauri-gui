//! Probe-related pure tests: Windows command extension candidates.

use crate::core::process;

#[test]
fn windows_command_candidates_prefer_exe() {
    // The candidate list is platform-conditional; on Windows the shim
    // extensions must precede the bare name, on Unix there is one entry.
    let candidates = process::command_candidates("dsh");
    if cfg!(windows) {
        assert_eq!(candidates, vec!["dsh.exe", "dsh.cmd", "dsh.bat", "dsh"]);
    } else {
        assert_eq!(candidates, vec!["dsh"]);
    }
}

#[test]
fn version_line_extraction_handles_stderr() {
    use crate::core::version::first_version_line;
    assert_eq!(
        first_version_line("", "v22.19.0\n").as_deref(),
        Some("v22.19.0")
    );
    assert_eq!(first_version_line("", ""), None);
}
