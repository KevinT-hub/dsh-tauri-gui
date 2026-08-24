// Rust command/event 对应的公共类型（与 src-tauri 侧 serde 契约一一对应）

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

// ---------------------------------------------------------------------------
// 环境检测（detection）
// ---------------------------------------------------------------------------

export type DependencyId = "node" | "npm" | "pnpm" | "dsh";

export type CheckStatus =
  | "checking"
  | "passed"
  | "missing"
  | "unsupported"
  | "unknown";

export interface DependencyInfo {
  id: DependencyId;
  status: CheckStatus;
  path: string | null;
  version: string | null;
  error: string | null;
  installHint: string | null;
}

export type RegionCode = "cn" | "world" | "unknown";

export interface GeoResult {
  region: RegionCode;
  country: string | null;
  matched: number;
  total: number;
  sources: string[];
}

export interface SourcePolicy {
  region: RegionCode;
  npmRegistry: string;
  nodeMirror: string;
}

export interface SetupState {
  appVersion: string;
  dependencies: DependencyInfo[];
  allPassed: boolean;
  sourcePolicy: SourcePolicy;
  geo: GeoResult;
}

export interface GeoState {
  geo: GeoResult;
  sourcePolicy: SourcePolicy;
}

// ---------------------------------------------------------------------------
// 应用配置与诊断
// ---------------------------------------------------------------------------

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
  setupSeenVersion: string;
  webuiPort: number;
  engineHome: string | null;
}

export interface ChecklistState {
  required: boolean;
  appVersion: string;
}

export interface Diagnostics {
  appVersion: string;
  dshVersion: string | null;
  nodeVersion: string | null;
  shellHome: string;
  engineHome: string;
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

// ---------------------------------------------------------------------------
// 应用更新
// ---------------------------------------------------------------------------

export interface AppUpdateInfo {
  available: boolean;
  version: string;
  notes: string;
  date: string;
  downloadUrl: string;
  sha256: string;
  source: string;
}

export interface DshUpdateInfo {
  available: boolean;
  currentVersion: string;
  latestVersion: string;
  installCommand: string;
  registry: string;
}

export interface DownloadProgressEvent {
  event: "Started" | "Progress" | "Finished";
  contentLength?: number;
  chunkLength?: number;
  downloaded: number;
  total?: number;
  percentage?: number;
}
