//! Platform differences (Windows/macOS/Linux) that do not belong in domain
//! modules: path separators, open-command mapping and similar.

/// Path-list separator for `PATH`-style strings.
#[allow(dead_code)]
pub fn path_separator() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

/// Native command that reveals a path in the OS file manager.
pub fn reveal_command() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "explorer.exe"
    }
    #[cfg(target_os = "macos")]
    {
        "open"
    }
    #[cfg(target_os = "linux")]
    {
        "xdg-open"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_separator_matches_platform() {
        assert_eq!(path_separator(), if cfg!(windows) { ";" } else { ":" });
    }
}
