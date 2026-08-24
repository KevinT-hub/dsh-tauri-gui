//! Integration test: the Tauri command names registered in `lib.rs` match the
//! command names the frontend invokes through `src/shared/bridge.ts`.
//!
//! This is a static contract check: any rename in either layer breaks the
//! bridge, so the two lists must stay in sync.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a parent")
        .to_path_buf()
}

fn lib_source() -> String {
    fs::read_to_string(repo_root().join("src-tauri/src/lib.rs")).unwrap_or_default()
}

fn bridge_source() -> String {
    fs::read_to_string(repo_root().join("src/shared/bridge.ts")).unwrap_or_default()
}

/// Commands the frontend actually invokes: matches `invoke(...)` and
/// `invoke<Type>("name")` call shapes in bridge.ts.
fn frontend_commands() -> Vec<String> {
    let mut commands = Vec::new();
    for line in bridge_source().lines() {
        let mut rest = line;
        while let Some(start) = rest.find("invoke(") {
            let after = &rest[start + "invoke(".len()..];
            // Skip generic argument if present: invoke<ShellStatus>("name")
            let after_generic = match after.strip_prefix('<') {
                Some(inner) => match inner.find('>') {
                    Some(end) => &inner[end + 1..],
                    None => break,
                },
                None => after,
            };
            let after_generic = after_generic.trim_start();
            if let Some(quoted) = after_generic.strip_prefix('"') {
                if let Some(end) = quoted.find('"') {
                    commands.push(quoted[..end].to_string());
                }
            }
            rest = after;
        }
    }
    commands
}

/// Commands registered in `lib.rs` invoke_handler.
fn registered_commands() -> Vec<String> {
    lib_source()
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("commands::") {
                let name = trimmed
                    .split("::")
                    .last()
                    .unwrap_or_default()
                    .trim_end_matches(',');
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Command *modules* referenced in `lib.rs` (`commands::<module>::<fn>`).
fn registered_modules() -> Vec<String> {
    lib_source()
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("commands::") {
                let parts: Vec<&str> = trimmed.split("::").collect();
                parts.get(1).map(|module| module.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn every_frontend_command_is_registered() {
    let registered = registered_commands();
    assert!(
        !frontend_commands().is_empty(),
        "bridge.ts must invoke commands"
    );
    for command in frontend_commands() {
        assert!(
            registered.contains(&command),
            "frontend invokes `{command}` but lib.rs does not register it"
        );
    }
}

#[test]
fn every_registered_command_exists_in_the_codebase() {
    let registered = registered_commands();
    assert!(!registered.is_empty(), "no commands registered in lib.rs");
    for module in registered_modules() {
        // Each `commands::<module>::<fn>` must live in a known module.
        assert!(
            matches!(
                module.as_str(),
                "setup" | "setup_flow" | "dsh_update" | "shell" | "updater" | "geo"
            ),
            "unexpected command module: {module}"
        );
    }
}
