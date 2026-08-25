import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppUpdateInfo,
  ChecklistState,
  DependencyId,
  Diagnostics,
  DownloadProgressEvent,
  GeoState,
  DshUpdateInfo,
  SetupState,
  ShellConfig,
  ShellLogLine,
  ShellStatus,
  ThemeState,
  UpdateNotice,
} from "./types";

// ---------------------------------------------------------------------------
// shell 状态与配置
// ---------------------------------------------------------------------------

export function getShellStatus(): Promise<ShellStatus> {
  return invoke<ShellStatus>("shell_status");
}

export function getShellConfig(): Promise<ShellConfig> {
  return invoke<ShellConfig>("get_shell_config");
}

export function setShellConfig(patch: Partial<ShellConfig>): Promise<ShellConfig> {
  return invoke<ShellConfig>("set_shell_config", { patch });
}

export function getDiagnostics(): Promise<Diagnostics> {
  return invoke<Diagnostics>("get_diagnostics");
}

export function getChecklistState(): Promise<ChecklistState> {
  return invoke<ChecklistState>("checklist_state");
}

export function notifyShellReady(): Promise<void> {
  return invoke("shell_ready");
}

// ---------------------------------------------------------------------------
// 环境检测 / 安装帮助
// ---------------------------------------------------------------------------

let detectionPromise: Promise<SetupState> | null = null;

/**
 * 运行环境检测。使用单例 Promise 防止并发调用：
 * - React StrictMode 双 mount 不会触发两次检测
 * - 用户快速点击「重新检测」时复用正在进行的检测
 */
export function runDetection(): Promise<SetupState> {
  if (detectionPromise) {
    return detectionPromise;
  }
  detectionPromise = invoke<SetupState>("run_detection_v2").finally(() => {
    detectionPromise = null;
  });
  return detectionPromise;
}

export function markSetupSeen(): Promise<void> {
  return invoke("mark_setup_seen");
}

export function installDependency(dependency: DependencyId): Promise<SetupState> {
  return invoke("install_dependency_v2", { dependency });
}

export function getGeoState(): Promise<GeoState> {
  return invoke<GeoState>("get_geo_state");
}

export function enterHarness(): Promise<void> {
  return invoke("enter_harness");
}

export function beginSetup(): Promise<void> {
  return invoke("begin_setup_v2");
}

export function recheckEnvironment(): Promise<void> {
  return invoke("recheck_environment_v2");
}

// ---------------------------------------------------------------------------
// 引擎控制
// ---------------------------------------------------------------------------

export function restartEngine(): Promise<void> {
  return invoke("restart_engine");
}

export function openWebUi(): Promise<void> {
  return invoke("open_web_ui");
}

export function openLogsDir(): Promise<void> {
  return invoke("open_logs_dir");
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

// ---------------------------------------------------------------------------
// 主题
// ---------------------------------------------------------------------------

export function getThemeState(): Promise<ThemeState> {
  return invoke<ThemeState>("get_theme_state");
}

export function setUiTheme(mode: ThemeState["mode"]): Promise<ThemeState> {
  return invoke<ThemeState>("set_ui_theme", { mode });
}

// ---------------------------------------------------------------------------
// 应用更新
// ---------------------------------------------------------------------------

export function checkAppUpdate(): Promise<AppUpdateInfo> {
  return invoke<AppUpdateInfo>("check_app_update");
}

export function getAppUpdate(): Promise<AppUpdateInfo | null> {
  return invoke<AppUpdateInfo | null>("get_app_update");
}

export function applyAppUpdate(): Promise<void> {
  return invoke("apply_app_update");
}

export function installDshUpdate(): Promise<void> {
  return invoke("install_dsh_update");
}

export function getDshUpdate(): Promise<DshUpdateInfo | null> {
  return invoke<DshUpdateInfo | null>("get_dsh_update");
}

export function hideUpdateOverlay(): Promise<void> {
  return invoke("hide_update_overlay");
}

export function getUpdateNotice(): Promise<UpdateNotice | null> {
  return invoke<UpdateNotice | null>("get_update_notice");
}

// ---------------------------------------------------------------------------
// 事件订阅
// ---------------------------------------------------------------------------

export function onShellStatus(
  handler: (status: ShellStatus) => void,
): Promise<UnlistenFn> {
  return listen<ShellStatus>("shell://status", (event) => handler(event.payload));
}

export function onShellLog(
  handler: (line: ShellLogLine) => void,
): Promise<UnlistenFn> {
  return listen<ShellLogLine>("shell://log", (event) => handler(event.payload));
}

export function onTheme(handler: (theme: ThemeState) => void): Promise<UnlistenFn> {
  return listen<ThemeState>("shell://theme", (event) => handler(event.payload));
}

export function onAppUpdate(
  handler: (info: AppUpdateInfo) => void,
): Promise<UnlistenFn> {
  return listen<AppUpdateInfo>("shell://app-update", (event) =>
    handler(event.payload),
  );
}

export function onDshUpdate(
  handler: (info: DshUpdateInfo) => void,
): Promise<UnlistenFn> {
  return listen<DshUpdateInfo>("shell://dsh-update", (event) =>
    handler(event.payload),
  );
}

export function onUpdateNotice(
  handler: (notice: UpdateNotice) => void,
): Promise<UnlistenFn> {
  return listen<UpdateNotice>("shell://update-notice", (event) =>
    handler(event.payload),
  );
}

export function onDownloadProgress(
  handler: (event: DownloadProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgressEvent>("updater-download-progress", (event) =>
    handler(event.payload),
  );
}
