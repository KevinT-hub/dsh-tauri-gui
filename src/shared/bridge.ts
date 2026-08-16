import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppUpdateInfo,
  ChecklistState,
  Diagnostics,
  DownloadProgressEvent,
  RuntimeUpdateCheck,
  ShellConfig,
  ShellLogLine,
  ShellStatus,
  ThemeState,
} from "./types";

export function getShellStatus(): Promise<ShellStatus> {
  return invoke<ShellStatus>("shell_status");
}

export function getShellConfig(): Promise<ShellConfig> {
  return invoke<ShellConfig>("get_shell_config");
}

export function setShellConfig(patch: Partial<ShellConfig>): Promise<ShellConfig> {
  return invoke<ShellConfig>("set_shell_config", { patch });
}

export function restartEngine(): Promise<void> {
  return invoke("restart_engine");
}

export function openLogsDir(): Promise<void> {
  return invoke("open_logs_dir");
}

export function openWebUi(): Promise<void> {
  return invoke("open_web_ui");
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

export function checkRuntimeUpdate(): Promise<RuntimeUpdateCheck> {
  return invoke<RuntimeUpdateCheck>("check_runtime_update");
}

export function applyRuntimeUpdate(): Promise<void> {
  return invoke("apply_runtime_update");
}

export function getThemeState(): Promise<ThemeState> {
  return invoke<ThemeState>("get_theme_state");
}

export function setUiTheme(mode: ThemeState["mode"]): Promise<ThemeState> {
  return invoke<ThemeState>("set_ui_theme", { mode });
}

export function hideUpdateOverlay(): Promise<void> {
  return invoke("hide_update_overlay");
}

export function getChecklistState(): Promise<ChecklistState> {
  return invoke<ChecklistState>("checklist_state");
}

export function notifyShellReady(): Promise<void> {
  return invoke("shell_ready");
}

export function getDiagnostics(): Promise<Diagnostics> {
  return invoke<Diagnostics>("get_diagnostics");
}

export function enterHarness(): Promise<void> {
  return invoke("enter_harness");
}

export function checkAppUpdate(): Promise<AppUpdateInfo> {
  return invoke<AppUpdateInfo>("check_app_update");
}

export function applyAppUpdate(): Promise<void> {
  return invoke("apply_app_update");
}

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

export function onDownloadProgress(
  handler: (event: DownloadProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgressEvent>("updater-download-progress", (event) =>
    handler(event.payload),
  );
}
