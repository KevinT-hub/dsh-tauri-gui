//! Port probing and WebView navigation rules.
//!
//! - `is_dsh_web_alive` detects an official `dsh web` instance already
//!   serving the configured port so the shell can attach instead of spawning
//!   a second engine.
//! - Navigation is whitelisted to the exact origin recorded in
//!   `ready_url`, so a stray local service or a polluted engine log line can
//!   never hijack the WebView.

use crate::app::state::AppState;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// Probe whether an official `dsh web` instance is already serving the
/// configured port. If it is, the desktop app connects to it instead of
/// spawning a second engine, so config/sessions stay in one service area.
pub fn is_dsh_web_alive(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(800)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1200)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));
    let request = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut head = String::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                head.push_str(&String::from_utf8_lossy(&buf[..n]));
                if head.contains("__DSH_BOOT__") {
                    return true;
                }
            }
        }
    }
    head.starts_with("HTTP/") && head.contains("__DSH_BOOT__")
}

/// Only the origin recorded in `ready_url` may be loaded by the main
/// WebView. Everything else is refused, so a stray local service or a
/// polluted engine log line cannot hijack the window.
pub fn is_allowed_web_url(state: &AppState, url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "http" || parsed.host_str() != Some("127.0.0.1") {
        return false;
    }
    let Some(port) = parsed.port() else {
        return false;
    };
    let ready = state.ready_url.lock().unwrap().clone();
    let Some(ready) = ready else {
        return false;
    };
    url::Url::parse(&ready)
        .map(|expected| {
            expected.scheme() == "http"
                && expected.host_str() == Some("127.0.0.1")
                && expected.port() == Some(port)
        })
        .unwrap_or(false)
}

/// The shell's own pages (packaged assets or the Vite dev server).
pub fn is_shell_url(url: &str) -> bool {
    url.starts_with("tauri://localhost")
        || url.starts_with("https://tauri.localhost")
        || url.starts_with("http://localhost:1420")
}

/// Navigate the main WebView to a whitelisted URL. The window is revealed by
/// `on_page_load` after the Web UI has painted, so there is no black flash.
pub fn navigate(app: &AppHandle, url: &str) {
    let url = url.to_string();
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
        if !is_allowed_web_url(&state, &url) {
            state.logger.warn(&format!(
                "refusing navigation to non-whitelisted url: {url}"
            ));
            return;
        }
        if let Some(window) = app.get_webview_window("main") {
            if let Ok(parsed) = url::Url::parse(&url) {
                let _ = window.navigate(parsed);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::is_shell_url;

    #[test]
    fn shell_urls_are_recognized() {
        assert!(is_shell_url("tauri://localhost"));
        assert!(is_shell_url("https://tauri.localhost"));
        assert!(is_shell_url("http://localhost:1420/"));
        assert!(!is_shell_url("http://127.0.0.1:3080/"));
        assert!(!is_shell_url("https://example.com"));
    }
}
