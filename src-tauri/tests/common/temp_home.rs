//! Temporary shell-home helper: gives each test an isolated, auto-cleaned
//! directory instead of touching the real user home.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a unique temporary home directory and return its path. The
/// directory is created eagerly so `tempfile`-style cleanup is safe to
/// skip; callers should remove it after the test.
pub fn temp_home() -> PathBuf {
    let base = std::env::temp_dir().join("dsh-tauri-gui-tests");
    let dir = base.join(format!(
        "home-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).expect("create temp home");
    dir
}

/// Remove a temp home (best-effort).
pub fn cleanup(home: &Path) {
    let _ = std::fs::remove_dir_all(home);
}
