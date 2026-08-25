//! Application orchestration layer: state, config, status and lifecycle.
//! Commands and services talk to `AppState` through these modules.

pub mod bootstrap;
pub mod config;
pub mod engine_session;
pub mod events;
pub mod lifecycle;
pub mod state;
pub mod status;
pub mod toolchain_cache;

pub use state::AppState;
pub use status::{
    clear_update_notice, emit_log, set_status, set_update_notice, ShellStatus, UpdateNotice,
};
