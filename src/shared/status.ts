import type { ShellStatus, ShellStatusCode, UpdateNotice } from "./types";

const STATUS_LABELS: Record<ShellStatusCode, string> = {
  initializing: "正在初始化…",
  environmentChecking: "正在检查运行环境…",
  environmentFailed: "运行环境检查未通过",
  environmentCheckFailed: "环境检查失败",
  engineStarting: "正在准备引擎…",
  engineRestarting: "正在重新准备引擎…",
  engineRestartingAfterCrash: "引擎正在恢复…",
  engineReady: "引擎已就绪",
  engineReadyExisting: "已连接到运行中的引擎",
  engineStopped: "引擎已停止",
  engineStartFailed: "引擎启动失败",
  engineRestartFailed: "引擎重启失败",
  webUiOpenFailed: "Web UI 打开失败",
};

export function shellStatusLabel(status: ShellStatus): string {
  return STATUS_LABELS[status.code] ?? status.code;
}

export function updateNoticeLabel(notice: UpdateNotice): string {
  if (notice.phase === "checking") return "正在检查…";
  if (notice.phase === "latest") {
    return `当前 ${notice.target === "dsh" ? "dsh" : "应用"} 已是最新版本`;
  }
  if (notice.phase === "available") {
    const version = notice.version ? ` v${notice.version}` : "";
    return `发现${notice.target === "dsh" ? " dsh" : "应用"}新版本${version}`;
  }
  if (notice.phase === "installed") {
    return `${notice.target === "dsh" ? "dsh" : "应用"}更新已安装，应用即将重启`;
  }
  return notice.error
    ? `检查失败：${notice.error}`
    : "检查更新失败";
}
