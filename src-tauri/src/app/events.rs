//! Rust → frontend event names. Keeping the constants here avoids string
//! literals scattered across the codebase.

/// Shell phase/message updates (`ShellStatus` payload).
pub const STATUS_EVENT: &str = "shell://status";
/// Live log lines (`{"level","line"}` payload).
pub const LOG_EVENT: &str = "shell://log";
/// Theme state updates (`ThemeState` payload).
pub const THEME_EVENT: &str = "shell://theme";
/// App-update availability (`AppUpdateInfo` payload).
pub const APP_UPDATE_EVENT: &str = "shell://app-update";
/// DeepSeek Harness package-update availability (`DshUpdateInfo` payload).
pub const DSH_UPDATE_EVENT: &str = "shell://dsh-update";
/// Ask the frontend to show and remount the setup checklist.
pub const SETUP_REQUESTED_EVENT: &str = "shell://setup-requested";
/// Download progress events for the app updater overlay.
pub const UPDATER_DOWNLOAD_EVENT: &str = "updater-download-progress";
