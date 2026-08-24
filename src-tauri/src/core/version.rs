//! Version parsing and requirement checks for the external toolchain.
//!
//! The official `@deepseek-ai/dsh` package requires Node `^22.19.0 || >=24`.
//! npm/pnpm/dsh only need to exist and report a version. All parsing here is
//! pure and unit-testable without touching the filesystem or processes.

/// Parse the `node --version` output (`v22.19.0\n`) into a bare version
/// string (`22.19.0`). Returns `None` for empty or malformed output.
pub fn parse_node_version_output(output: &str) -> Option<String> {
    let version = output.trim().trim_start_matches('v');
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

/// The first non-empty line of a `--version` invocation. Some tools print
/// diagnostics to stdout and the version to stderr (or vice versa), so both
/// streams are candidates.
pub fn first_version_line(stdout: &str, stderr: &str) -> Option<String> {
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// Parse `22.19.0` / `v22.19.0` / `22.19.0-alpha.1` into comparable parts.
fn parse_major_minor(version: &str) -> Option<(u32, u32)> {
    let version = version.trim().trim_start_matches('v');
    let base = version.split('-').next().unwrap_or(version);
    let mut parts = base.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// Official dsh requirement: `^22.19.0 || >=24`.
pub fn node_supported(version: &str) -> bool {
    let Some((major, minor)) = parse_major_minor(version) else {
        return false;
    };
    match major {
        22 => minor >= 19,
        major => major >= 24,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_node_version_output() {
        assert_eq!(
            parse_node_version_output("v22.19.0\n").as_deref(),
            Some("22.19.0")
        );
        assert_eq!(
            parse_node_version_output("v24.1.2").as_deref(),
            Some("24.1.2")
        );
        assert_eq!(
            parse_node_version_output("  v20.11.1  ").as_deref(),
            Some("20.11.1")
        );
        assert_eq!(parse_node_version_output(""), None);
        assert_eq!(parse_node_version_output("v\n"), None);
    }

    #[test]
    fn picks_first_nonempty_line_across_streams() {
        assert_eq!(
            first_version_line("v22.19.0\n", "").as_deref(),
            Some("v22.19.0")
        );
        assert_eq!(
            first_version_line("", "v22.19.0\n").as_deref(),
            Some("v22.19.0")
        );
        assert_eq!(first_version_line("\n\n", "\n").as_deref(), None);
    }

    #[test]
    fn node_support_window_matches_official_engines() {
        assert!(node_supported("22.19.0"));
        assert!(node_supported("22.99.0"));
        assert!(node_supported("24.0.0"));
        assert!(node_supported("24.1.2"));
        assert!(node_supported("v22.19.0"));
        assert!(!node_supported("22.18.9"));
        assert!(!node_supported("20.11.1"));
        assert!(!node_supported("23.0.0"));
        assert!(!node_supported(""));
        assert!(!node_supported("not-a-version"));
    }
}
