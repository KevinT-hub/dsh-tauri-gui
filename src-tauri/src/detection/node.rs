//! Node.js probe: locate `node` on PATH, read `node --version` and enforce
//! the official dsh engine requirement `^22.19.0 || >=24`.

use super::model::{CheckStatus, DependencyId, DependencyInfo};
use super::probes;
use crate::core::version;

/// Run the Node.js probe and map the outcome onto a checklist row.
pub fn detect() -> DependencyInfo {
    let mut info = DependencyInfo::checking(DependencyId::Node);
    match probes::probe("node", "Node.js") {
        Ok(outcome) => {
            let version = match version::parse_node_version_output(&outcome.version) {
                Some(version) => version,
                None => {
                    info.status = CheckStatus::Unknown;
                    info.error = Some(format!("无法解析 Node.js 版本: {}", outcome.version));
                    return info;
                }
            };
            info.path = Some(outcome.path.display().to_string());
            info.version = Some(version.clone());
            if version::node_supported(&version) {
                info.status = CheckStatus::Passed;
            } else {
                info.status = CheckStatus::Unsupported;
                info.error = Some(format!(
                    "Node.js {version} 不满足 dsh 要求（{}）",
                    super::requirement::NODE_REQUIREMENT
                ));
                info.install_hint = Some("请安装 Node.js 22.19+ 或 24+ 版本".to_string());
            }
        }
        Err(error) => {
            info.status = CheckStatus::Missing;
            info.error = Some(error);
            info.install_hint = Some("请安装 Node.js 官方发行版（LTS 22.x 或 24.x）".to_string());
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_row_has_expected_id() {
        let info = DependencyInfo {
            id: DependencyId::Node,
            status: CheckStatus::Missing,
            path: None,
            version: None,
            error: Some("not found".into()),
            install_hint: Some("install node".into()),
        };
        assert_eq!(info.status, CheckStatus::Missing);
        assert_eq!(info.id, DependencyId::Node);
    }
}
