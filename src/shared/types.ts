export type ShellPhase =
  | "idle"
  | "bootstrapping"
  | "engine-starting"
  | "engine-ready"
  | "engine-stopped"
  | "updating"
  | "error";

export interface ShellStatus {
  phase: ShellPhase;
  message: string;
  detail?: string;
  url?: string;
  progress?: number;
  engineVersion?: string;
}

export interface ShellLogLine {
  level: "info" | "warn" | "error";
  line: string;
}

export interface RuntimeUpdateCheck {
  current: string;
  latest: string;
  updateAvailable: boolean;
}

export type RuntimeMode = "bundled" | "system";

export interface ShellConfig {
  minimizeToTray: boolean;
  autoStartEngine: boolean;
  restartOnCrash: boolean;
  telemetryDisabled: boolean;
  npmRegistry: string;
  defaultWorkspace: string | null;
  uiTheme: "light" | "dark" | "system";
  firstRunCompleted: boolean;
  lastChecklistVersion: string;
  webuiPort: number;
  engineHome: string | null;
  runtimeMode: RuntimeMode;
  runtimeModeSelected: boolean;
}

export interface ChecklistState {
  required: boolean;
  appVersion: string;
}

export interface Diagnostics {
  appVersion: string;
  dshVersion: string | null;
  nodeVersion: string | null;
  runtimeMode: RuntimeMode | null;
  shellHome: string;
  engineHome: string;
  runtimeDir: string;
  logsDir: string;
  webuiPort: number;
  status: ShellStatus;
  logTail: string[];
}

export interface ThemeState {
  mode: "light" | "dark" | "system";
  webuiPreference: "light" | "dark" | "system" | null;
  effective: "light" | "dark" | "system";
}

export interface AppUpdateInfo {
  available: boolean;
  version: string;
  notes: string;
  date: string;
  downloadUrl: string;
  sha256: string;
  source: string;
}

export interface DownloadProgressEvent {
  event: "Started" | "Progress" | "Finished";
  contentLength?: number;
  chunkLength?: number;
  downloaded: number;
  total?: number;
  percentage?: number;
}
