//! Version and availability requirements for the external toolchain.
//!
//! The default gate is:
//! - Node.js must exist and satisfy `^22.19.0 || >=24` (official dsh engines).
//! - npm or pnpm must be available (at least one).
//! - dsh must exist on PATH.
//!
//! The UI still shows npm and pnpm as separate rows. If the product ever
//! requires all four, only `gate_passed` needs to change — the architecture
//! stays the same.

use super::model::{CheckStatus, DependencyId, DependencyInfo};

/// Human-readable requirement for the setup screen.
pub const NODE_REQUIREMENT: &str = "^22.19.0 || >=24";
pub const PACKAGE_MANAGER_REQUIREMENT: &str = "npm 或 pnpm（至少一个）";
pub const DSH_REQUIREMENT: &str = "官方 @deepseek-ai/dsh CLI";

/// The checklist is green only when every mandatory gate is green.
pub fn gate_passed(items: &[DependencyInfo]) -> bool {
    node_ok(items) && package_manager_ok(items) && dsh_ok(items)
}

/// The list of dependencies whose gate failed (missing / unsupported /
/// unknown), in a stable order, for the UI to offer install help.
pub fn failed_items(items: &[DependencyInfo]) -> Vec<&DependencyInfo> {
    let mut failed: Vec<&DependencyInfo> = items
        .iter()
        .filter(|item| !matches!(item.status, CheckStatus::Passed | CheckStatus::Checking))
        .collect();
    failed.sort_by_key(|item| item.id as u8);
    failed
}

fn node_ok(items: &[DependencyInfo]) -> bool {
    items
        .iter()
        .find(|item| item.id == DependencyId::Node)
        .is_some_and(|item| item.status == CheckStatus::Passed)
}

fn package_manager_ok(items: &[DependencyInfo]) -> bool {
    items.iter().any(|item| {
        matches!(item.id, DependencyId::Npm | DependencyId::Pnpm)
            && item.status == CheckStatus::Passed
    })
}

fn dsh_ok(items: &[DependencyInfo]) -> bool {
    items
        .iter()
        .find(|item| item.id == DependencyId::Dsh)
        .is_some_and(|item| item.status == CheckStatus::Passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: DependencyId, status: CheckStatus) -> DependencyInfo {
        DependencyInfo {
            id,
            status,
            path: None,
            version: None,
            error: None,
            install_hint: None,
        }
    }

    #[test]
    fn gate_passes_with_node_npm_dsh() {
        let items = vec![
            item(DependencyId::Node, CheckStatus::Passed),
            item(DependencyId::Npm, CheckStatus::Passed),
            item(DependencyId::Pnpm, CheckStatus::Missing),
            item(DependencyId::Dsh, CheckStatus::Passed),
        ];
        assert!(gate_passed(&items));
    }

    #[test]
    fn gate_passes_with_node_pnpm_dsh() {
        let items = vec![
            item(DependencyId::Node, CheckStatus::Passed),
            item(DependencyId::Npm, CheckStatus::Missing),
            item(DependencyId::Pnpm, CheckStatus::Passed),
            item(DependencyId::Dsh, CheckStatus::Passed),
        ];
        assert!(gate_passed(&items));
    }

    #[test]
    fn gate_fails_without_node() {
        let items = vec![
            item(DependencyId::Node, CheckStatus::Missing),
            item(DependencyId::Npm, CheckStatus::Passed),
            item(DependencyId::Dsh, CheckStatus::Passed),
        ];
        assert!(!gate_passed(&items));
    }

    #[test]
    fn gate_fails_without_any_package_manager() {
        let items = vec![
            item(DependencyId::Node, CheckStatus::Passed),
            item(DependencyId::Npm, CheckStatus::Missing),
            item(DependencyId::Pnpm, CheckStatus::Missing),
            item(DependencyId::Dsh, CheckStatus::Passed),
        ];
        assert!(!gate_passed(&items));
    }

    #[test]
    fn gate_fails_without_dsh() {
        let items = vec![
            item(DependencyId::Node, CheckStatus::Passed),
            item(DependencyId::Npm, CheckStatus::Passed),
            item(DependencyId::Dsh, CheckStatus::Missing),
        ];
        assert!(!gate_passed(&items));
    }

    #[test]
    fn unsupported_node_version_fails_gate() {
        let items = vec![
            item(DependencyId::Node, CheckStatus::Unsupported),
            item(DependencyId::Npm, CheckStatus::Passed),
            item(DependencyId::Dsh, CheckStatus::Passed),
        ];
        assert!(!gate_passed(&items));
    }

    #[test]
    fn failed_items_lists_only_failed_in_stable_order() {
        let items = vec![
            item(DependencyId::Node, CheckStatus::Passed),
            item(DependencyId::Npm, CheckStatus::Missing),
            item(DependencyId::Pnpm, CheckStatus::Passed),
            item(DependencyId::Dsh, CheckStatus::Unsupported),
        ];
        let failed = failed_items(&items);
        assert_eq!(failed.len(), 2);
        assert_eq!(failed[0].id, DependencyId::Npm);
        assert_eq!(failed[1].id, DependencyId::Dsh);
    }
}
