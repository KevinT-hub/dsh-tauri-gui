//! Application orchestration layer: state, config, status and lifecycle.
//! Commands and services talk to `AppState` through these modules.

pub mod config;
pub mod events;
pub mod lifecycle;
pub mod state;
pub mod status;

pub use state::AppState;
pub use status::{emit_log, push_log_tail, set_status, ShellStatus};
