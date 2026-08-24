//! Core infrastructure: pure `std` + third-party helpers shared by every
//! domain. Nothing here depends on React, Tauri windows or any feature.

pub mod errors;
pub mod filesystem;
pub mod http;
pub mod logging;
pub mod paths;
pub mod platform;
pub mod process;
pub mod redact;
pub mod version;
