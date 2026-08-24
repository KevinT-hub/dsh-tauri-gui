//! Install help for missing dependencies. Nothing here executes commands:
//! it only answers "what should the user do" for the setup screen. Actual
//! installation actions are user-confirmed command invocations in
//! `commands/setup.rs`.

use super::model::DependencyId;
use crate::detection::sources::SourcePolicy;
use serde::Serialize;

/// What the UI should offer for a failed dependency.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallHelp {
    pub dependency: DependencyId,
    pub title: &'static str,
    pub message: String,
    /// Official page to open in the system browser.
    pub official_url: Option<&'static str>,
    /// Command to run (argument array, user-confirmed) — `None` when the
    /// user should install manually from the official page.
    pub command: Option<Vec<String>>,
}

/// Official install targets per platform.
#[cfg(target_os = "windows")]
const NODE_OFFICIAL_URL: &str = "https://nodejs.org/en/download";
#[cfg(target_os = "macos")]
const NODE_OFFICIAL_URL: &str = "https://nodejs.org/en/download";
#[cfg(target_os = "linux")]
const NODE_OFFICIAL_URL: &str = "https://nodejs.org/en/download/package-manager";

pub fn help_for(dependency: DependencyId, policy: &SourcePolicy) -> InstallHelp {
    match dependency {
        DependencyId::Node => InstallHelp {
            dependency,
            title: "安装 Node.js",
            message: format!(
                "请安装 Node.js 22.19+ 或 24+。国内用户可使用镜像：{}",
                policy.node_mirror
            ),
            official_url: Some(NODE_OFFICIAL_URL),
            // Opening the official download page is the safe, cross-platform
            // default; we never auto-run a package manager silently.
            command: None,
        },
        DependencyId::Npm => InstallHelp {
            dependency,
            title: "安装 npm",
            message: "npm 随 Node.js 官方发行版一同安装。安装 Node.js 后请重新检测。".to_string(),
            official_url: Some(NODE_OFFICIAL_URL),
            command: None,
        },
        DependencyId::Pnpm => InstallHelp {
            dependency,
            title: "安装 pnpm",
            message: "pnpm 可通过 corepack（随 Node.js 提供）或官方安装脚本安装。".to_string(),
            official_url: Some("https://pnpm.io/installation"),
            command: Some(vec!["corepack".to_string(), "enable".to_string()]),
        },
        DependencyId::Dsh => InstallHelp {
            dependency,
            title: "安装 dsh",
            message: format!(
                "请安装官方 CLI 包 @deepseek-ai/dsh（registry: {}）",
                policy.npm_registry
            ),
            official_url: Some("https://www.npmjs.com/package/@deepseek-ai/dsh"),
            command: Some(vec![
                "npm".to_string(),
                "install".to_string(),
                "-g".to_string(),
                "@deepseek-ai/dsh".to_string(),
                "--registry".to_string(),
                policy.npm_registry.to_string(),
            ]),
        },
    }
}

/// The default registry when no geo result exists yet — mirrors the
/// `REGISTRY_OFFICIAL` fallback so install help is always available.
#[cfg(test)]
pub fn default_policy() -> SourcePolicy {
    crate::detection::sources::resolve_sources(crate::geo::model::RegionCode::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsh_help_uses_official_package() {
        let policy = default_policy();
        let help = help_for(DependencyId::Dsh, &policy);
        assert!(help.message.contains("@deepseek-ai/dsh"));
        assert!(help.command.is_some());
    }

    #[test]
    fn node_help_has_official_url() {
        let policy = default_policy();
        let help = help_for(DependencyId::Node, &policy);
        assert!(help.official_url.is_some());
    }
}
