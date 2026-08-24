//! Gate aggregation tests: npm-or-pnpm, node and dsh requirements.

use crate::detection::model::{CheckStatus, DependencyId, DependencyInfo};
use crate::detection::requirement;

fn row(id: DependencyId, status: CheckStatus) -> DependencyInfo {
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
fn aggregate_accepts_npm_or_pnpm() {
    let with_npm = vec![
        row(DependencyId::Node, CheckStatus::Passed),
        row(DependencyId::Npm, CheckStatus::Passed),
        row(DependencyId::Pnpm, CheckStatus::Missing),
        row(DependencyId::Dsh, CheckStatus::Passed),
    ];
    assert!(requirement::gate_passed(&with_npm));

    let with_pnpm = vec![
        row(DependencyId::Node, CheckStatus::Passed),
        row(DependencyId::Npm, CheckStatus::Missing),
        row(DependencyId::Pnpm, CheckStatus::Passed),
        row(DependencyId::Dsh, CheckStatus::Passed),
    ];
    assert!(requirement::gate_passed(&with_pnpm));
}

#[test]
fn aggregate_rejects_without_package_manager() {
    let items = vec![
        row(DependencyId::Node, CheckStatus::Passed),
        row(DependencyId::Npm, CheckStatus::Missing),
        row(DependencyId::Pnpm, CheckStatus::Missing),
        row(DependencyId::Dsh, CheckStatus::Passed),
    ];
    assert!(!requirement::gate_passed(&items));
}

#[test]
fn aggregate_rejects_unsupported_node() {
    let items = vec![
        row(DependencyId::Node, CheckStatus::Unsupported),
        row(DependencyId::Npm, CheckStatus::Passed),
        row(DependencyId::Dsh, CheckStatus::Passed),
    ];
    assert!(!requirement::gate_passed(&items));
}
