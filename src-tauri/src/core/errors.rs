//! Domain-error helpers. The shell's commands surface errors to the
//! frontend as `String`; this module centralizes the common conversions so
//! services never format user-facing text ad hoc.

use std::io;

/// Convert an `io::Error` into a command-facing message with context.
#[allow(dead_code)] // 命令层错误统一转换的扩展边界（当前命令直接返回 String）
pub fn io_error(context: &str, err: io::Error) -> String {
    format!("{context}: {err}")
}

/// Convert an `io::Result<T>` into `Result<T, String>` with context.
#[allow(dead_code)]
pub fn map_io<T>(result: io::Result<T>, context: &str) -> Result<T, String> {
    result.map_err(|err| io_error(context, err))
}

/// Convert a `serde_json::Error` into a command-facing message.
#[allow(dead_code)]
pub fn json_error(context: &str, err: serde_json::Error) -> String {
    format!("{context}: {err}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_includes_context() {
        let message = io_error("无法写入配置", io::Error::other("denied"));
        assert!(message.contains("无法写入配置"));
        assert!(message.contains("denied"));
    }

    #[test]
    fn map_io_maps_error() {
        let result: io::Result<()> = Err(io::Error::other("missing"));
        assert!(map_io(result, "读取文件").is_err());
    }
}
