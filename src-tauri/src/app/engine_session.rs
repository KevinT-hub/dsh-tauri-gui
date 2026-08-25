//! Desktop-shell ownership record for a warm dsh process.
//!
//! This metadata belongs to the shell home, not `$DSH_HOME`. The engine only
//! receives the recorded port and never owns this persistence format.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SESSION_FILE: &str = "engine-session.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineSession {
    pub pid: u32,
    pub port: u16,
}

fn path(home: &Path) -> PathBuf {
    home.join(SESSION_FILE)
}

pub fn load(home: &Path) -> Option<EngineSession> {
    let text = std::fs::read_to_string(path(home)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(home: &Path, session: &EngineSession) -> Result<(), String> {
    let text = serde_json::to_string(session).map_err(|err| err.to_string())?;
    crate::core::filesystem::atomic_write(&path(home), &text)
}

pub fn clear(home: &Path) {
    let _ = std::fs::remove_file(path(home));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn session_round_trip_is_recoverable() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home = std::env::temp_dir().join(format!("dsh-engine-session-{nonce}"));
        std::fs::create_dir_all(&home).unwrap();
        let expected = EngineSession {
            pid: 42,
            port: 3080,
        };

        save(&home, &expected).unwrap();
        assert_eq!(load(&home).as_ref(), Some(&expected));
        clear(&home);
        assert!(load(&home).is_none());
        std::fs::remove_dir_all(home).unwrap();
    }
}
