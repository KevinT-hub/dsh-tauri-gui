use crate::app::config::RuntimeMode;
use crate::app::{AppState, RuntimeInfo};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeManifest {
    pub dsh_version: String,
    pub node_version: String,
    pub pnpm_version: String,
    pub created_at: String,
}

const RUNTIME_ARCHIVE: &str = "runtime.tar.gz";
const RUNTIME_MANIFEST: &str = "runtime.json";

fn node_path(runtime_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        runtime_dir.join("node/node.exe")
    } else {
        runtime_dir.join("node/bin/node")
    }
}

pub fn npm_cli_path(runtime_dir: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![runtime_dir.join("node/node_modules/npm/bin/npm-cli.js")]
    } else {
        vec![
            runtime_dir.join("node/lib/node_modules/npm/bin/npm-cli.js"),
            runtime_dir.join("node/node_modules/npm/bin/npm-cli.js"),
        ]
    };
    candidates.into_iter().find(|path| path.is_file())
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let names = if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            format!("{name}.bat"),
            name.to_string(),
        ]
    } else {
        vec![name.to_string()]
    };
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for candidate in &names {
            let full = directory.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

fn run_version_command(path: &Path, label: &str) -> Result<String, String> {
    let mut command = if cfg!(windows)
        && matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("cmd") | Some("bat")
        ) {
        let mut command = Command::new("cmd.exe");
        command.args(["/d", "/c"]).arg(path).arg("--version");
        command
    } else {
        let mut command = Command::new(path);
        command.arg("--version");
        command
    };
    crate::engine::hide_console(&mut command);
    let output = command
        .output()
        .map_err(|err| format!("cannot run system {label} at {}: {err}", path.display()))?;

    if !output.status.success() {
        return Err(format!(
            "system {label} --version failed with {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let version = stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| format!("system {label} returned an empty version"))?;
    Ok(version.to_string())
}

fn required_system_command(name: &str) -> Result<PathBuf, String> {
    find_on_path(name).ok_or_else(|| {
        format!("system {name} was not found on PATH; install it or switch runtime mode to bundled")
    })
}

pub fn resolve_dsh_bin(package_dir: &Path) -> Result<PathBuf, String> {
    let manifest_path = package_dir.join("package.json");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("cannot read {}: {err}", manifest_path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| format!("invalid dsh package.json: {err}"))?;
    let bin = value
        .get("bin")
        .ok_or_else(|| "dsh package.json has no bin field".to_string())?;
    let relative = if let Some(path) = bin.as_str() {
        path.to_string()
    } else if let Some(map) = bin.as_object() {
        map.get("dsh")
            .and_then(|v| v.as_str())
            .or_else(|| map.values().find_map(|v| v.as_str()))
            .map(str::to_string)
            .ok_or_else(|| "dsh bin field has no usable entry".to_string())?
    } else {
        return Err("dsh bin field has an invalid type".to_string());
    };
    let path = Path::new(&relative);
    if path.is_absolute() || relative.split(['/', '\\']).any(|part| part == "..") {
        return Err(format!("unsafe dsh bin path: {relative}"));
    }
    Ok(PathBuf::from(relative))
}

pub fn dsh_bin_path(runtime_dir: &Path) -> Result<PathBuf, String> {
    let package_dir = runtime_dir.join("app/node_modules/@deepseek-ai/dsh");
    Ok(package_dir.join(resolve_dsh_bin(&package_dir)?))
}

pub fn read_node_version(runtime_dir: &Path) -> Result<String, String> {
    // Official Node distributions do not ship a `package.json` next to the
    // binary, so the version must come from executing `node --version`.
    // This also verifies that the bundled binary actually runs.
    let node_exe = node_path(runtime_dir);
    if node_exe.exists() {
        let mut command = Command::new(&node_exe);
        command.arg("--version");
        crate::engine::hide_console(&mut command);
        let output = command
            .output()
            .map_err(|err| format!("cannot run {}: {err}", node_exe.display()))?;
        if !output.status.success() {
            return Err(format!(
                "node --version exited with {:?}",
                output.status.code()
            ));
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|err| format!("node --version produced invalid UTF-8: {err}"))?;
        if let Some(version) = parse_node_version_output(&text) {
            return Ok(version);
        }
    }
    // Fallback for distributions that do ship a root package.json.
    let manifest_path = runtime_dir.join("node/package.json");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("cannot read {}: {err}", manifest_path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| format!("invalid node package.json: {err}"))?;
    value
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "node package.json has no version".to_string())
}

fn parse_node_version_output(output: &str) -> Option<String> {
    let version = output.trim().trim_start_matches('v');
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

pub fn read_pnpm_version(runtime_dir: &Path) -> Result<String, String> {
    let manifest_path = runtime_dir.join("tools/node_modules/pnpm/package.json");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("cannot read {}: {err}", manifest_path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| format!("invalid pnpm package.json: {err}"))?;
    value
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "pnpm package.json has no version".to_string())
}

pub fn read_dsh_version(runtime_dir: &Path) -> Result<String, String> {
    read_dsh_version_at(&runtime_dir.join("app/node_modules/@deepseek-ai/dsh"))
}

pub fn read_dsh_version_at(package_dir: &Path) -> Result<String, String> {
    let manifest_path = package_dir.join("package.json");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("cannot read {}: {err}", manifest_path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| format!("invalid dsh package.json: {err}"))?;
    value
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "dsh package.json has no version".to_string())
}

fn node_version_supported(version: &str) -> bool {
    let parts: Vec<u32> = version
        .split('.')
        .take(2)
        .filter_map(|part| part.parse().ok())
        .collect();
    match parts.as_slice() {
        [22, minor] => *minor >= 19,
        [major, _] => *major >= 24,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_node_version_output;

    #[test]
    fn parses_node_version_output() {
        assert_eq!(
            parse_node_version_output("v22.19.0\n").as_deref(),
            Some("22.19.0")
        );
        assert_eq!(
            parse_node_version_output("v24.1.2").as_deref(),
            Some("24.1.2")
        );
        assert_eq!(
            parse_node_version_output("  v20.11.1  ").as_deref(),
            Some("20.11.1")
        );
        assert_eq!(parse_node_version_output(""), None);
        assert_eq!(parse_node_version_output("v\n"), None);
    }

    #[test]
    fn node_version_support_window_matches_official_engines() {
        assert!(super::node_version_supported("22.19.0"));
        assert!(super::node_version_supported("22.99.0"));
        assert!(super::node_version_supported("24.0.0"));
        assert!(!super::node_version_supported("22.18.9"));
        assert!(!super::node_version_supported("20.11.1"));
        assert!(!super::node_version_supported("23.0.0"));
    }
}

pub fn runtime_info(runtime_dir: &Path) -> Result<RuntimeInfo, String> {
    let node_exe = node_path(runtime_dir);
    if !node_exe.exists() {
        return Err(format!(
            "bundled Node runtime missing at {}",
            node_exe.display()
        ));
    }
    let node_version = read_node_version(runtime_dir)?;
    if !node_version_supported(&node_version) {
        return Err(format!(
            "bundled Node {node_version} does not satisfy dsh requirement (^22.19.0 || >=24)"
        ));
    }
    let dsh_bin = dsh_bin_path(runtime_dir)?;
    if !dsh_bin.exists() {
        return Err(format!("@deepseek-ai/dsh missing at {}", dsh_bin.display()));
    }
    Ok(RuntimeInfo {
        mode: RuntimeMode::Bundled,
        node_exe,
        dsh_bin,
        node_version,
        dsh_version: read_dsh_version(runtime_dir)?,
    })
}

pub fn system_runtime_info() -> Result<RuntimeInfo, String> {
    let node_exe = required_system_command("node")?;
    let npm_exe = required_system_command("npm")?;
    let dsh_bin = required_system_command("dsh")?;

    let node_output = run_version_command(&node_exe, "Node.js")?;
    let node_version = parse_node_version_output(&node_output)
        .ok_or_else(|| format!("cannot parse system Node.js version: {node_output}"))?;
    if !node_version_supported(&node_version) {
        return Err(format!(
            "system Node.js {node_version} does not satisfy dsh requirement (^22.19.0 || >=24)"
        ));
    }
    let _npm_version = run_version_command(&npm_exe, "npm")?;
    let dsh_version = run_version_command(&dsh_bin, "dsh")?;

    Ok(RuntimeInfo {
        mode: RuntimeMode::System,
        node_exe,
        dsh_bin,
        node_version,
        dsh_version,
    })
}

fn read_manifest(path: &Path) -> Option<RuntimeManifest> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Locate the bundled runtime archive (`runtime.tar.gz`) shipped with the
/// app, if present. `None` means the archive is not available (e.g. dev
/// builds without `runtime:prepare`), in which case the bundled runtime must
/// already be extracted locally.
pub fn bundled_archive(app: &AppHandle) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(RUNTIME_ARCHIVE));
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(
            PathBuf::from(manifest_dir)
                .join("resources")
                .join(RUNTIME_ARCHIVE),
        );
    }
    candidates.into_iter().find(|path| path.exists())
}

fn bundled_manifest(app: &AppHandle) -> Option<RuntimeManifest> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join(RUNTIME_MANIFEST));
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(
            PathBuf::from(manifest_dir)
                .join("resources")
                .join(RUNTIME_MANIFEST),
        );
    }
    candidates
        .into_iter()
        .find(|path| path.exists())
        .and_then(|path| read_manifest(&path))
}

fn extract_with_swap(archive: &Path, dest: &Path) -> Result<(), String> {
    let parent = dest
        .parent()
        .ok_or_else(|| "runtime directory has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let pid = std::process::id();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let tmp = parent.join(format!(".runtime-extract-{pid}-{nonce}"));
    let backup = parent.join(format!(".runtime-old-{pid}-{nonce}"));

    // Remove leftovers from previous crashes. Cleanup is best-effort: a
    // locked stale directory must never block startup.
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if (name.starts_with(".runtime-extract-")
                || name.starts_with(".runtime-old-")
                || name.starts_with(".swap-old-"))
                && entry.path() != tmp
                && entry.path() != backup
            {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }

    let result = (|| -> Result<(), String> {
        let file = File::open(archive)
            .map_err(|err| format!("cannot open runtime archive {}: {err}", archive.display()))?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(decoder);
        tar.unpack(&tmp)
            .map_err(|err| format!("cannot extract runtime archive: {err}"))?;
        if dest.exists() {
            fs::rename(dest, &backup).map_err(|err| err.to_string())?;
        }
        fs::rename(&tmp, dest).map_err(|err| err.to_string())?;
        if backup.exists() {
            let _ = fs::remove_dir_all(&backup);
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp);
        if backup.exists() && !dest.exists() {
            let _ = fs::rename(&backup, dest);
        }
    }
    result
}

fn swap_dir(src: &Path, dst: &Path) -> Result<(), String> {
    let parent = dst
        .parent()
        .ok_or_else(|| "swap target has no parent".to_string())?;
    let pid = std::process::id();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let backup = parent.join(format!(".swap-old-{pid}-{nonce}"));
    if dst.exists() {
        fs::rename(dst, &backup).map_err(|err| err.to_string())?;
    }
    match fs::rename(src, dst) {
        Ok(()) => {
            let _ = fs::remove_dir_all(&backup);
            Ok(())
        }
        Err(err) => {
            if backup.exists() && !dst.exists() {
                let _ = fs::rename(&backup, dst);
            }
            Err(err.to_string())
        }
    }
}

/// Extract only the `node/` and `tools/` subtrees from the bundled archive
/// and swap them into the existing runtime, preserving the hot-updated
/// `app/` directory. Used when an app update ships a newer Node/pnpm than
/// the locally extracted runtime.
fn extract_node_tools(archive: &Path, dest: &Path) -> Result<(), String> {
    let parent = dest
        .parent()
        .ok_or_else(|| "runtime directory has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let pid = std::process::id();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let tmp = parent.join(format!(".runtime-sync-{pid}-{nonce}"));

    let result = (|| -> Result<(), String> {
        fs::create_dir_all(&tmp).map_err(|err| err.to_string())?;
        let file = File::open(archive)
            .map_err(|err| format!("cannot open runtime archive {}: {err}", archive.display()))?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        let entries = archive.entries().map_err(|err| err.to_string())?;
        for entry in entries {
            let mut entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path().map_err(|err| err.to_string())?.into_owned();
            let normalized = path.to_string_lossy().replace('\\', "/");
            if normalized.starts_with("node/") || normalized.starts_with("tools/") {
                entry.unpack_in(&tmp).map_err(|err| err.to_string())?;
            }
        }
        let node_ok = tmp
            .join("node")
            .join(if cfg!(windows) {
                "node.exe"
            } else {
                "bin/node"
            })
            .exists();
        let tools_ok = tmp.join("tools/node_modules/pnpm/package.json").exists();
        if !node_ok || !tools_ok {
            return Err("bundled archive is missing node/ or tools/ subtree".to_string());
        }
        swap_dir(&tmp.join("node"), &dest.join("node"))?;
        swap_dir(&tmp.join("tools"), &dest.join("tools"))?;
        let _ = fs::remove_dir_all(&tmp);
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp);
    }
    result
}

fn node_tools_sync_needed(local: &RuntimeManifest, bundled: &RuntimeManifest) -> bool {
    local.node_version != bundled.node_version || local.pnpm_version != bundled.pnpm_version
}

fn cleanup_runtime_leftovers(runtime_dir: &Path) {
    if let Ok(entries) = fs::read_dir(runtime_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if name.starts_with(".app-staging-")
                || name.starts_with(".app-old-")
                || name.starts_with(".swap-old-")
            {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

/// Existence-based runtime fallback: a working local runtime is reused even
/// when the bundled manifest differs (after a hot dsh update or an app
/// update), so updates never force a re-download/re-extract. Extraction only
/// happens when the local runtime is missing or unusable.
pub async fn ensure_runtime(app: AppHandle, state: Arc<AppState>) -> Result<RuntimeInfo, String> {
    if state.config.lock().unwrap().runtime_mode == RuntimeMode::System {
        let info = system_runtime_info()?;
        crate::app::emit_log(
            &state,
            Some(&app),
            "INFO",
            format!(
                "[check] using system runtime: node={} dsh={} node=v{} dsh={}",
                info.node_exe.display(),
                info.dsh_bin.display(),
                info.node_version,
                info.dsh_version
            ),
        );
        return Ok(info);
    }
    cleanup_runtime_leftovers(&state.runtime_dir);
    if let Ok(info) = runtime_info(&state.runtime_dir) {
        if let (Some(bundled), Some(local)) = (
            bundled_manifest(&app),
            read_manifest(&state.runtime_dir.join(RUNTIME_MANIFEST)),
        ) {
            if node_tools_sync_needed(&local, &bundled) {
                match bundled_archive(&app) {
                    Some(archive) => {
                        crate::app::emit_log(
                            &state,
                            Some(&app),
                            "INFO",
                            format!(
                                "[check] 内置 Node/pnpm 与本地不一致（node {} -> {}，pnpm {} -> {}），正在增量同步并保留 app",
                                local.node_version,
                                bundled.node_version,
                                local.pnpm_version,
                                bundled.pnpm_version
                            ),
                        );
                        match extract_node_tools(&archive, &state.runtime_dir) {
                            Ok(()) => {
                                let dsh_version = read_dsh_version(&state.runtime_dir)
                                    .unwrap_or_else(|_| bundled.dsh_version.clone());
                                let manifest = RuntimeManifest {
                                    dsh_version,
                                    node_version: bundled.node_version.clone(),
                                    pnpm_version: bundled.pnpm_version.clone(),
                                    created_at: bundled.created_at.clone(),
                                };
                                let text = serde_json::to_string_pretty(&manifest)
                                    .map_err(|err| err.to_string())?;
                                if let Err(err) =
                                    fs::write(state.runtime_dir.join(RUNTIME_MANIFEST), text)
                                {
                                    crate::app::emit_log(
                                        &state,
                                        Some(&app),
                                        "WARN",
                                        format!("[check] 无法更新 runtime.json: {err}"),
                                    );
                                }
                                crate::app::emit_log(
                                    &state,
                                    Some(&app),
                                    "INFO",
                                    "[check] 内置 Node/pnpm 同步完成".to_string(),
                                );
                            }
                            Err(err) => {
                                crate::app::emit_log(
                                    &state,
                                    Some(&app),
                                    "WARN",
                                    format!(
                                        "[check] 内置 Node/pnpm 同步失败，继续使用现有运行时: {err}"
                                    ),
                                );
                            }
                        }
                    }
                    None => {
                        crate::app::emit_log(
                            &state,
                            Some(&app),
                            "WARN",
                            "[check] 内置运行时空档缺失，跳过 Node/pnpm 同步".to_string(),
                        );
                    }
                }
            }
        }
        crate::app::emit_log(
            &state,
            Some(&app),
            "INFO",
            format!(
                "[check] 运行时已存在且可用: node={} dsh=v{}",
                info.node_exe.display(),
                info.dsh_version
            ),
        );
        return Ok(info);
    }

    let archive = bundled_archive(&app);
    let bundled = bundled_manifest(&app);
    match archive {
        Some(archive) => {
            crate::app::emit_log(
                &state,
                Some(&app),
                "INFO",
                format!(
                    "[check] 本地运行时缺失或不可用，从内置归档解压: 归档={} 目标={}",
                    archive.display(),
                    state.runtime_dir.display()
                ),
            );
            crate::app::set_status(
                &state,
                Some(&app),
                "bootstrapping",
                "正在解压内置运行时（约 1-2 分钟）…",
                Some(format!("目标目录: {}", state.runtime_dir.display())),
                None,
                None,
                None,
            );
            let archive_for_task = archive.clone();
            let runtime_dir = state.runtime_dir.clone();
            tauri::async_runtime::spawn_blocking(move || {
                extract_with_swap(&archive_for_task, &runtime_dir)
            })
            .await
            .map_err(|err| err.to_string())??;
            if let Some(manifest) = &bundled {
                let text = serde_json::to_string_pretty(manifest).map_err(|err| err.to_string())?;
                fs::write(state.runtime_dir.join(RUNTIME_MANIFEST), text)
                    .map_err(|err| err.to_string())?;
            }
            let info = runtime_info(&state.runtime_dir)?;
            crate::app::emit_log(
                &state,
                Some(&app),
                "INFO",
                format!(
                    "[check] 运行时解压完成: node={} dsh=v{}",
                    info.node_exe.display(),
                    info.dsh_version
                ),
            );
            Ok(info)
        }
        None => Err(
            "未找到内置运行时资源。开发模式请先运行 `pnpm runtime:prepare --dev` 准备本地运行时。"
                .to_string(),
        ),
    }
}

pub fn startup(app: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        crate::app::emit_log(
            &state,
            Some(&app),
            "INFO",
            format!(
                "[check] 开始环境检测: shell_home={} engine_home={} runtime_dir={} logs_dir={}",
                state.home.display(),
                state.engine_home.display(),
                state.runtime_dir.display(),
                state.logs_dir.display()
            ),
        );
        crate::app::set_status(
            &state,
            Some(&app),
            "bootstrapping",
            "正在检查运行时…",
            None,
            None,
            None,
            None,
        );
        match ensure_runtime(app.clone(), state.clone()).await {
            Ok(info) => {
                state
                    .runtime_broken
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                crate::app::emit_log(
                    &state,
                    Some(&app),
                    "INFO",
                    format!("[check] dsh 核心加载完成: v{}", info.dsh_version),
                );
                *state.runtime.lock().unwrap() = Some(info.clone());
                let auto_start = state.config.lock().unwrap().auto_start_engine;
                if auto_start {
                    crate::app::set_status(
                        &state,
                        Some(&app),
                        "engine-starting",
                        format!("dsh 核心 v{}，正在启动引擎…", info.dsh_version),
                        None,
                        None,
                        None,
                        Some(info.dsh_version.clone()),
                    );
                    if let Err(err) = crate::engine::connect_existing_or_spawn(&app, &state) {
                        crate::app::set_status(
                            &state,
                            Some(&app),
                            "error",
                            "引擎启动失败",
                            Some(err),
                            None,
                            None,
                            Some(info.dsh_version),
                        );
                    }
                } else {
                    crate::app::set_status(
                        &state,
                        Some(&app),
                        "engine-stopped",
                        "引擎未启动（自动启动已关闭），可点击“重启引擎”。",
                        None,
                        None,
                        None,
                        Some(info.dsh_version),
                    );
                }
            }
            Err(err) => {
                state
                    .runtime_broken
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                state.logger.error(&err);
                crate::app::emit_log(
                    &state,
                    Some(&app),
                    "ERROR",
                    format!("[check] 运行时准备失败: {err}"),
                );
                crate::app::set_status(
                    &state,
                    Some(&app),
                    "error",
                    "运行时准备失败",
                    Some(err),
                    None,
                    None,
                    None,
                );
            }
        }
    });
}
