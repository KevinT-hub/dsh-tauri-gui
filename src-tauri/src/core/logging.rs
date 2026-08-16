use crate::core::redact::redact;
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Minimal daily-rotating file logger for the desktop shell and the dsh
/// engine's captured output.
pub struct Logger {
    dir: PathBuf,
}

impl Logger {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn info(&self, message: &str) {
        self.log("app", "INFO", message);
    }

    pub fn warn(&self, message: &str) {
        self.log("app", "WARN", message);
    }

    pub fn error(&self, message: &str) {
        self.log("app", "ERROR", message);
    }

    pub fn log(&self, stream: &str, level: &str, message: &str) {
        let filename = format!("{stream}-{}.log", Local::now().format("%Y-%m-%d"));
        let path = self.dir.join(filename);
        let message = redact(message);
        let line = format!(
            "[{}] [{}] {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            level,
            message
        );
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }

    #[allow(dead_code)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}
