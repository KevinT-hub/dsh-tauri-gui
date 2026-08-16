/// Best-effort redaction of common secret shapes before a line enters the
/// log buffer, the log file or the frontend event stream.
///
/// This is a second line of defence, not a guarantee: matching is shape
/// based and deliberately conservative, so unusual encodings can still leak.
pub fn redact(line: &str) -> String {
    let line = redact_url_userinfo(line);
    const MARKERS: &[&str] = &[
        "sk-",
        "bearer ",
        "authorization:",
        "x-api-key",
        "api-key",
        "api_key",
        "apikey",
        "api key",
        "password",
        "passwd",
        "token",
        "secret",
    ];

    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize;
    let mut cursor = 0usize;
    let mut changed = false;

    while cursor < lower.len() {
        let mut found: Option<(usize, usize)> = None; // (marker_len, abs_pos)
        for marker in MARKERS {
            if let Some(rel) = lower[cursor..].find(marker) {
                let abs = cursor + rel;
                if found.is_none_or(|(_, pos)| abs < pos) {
                    found = Some((marker.len(), abs));
                }
            }
        }
        let Some((marker_len, pos)) = found else {
            break;
        };

        let after = pos + marker_len;
        let mut value_start = after;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            value_start += 1;
        }

        let has_separator = bytes
            .get(value_start)
            .is_some_and(|b| *b == b':' || *b == b'=');
        let marker_lower = &lower[pos..after];

        if has_separator {
            value_start += 1;
            while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
                value_start += 1;
            }
            // Authorization headers carry "Bearer <token>": redact to the end
            // of the line so the token itself never survives.
            let end = if marker_lower.contains("authorization") {
                lower.len()
            } else {
                let mut end = value_start;
                while end < bytes.len()
                    && !matches!(
                        bytes[end],
                        b' ' | b'\t' | b'\r' | b'\n' | b',' | b'"' | b'\'' | b']' | b'}' | b')'
                    )
                {
                    end += 1;
                }
                end
            };
            if end > value_start {
                out.push_str(&line[last..pos]);
                out.push_str("***");
                changed = true;
                last = end;
                cursor = end;
                continue;
            }
        }

        // `sk-...` API-key tokens are redacted even without a `key:` prefix.
        if marker_lower == "sk-" {
            let mut end = after;
            while end < bytes.len()
                && !matches!(
                    bytes[end],
                    b' ' | b'\t' | b'\r' | b'\n' | b',' | b'"' | b'\'' | b']' | b'}' | b')'
                )
            {
                end += 1;
            }
            if end > after {
                out.push_str(&line[last..pos]);
                out.push_str("***");
                changed = true;
                last = end;
                cursor = end;
                continue;
            }
        }

        // `bearer <token>` without a preceding key.
        if marker_lower == "bearer " {
            let mut end = value_start;
            while end < bytes.len()
                && !matches!(
                    bytes[end],
                    b' ' | b'\t' | b'\r' | b'\n' | b',' | b'"' | b'\'' | b']' | b'}' | b')'
                )
            {
                end += 1;
            }
            if end > value_start {
                out.push_str(&line[last..pos]);
                out.push_str("***");
                changed = true;
                last = end;
                cursor = end;
                continue;
            }
        }

        cursor = after;
    }

    out.push_str(&line[last..]);
    if changed {
        out
    } else {
        line.to_string()
    }
}

/// Redact credentials embedded in URLs (`scheme://user:pass@host/...`).
fn redact_url_userinfo(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize;
    let mut cursor = 0usize;
    while cursor + 2 < bytes.len() {
        if bytes[cursor..cursor + 3] == *b"://" {
            let authority_start = cursor + 3;
            let mut end = authority_start;
            while end < bytes.len()
                && !matches!(bytes[end], b'/' | b'@' | b' ' | b'\t' | b'\r' | b'\n')
            {
                end += 1;
            }
            if end < bytes.len() && bytes[end] == b'@' && end > authority_start {
                out.push_str(&line[last..authority_start]);
                out.push_str("***@");
                last = end + 1;
                cursor = end + 1;
                continue;
            }
        }
        cursor += 1;
    }
    out.push_str(&line[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn redacts_authorization_header() {
        let out = redact("Authorization: Bearer sk-abcdef123456");
        assert!(out.contains("***"));
        assert!(!out.to_lowercase().contains("sk-abcdef123456"));
    }

    #[test]
    fn redacts_sk_token_without_key() {
        let out = redact("using key sk-abc123 now");
        assert!(out.contains("***"));
        assert!(!out.contains("sk-abc123"));
    }

    #[test]
    fn redacts_url_userinfo() {
        let out = redact("registry https://user:supersecret@npm.example.com/x");
        assert!(out.contains("***@"));
        assert!(!out.contains("supersecret"));
        assert!(out.contains("npm.example.com"));
    }

    #[test]
    fn redacts_key_value_forms() {
        let out = redact("api key=abcd1234 and token: wxyz5678");
        assert!(out.contains("***"));
        assert!(!out.contains("abcd1234"));
        assert!(!out.contains("wxyz5678"));
    }

    #[test]
    fn leaves_normal_lines_untouched() {
        let line = "engine started, node version is 22.19.0";
        assert_eq!(redact(line), line);
    }

    #[test]
    fn leaves_plain_token_word_alone() {
        let line = "token meter consumed 123 tokens";
        assert_eq!(redact(line), line);
    }
}
