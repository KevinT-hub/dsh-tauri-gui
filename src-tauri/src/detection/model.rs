//! Detection domain types: what the shell probes for, what each probe
//! reports, and the validated `CommandSpec` handed to the engine.
//!
//! This module is pure data — no Tauri, no UI, no process spawning.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The four external dependencies the desktop shell requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyId {
    Node,
    Npm,
    Pnpm,
    Dsh,
}

impl DependencyId {
    pub fn label(self) -> &'static str {
        match self {
            DependencyId::Node => "Node.js",
            DependencyId::Npm => "npm",
            DependencyId::Pnpm => "pnpm",
            DependencyId::Dsh => "dsh",
        }
    }
}

/// Outcome of one dependency probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    /// Probe is running.
    Checking,
    /// Found on PATH and satisfies the version requirement.
    Passed,
    /// Not found on PATH.
    Missing,
    /// Found but the version does not satisfy the requirement.
    Unsupported,
    /// Probe could not be completed (no definitive answer).
    Unknown,
}

/// One row of the setup dependency checklist, serialized verbatim to the
/// frontend (`setup_state`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyInfo {
    pub id: DependencyId,
    pub status: CheckStatus,
    /// Absolute path of the resolved executable, when found.
    pub path: Option<String>,
    /// Reported `--version` output, when available.
    pub version: Option<String>,
    /// Human-readable failure reason, when not passed.
    pub error: Option<String>,
    /// Short installation hint for the UI (see `detection/installer.rs`).
    pub install_hint: Option<String>,
}

impl DependencyInfo {
    pub fn checking(id: DependencyId) -> Self {
        Self {
            id,
            status: CheckStatus::Checking,
            path: None,
            version: None,
            error: None,
            install_hint: None,
        }
    }
}

/// A validated external toolchain, produced by `detection::aggregate` after
/// every gate passed. The engine starts the official `dsh` CLI with this
/// spec and knows nothing about bundled runtimes or archives.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// Absolute path of the `dsh` executable (`.cmd`/`.bat` wrapper on
    /// Windows when installed through npm).
    pub dsh_bin: PathBuf,
    /// Absolute path of the `node` executable, if found. Its directory is
    /// prepended to `PATH` so dsh child processes resolve the same node.
    pub node_bin: Option<PathBuf>,
    /// Absolute path of `npm` or `pnpm`, if found, for PATH propagation.
    pub package_manager_bin: Option<PathBuf>,
    /// Version reported by `dsh --version`.
    pub dsh_version: String,
    /// Node version reported by `node --version`.
    pub node_version: Option<String>,
}
