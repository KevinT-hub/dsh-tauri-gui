import { useCallback, useMemo, useState, type RefObject } from "react";
import {
  enterHarness,
  openLogsDir,
  quitApp,
  restartEngine,
  setShellConfig,
  setUiTheme,
} from "../../shared/bridge";
import type {
  ShellConfig,
  ShellLogLine,
  ShellStatus,
  ThemeState,
} from "../../shared/types";

type CheckState = "pending" | "checking" | "ok" | "error";

interface CheckItem {
  id: string;
  label: string;
  description: string;
  state: CheckState;
  detail?: string;
}

interface BootScreenProps {
  status: ShellStatus;
  logs: ShellLogLine[];
  config: ShellConfig | null;
  theme: ThemeState | null;
  appVersion: string;
  onConfigChange: (config: ShellConfig) => void;
  logEndRef: RefObject<HTMLDivElement | null>;
}

const CHECK_DEFS: Array<Pick<CheckItem, "id" | "label" | "description">> = [
  {
    id: "env",
    label: "运行环境",
    description: "解析桌面壳与引擎数据目录、加载本地配置",
  },
  {
    id: "runtime",
    label: "运行时准备",
    description: "检查/解压内置 Node.js 与 dsh 运行时",
  },
  {
    id: "core",
    label: "dsh 核心加载",
    description: "读取 @deepseek-ai/dsh 版本并校验完整性",
  },
  {
    id: "port",
    label: "端口与实例检测",
    description: "探测 3080 端口，复用已有 WebUI 或准备新实例",
  },
  {
    id: "engine",
    label: "引擎启动",
    description: "启动 dsh web 引擎进程并等待就绪",
  },
  {
    id: "webui",
    label: "Web UI 就绪",
    description: "确认官方 Web UI 已可访问",
  },
];

function deriveChecks(status: ShellStatus): CheckItem[] {
  const items: CheckItem[] = CHECK_DEFS.map((def) => ({
    ...def,
    state: "pending" as CheckState,
  }));
  const failAt = (index: number, detail?: string) => {
    items[index].state = "error";
    items[index].detail = detail ?? status.detail ?? status.message;
  };

  switch (status.phase) {
    case "idle":
      items[0].state = "checking";
      break;
    case "bootstrapping":
      items[0].state = "ok";
      items[1].state = "checking";
      break;
    case "engine-starting":
      items[0].state = "ok";
      items[1].state = "ok";
      items[2].state = "ok";
      items[3].state = "checking";
      items[4].state = "checking";
      break;
    case "updating":
      items[0].state = "ok";
      items[1].state = "ok";
      items[2].state = "checking";
      break;
    case "engine-ready":
      items.forEach((item) => {
        item.state = "ok";
      });
      break;
    case "engine-stopped":
      items.forEach((item, index) => {
        item.state = index < 4 ? "ok" : index === 4 ? "error" : "pending";
      });
      items[4].detail = status.detail ?? status.message;
      break;
    case "error": {
      const detail = `${status.detail ?? ""} ${status.message}`;
      const index = detail.includes("运行时")
        || detail.includes("system Node")
        || detail.includes("system npm")
        || detail.includes("system dsh")
        || detail.includes("bundled runtime")
        ? 1
        : detail.includes("dsh") || detail.includes("核心")
          ? 2
          : detail.includes("端口") || detail.includes("EADDRINUSE") || detail.includes("占用")
            ? 3
            : detail.includes("Web")
              ? 5
              : 4;
      items.forEach((item, i) => {
        item.state = i < index ? "ok" : i === index ? "error" : "pending";
      });
      failAt(index);
      break;
    }
  }
  return items;
}

function statusText(state: CheckState): string {
  switch (state) {
    case "checking":
      return "检查中…";
    case "ok":
      return "已通过";
    case "error":
      return "未通过";
    default:
      return "等待中";
  }
}

function StatusIcon({ state }: { state: CheckState }) {
  if (state === "checking") {
    return <md-circular-progress indeterminate className="h-5 w-5" />;
  }
  if (state === "ok") {
    return (
      <span className="grid h-5 w-5 place-items-center rounded-full bg-[var(--md-ok)] text-[11px] font-bold text-white">
        ✓
      </span>
    );
  }
  if (state === "error") {
    return (
      <span className="grid h-5 w-5 place-items-center rounded-full bg-[var(--md-error)] text-[11px] font-bold text-white">
        !
      </span>
    );
  }
  return <span className="h-2.5 w-2.5 rounded-full border-2 border-[var(--md-outline)]" />;
}

export default function BootScreen({
  status,
  logs,
  config,
  theme,
  appVersion,
  onConfigChange,
  logEndRef,
}: BootScreenProps) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const checks = useMemo(() => deriveChecks(status), [status]);
  const active = selectedId ?? checks.find((item) => item.state !== "ok")?.id ?? checks[0].id;
  const current = checks.find((item) => item.id === active) ?? checks[0];
  const allPassed = checks.every((item) => item.state === "ok");
  const ready = status.phase === "engine-ready";

  const busyAction = useCallback(async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
    } finally {
      setBusy(false);
    }
  }, []);

  const setThemeMode = useCallback(
    (mode: ThemeState["mode"]) => {
      void busyAction(() => setUiTheme(mode));
    },
    [busyAction],
  );

  const toggleMinimizeToTray = useCallback(async () => {
    if (!config) return;
    const next = await setShellConfig({ minimizeToTray: !config.minimizeToTray });
    onConfigChange(next);
  }, [config, onConfigChange]);

  return (
    <main className="flex h-full bg-[var(--md-surface-low)] text-[var(--md-on-surface)]">
      <aside className="flex w-64 shrink-0 flex-col gap-1 border-r border-[var(--md-outline)] bg-[var(--md-surface)]/60 p-3">
        <md-list className="flex flex-col gap-1 overflow-y-auto">
          {checks.map((item, index) => (
            <md-list-item
              key={item.id}
              type="button"
              onClick={() => setSelectedId(item.id)}
              className={`animate-fade-in-up rounded-xl ${
                item.id === active
                  ? "bg-[var(--md-surface)] shadow-sm ring-1 ring-[var(--md-outline)]"
                  : "hover:bg-[var(--md-surface)]/60"
              }`}
              style={{ animationDelay: `${index * 55}ms` }}
            >
              <span slot="start">
                <StatusIcon state={item.state} />
              </span>
              <span slot="headline">{item.label}</span>
              <span slot="supporting-text">{statusText(item.state)}</span>
            </md-list-item>
          ))}
        </md-list>
      </aside>

      <section className="flex min-w-0 flex-1 flex-col p-6">
        <header className="animate-fade-in mb-5 flex items-start justify-between gap-4">
          <div>
            <p className="text-xs font-medium tracking-wide text-[var(--md-on-surface-variant)] uppercase">
              {appVersion ? `v${appVersion}` : ""} · 环境检测
            </p>
            <h1 className="mt-1 text-2xl font-semibold">{current.label}</h1>
            <p className="mt-1 text-sm text-[var(--md-on-surface-variant)]">
              {current.description}
            </p>
          </div>
          <div className="flex shrink-0 gap-1 rounded-full bg-[var(--md-surface)] p-1 ring-1 ring-[var(--md-outline)]">
            {(["light", "dark", "system"] as const).map((mode) => (
              <button
                key={mode}
                onClick={() => void setThemeMode(mode)}
                className={`rounded-full px-3 py-1.5 text-xs transition-colors ${
                  theme?.mode === mode
                    ? "bg-[var(--md-primary)] font-semibold text-white"
                    : "text-[var(--md-on-surface-variant)] hover:bg-[var(--md-surface-low)]"
                }`}
              >
                {mode === "light" ? "亮色" : mode === "dark" ? "暗色" : "跟随系统"}
              </button>
            ))}
          </div>
        </header>

        <div key={current.id} className="animate-fade-in-up flex-1 space-y-4">
          <div className="rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-5">
            {current.state === "checking" ? (
              <div className="flex items-center gap-3 text-sm text-[var(--md-on-surface-variant)]">
                <md-circular-progress indeterminate className="h-6 w-6" />
                正在检查，请稍候…
              </div>
            ) : current.state === "error" ? (
              <p className="text-sm text-[var(--md-error)]">
                {current.detail ?? status.detail ?? status.message}
              </p>
            ) : current.state === "ok" ? (
              <p className="text-sm text-[var(--md-ok)]">{status.message}</p>
            ) : (
              <p className="text-sm text-[var(--md-on-surface-variant)]">
                等待前置检查完成
              </p>
            )}
            {status.url && (
              <p className="mt-2 font-mono text-xs text-[var(--md-primary)]">{status.url}</p>
            )}
          </div>

          <div className="rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-4">
            <p className="mb-2 text-xs font-medium text-[var(--md-on-surface-variant)]">
              最近日志
            </p>
            <div className="max-h-48 overflow-y-auto font-mono text-xs leading-relaxed text-[var(--md-on-surface-variant)]">
              {logs.length === 0 ? (
                <p>暂无日志</p>
              ) : (
                logs.map((entry, index) => (
                  <pre
                    key={index}
                    className={`whitespace-pre-wrap break-all ${
                      entry.level === "error"
                        ? "text-[var(--md-error)]"
                        : entry.level === "warn"
                          ? "text-[var(--md-warn)]"
                          : ""
                    }`}
                  >
                    {entry.line}
                  </pre>
                ))
              )}
              <div ref={logEndRef} />
            </div>
          </div>

          {config && (
            <div className="space-y-2 rounded-xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-4 text-xs text-[var(--md-on-surface-variant)]">
              <p className="font-medium text-[var(--md-on-surface)]">运行时</p>
              <div className="flex items-center gap-2">
                <span
                  className={`rounded-full px-2.5 py-1 text-[11px] font-semibold ${
                    config.runtimeMode === "bundled"
                      ? "bg-[var(--md-primary)]/15 text-[var(--md-primary)]"
                      : "bg-[var(--md-surface-low)] text-[var(--md-on-surface-variant)]"
                  }`}
                >
                  {config.runtimeMode === "bundled"
                    ? "内置运行时（推荐）"
                    : "系统运行时"}
                </span>
                {config.runtimeMode === "system" && (
                  <span className="text-[var(--md-warn)]">
                    系统模式不会修改全局 npm 包，dsh 更新由系统环境自行管理。
                  </span>
                )}
              </div>
              <p>
                如需在两种运行时之间切换，请使用系统托盘菜单中的「运行时」。切换将在应用重启后生效。
              </p>
            </div>
          )}
          {config && (
            <label className="flex cursor-pointer items-center gap-2 text-xs text-[var(--md-on-surface-variant)] select-none">
              <input
                type="checkbox"
                checked={config.minimizeToTray}
                onChange={() => void toggleMinimizeToTray()}
                className="h-4 w-4 accent-[var(--md-primary)]"
              />
              关闭窗口时最小化到托盘
            </label>
          )}
        </div>

        <footer className="mt-6 flex items-center justify-between gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <md-outlined-button onClick={() => void busyAction(restartEngine)} disabled={busy}>
              重启引擎
            </md-outlined-button>
            <md-text-button onClick={openLogsDir}>打开日志</md-text-button>
            <md-text-button onClick={quitApp}>退出</md-text-button>
          </div>
          {allPassed && ready && (
            <md-filled-button
              className="animate-pop-in"
              disabled={busy}
              onClick={() => void busyAction(enterHarness)}
            >
              进入 Harness →
            </md-filled-button>
          )}
        </footer>
      </section>
    </main>
  );
}
