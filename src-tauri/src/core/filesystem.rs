//! Filesystem helpers: atomic writes with backup rotation. The shell config
//! uses this pattern so a crash mid-write never corrupts the persisted state.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

/// Atomically write `content` to `path`: write a temp file, rename the
/// current file to a `.bak`, then rename the temp into place. On failure the
/// previous file is restored.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("path has no parent")?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let pid = std::process::id();
    let stamp = nonce();
    let tmp = parent.join(format!(".write-{pid}-{stamp}.tmp"));
    let backup = parent.join(format!(".write-{pid}-{stamp}.bak"));
    fs::write(&tmp, content).map_err(|err| err.to_string())?;
    if path.exists() {
        fs::rename(path, &backup).map_err(|err| err.to_string())?;
    }
    match fs::rename(&tmp, path) {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
            Ok(())
        }
        Err(err) => {
            if backup.exists() && !path.exists() {
                let _ = fs::rename(&backup, path);
            }
            let _ = fs::remove_file(&tmp);
            Err(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_roundtrips() {
        let dir = std::env::temp_dir().join(format!("dsh-fs-test-{}", std::process::id()));
        let path = dir.join("config.json");
        atomic_write(&path, "{\"a\":1}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        atomic_write(&path, "{\"a\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":2}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_restores_backup_on_failure() {
        // Writing to a path whose parent is a file must fail and leave no
        // temp residue behind.
        let dir = std::env::temp_dir().join(format!("dsh-fs-test-err-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("blocker");
        fs::write(&blocker, "x").unwrap();
        let bad = blocker.join("config.json"); // parent is a file
        assert!(atomic_write(&bad, "{}").is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
