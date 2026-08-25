//! Official dsh engine service.
//!
//! The engine only ever receives a validated external `CommandSpec` (from
//! `detection::aggregate::command_spec`) and hands it to the official
//! `dsh web` CLI. There is no bundled-runtime, archive, extraction or
//! runtime-version-list knowledge anywhere in this module.

pub mod command;
pub mod environment;
pub mod lifecycle;
pub mod process;
pub mod web;
pub mod workspace;

pub use lifecycle::{
    connect_existing_or_spawn, detach_engine, navigate_when_ready, open_web_ui,
    open_web_ui_browser, restart_engine, stop_engine,
};
pub use web::{is_allowed_web_url, is_shell_url, navigate_to_shell};
pub use workspace::workspace_dir;
