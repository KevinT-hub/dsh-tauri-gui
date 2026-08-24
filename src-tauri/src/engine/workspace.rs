//! Workspace / working-directory resolution for the engine process.

pub use crate::engine::environment::workspace_dir;

#[cfg(test)]
mod tests {
    use super::workspace_dir;
    use crate::app::config::ShellConfig;

    #[test]
    fn workspace_falls_back_to_home() {
        let config = ShellConfig::default();
        let dir = workspace_dir(&config);
        assert!(dir.is_absolute());
    }
}
