//! `dsh web` stdout protocol parsing: the ready marker that tells the shell
//! the Web UI is reachable at a specific localhost origin.

/// Parse a ready marker line like `dsh web: http://127.0.0.1:3080/` into a
/// navigable URL. Only loopback HTTP URLs with a non-zero port are accepted.
pub fn parse_web_url(line: &str) -> Option<String> {
    const MARKER: &str = "dsh web: ";
    let start = line.find(MARKER)? + MARKER.len();
    let rest = &line[start..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let url = &rest[..end];
    let parsed = url::Url::parse(url).ok()?;
    let port = parsed.port()?;
    if parsed.scheme() == "http" && parsed.host_str() == Some("127.0.0.1") && port != 0 {
        Some(parsed.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::parse_web_url;

    #[test]
    fn parses_ready_marker_line() {
        let line = "dsh web: http://127.0.0.1:3080/";
        assert_eq!(
            parse_web_url(line).as_deref(),
            Some("http://127.0.0.1:3080/")
        );
    }

    #[test]
    fn parses_marker_embedded_in_longer_output() {
        let line = "[12:00:00] info dsh web: http://127.0.0.1:51234/ (pid 1234)";
        assert_eq!(
            parse_web_url(line).as_deref(),
            Some("http://127.0.0.1:51234/")
        );
    }

    #[test]
    fn rejects_non_loopback_and_http() {
        assert_eq!(parse_web_url("dsh web: https://127.0.0.1:3080/"), None);
        assert_eq!(parse_web_url("dsh web: http://localhost:3080/"), None);
        assert_eq!(parse_web_url("dsh web: http://127.0.0.1:0/"), None);
        assert_eq!(parse_web_url("dsh web: http://192.168.1.1:3080/"), None);
        assert_eq!(parse_web_url("no marker here"), None);
    }
}
