//! dsh CLI probe. Only the official `@deepseek-ai/dsh` package is
//! supported; the shell never bundles or copies the plugin/session data.

use super::model::{CheckStatus, DependencyId, DependencyInfo};
use super::probes;

/// The official package name shown in install help and logs.
pub const DSH_PACKAGE: &str = "@deepseek-ai/dsh";

/// Probe `dsh` on PATH and map the outcome onto a checklist row.
pub fn detect() -> DependencyInfo {
    let mut info = DependencyInfo::checking(DependencyId::Dsh);
    match probes::probe("dsh", "dsh") {
        Ok(outcome) => {
            info.path = Some(outcome.path.display().to_string());
            info.version = Some(outcome.version);
            info.status = CheckStatus::Passed;
        }
        Err(error) => {
            info.status = CheckStatus::Missing;
            info.error = Some(error);
            info.install_hint = Some(format!(
                "请通过 npm/pnpm 安装官方包：npm install -g {DSH_PACKAGE}"
            ));
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_is_official() {
        assert_eq!(DSH_PACKAGE, "@deepseek-ai/dsh");
    }
}
