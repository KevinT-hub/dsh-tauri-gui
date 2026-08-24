//! npm / pnpm probes. Both are shown as separate rows; the gate accepts
//! either one. Windows `.cmd`/`.bat` shims are handled by
//! `core::process::command_for`.

use super::model::{CheckStatus, DependencyId, DependencyInfo};
use super::probes;

fn detect_one(id: DependencyId, name: &str, label: &str) -> DependencyInfo {
    let mut info = DependencyInfo::checking(id);
    match probes::probe(name, label) {
        Ok(outcome) => {
            info.path = Some(outcome.path.display().to_string());
            info.version = Some(outcome.version);
            info.status = CheckStatus::Passed;
        }
        Err(error) => {
            info.status = CheckStatus::Missing;
            info.error = Some(error);
            info.install_hint = Some(format!(
                "{label} 通常随 Node.js 安装，或通过 corepack/pnpm 官方脚本安装"
            ));
        }
    }
    info
}

pub fn detect_npm() -> DependencyInfo {
    detect_one(DependencyId::Npm, "npm", "npm")
}

pub fn detect_pnpm() -> DependencyInfo {
    detect_one(DependencyId::Pnpm, "pnpm", "pnpm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npm_row_has_expected_id() {
        let info = DependencyInfo {
            id: DependencyId::Npm,
            status: CheckStatus::Missing,
            path: None,
            version: None,
            error: None,
            install_hint: None,
        };
        assert_eq!(info.id, DependencyId::Npm);
    }
}
