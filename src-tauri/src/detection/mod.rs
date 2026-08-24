//! Environment detection facade. The rest of the shell only talks to this
//! module (or `aggregate::run_all`) — never to individual probes.

pub mod aggregate;
pub mod dsh;
pub mod installer;
pub mod model;
pub mod node;
pub mod package_manager;
pub mod probes;
pub mod requirement;
pub mod session;
pub mod sources;
#[cfg(test)]
pub mod tests;

pub use model::DependencyInfo;
pub use requirement::gate_passed;
pub use sources::{resolve_sources, SourcePolicy};

/// Convenience entry point: run all probes and return the checklist rows.
pub fn detect_all() -> Vec<DependencyInfo> {
    aggregate::run_all()
}
