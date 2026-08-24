//! Shared HTTP client for geo, updater and public-metadata requests.
//!
//! All outbound requests from the shell must go through this module so
//! timeouts, the User-Agent and the no-credential policy stay uniform.
//! Mirrors are untrusted download proxies: they only receive public URLs
//! and are never sent tokens, cookies or forwarded authorization headers.

use std::time::Duration;

/// Build an HTTP agent with sane timeouts. A global timeout is only applied
/// to small metadata requests; downloads set their own stall detection.
pub fn http_agent(global_timeout: Option<Duration>) -> ureq::Agent {
    let mut builder = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(10)));
    if let Some(timeout) = global_timeout {
        builder = builder.timeout_global(Some(timeout));
    }
    ureq::Agent::new_with_config(builder.build())
}

/// Canonical User-Agent used by every shell-initiated request.
pub const USER_AGENT: &str = "dsh-tauri-gui";
