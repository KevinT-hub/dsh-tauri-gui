//! Run the four probes in parallel and aggregate the rows into a single
//! checklist snapshot. Pure domain logic: no UI, no Tauri.

use super::model::{CommandSpec, DependencyId, DependencyInfo};
use super::requirement;
use std::sync::{Arc, Mutex};

/// Run Node/npm/pnpm/dsh probes concurrently (bounded threads, no runtime
/// dependency) and return the four rows in a stable order.
pub fn run_all() -> Vec<DependencyInfo> {
    let rows = Arc::new(Mutex::new(Vec::with_capacity(4)));

    std::thread::scope(|scope| {
        for detect in [
            super::node::detect,
            super::package_manager::detect_npm,
            super::package_manager::detect_pnpm,
            super::dsh::detect,
        ] {
            let rows = Arc::clone(&rows);
            scope.spawn(move || {
                let info = detect();
                let mut guard = rows.lock().unwrap();
                guard.push(info);
            });
        }
    });

    let mut all = Arc::try_unwrap(rows).unwrap().into_inner().unwrap();
    all.sort_by_key(|item| item.id as u8);
    all
}

/// Build the validated `CommandSpec` from a green checklist, or return the
/// first blocking failure reason. The engine only ever receives a spec from
/// this function.
pub fn command_spec(items: &[DependencyInfo]) -> Result<CommandSpec, String> {
    if !requirement::gate_passed(items) {
        let failed = requirement::failed_items(items);
        let reason = failed
            .first()
            .map(|item| {
                item.error
                    .clone()
                    .unwrap_or_else(|| format!("{} 未通过检测", item.id.label()))
            })
            .unwrap_or_else(|| "环境检测未全部通过".to_string());
        return Err(reason);
    }

    let node = items
        .iter()
        .find(|item| item.id == DependencyId::Node)
        .and_then(|item| item.path.as_ref().map(std::path::PathBuf::from));
    let package_manager = items
        .iter()
        .find(|item| matches!(item.id, DependencyId::Npm | DependencyId::Pnpm))
        .and_then(|item| item.path.as_ref().map(std::path::PathBuf::from));
    let dsh = items
        .iter()
        .find(|item| item.id == DependencyId::Dsh)
        .and_then(|item| item.path.as_ref().map(std::path::PathBuf::from))
        .ok_or_else(|| "dsh 未通过检测".to_string())?;
    let dsh_version = items
        .iter()
        .find(|item| item.id == DependencyId::Dsh)
        .and_then(|item| item.version.clone())
        .unwrap_or_default();

    Ok(CommandSpec {
        dsh_bin: dsh,
        node_bin: node,
        package_manager_bin: package_manager,
        dsh_version,
        node_version: items
            .iter()
            .find(|item| item.id == DependencyId::Node)
            .and_then(|item| item.version.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::model::CheckStatus;

    fn passed(id: DependencyId, path: &str, version: &str) -> DependencyInfo {
        DependencyInfo {
            id,
            status: CheckStatus::Passed,
            path: Some(path.to_string()),
            version: Some(version.to_string()),
            error: None,
            install_hint: None,
        }
    }

    #[test]
    fn command_spec_from_green_checklist() {
        let items = vec![
            passed(DependencyId::Node, "C:/node/node.exe", "22.19.0"),
            passed(DependencyId::Npm, "C:/node/npm.cmd", "11.0.0"),
            DependencyInfo {
                id: DependencyId::Pnpm,
                status: CheckStatus::Missing,
                path: None,
                version: None,
                error: None,
                install_hint: None,
            },
            passed(DependencyId::Dsh, "C:/node/dsh.cmd", "0.9.0"),
        ];
        let spec = command_spec(&items).expect("gate passed");
        assert_eq!(spec.dsh_bin, std::path::PathBuf::from("C:/node/dsh.cmd"));
        assert_eq!(spec.dsh_version, "0.9.0");
        assert_eq!(
            spec.node_bin,
            Some(std::path::PathBuf::from("C:/node/node.exe"))
        );
        assert_eq!(spec.node_version.as_deref(), Some("22.19.0"));
    }

    #[test]
    fn command_spec_rejects_red_checklist() {
        let items = vec![
            passed(DependencyId::Node, "C:/node/node.exe", "22.19.0"),
            passed(DependencyId::Npm, "C:/node/npm.cmd", "11.0.0"),
            DependencyInfo {
                id: DependencyId::Dsh,
                status: CheckStatus::Missing,
                path: None,
                version: None,
                error: Some("not found".into()),
                install_hint: None,
            },
        ];
        assert!(command_spec(&items).is_err());
    }
}
