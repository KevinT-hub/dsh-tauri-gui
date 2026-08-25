import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import {
  enterHarness,
  openLogsDir,
  onTheme,
  quitApp,
  restartEngine,
  runDetection,
  setShellConfig,
  setUiTheme,
} from "../../shared/bridge";
import type { ShellConfig, ShellLogLine, ShellStatus, SetupState } from "../../shared/types";
import { shellStatusLabel } from "../../shared/status";
import DependencyChecklist from "./DependencyChecklist";
import DependencyDetails from "./DependencyDetails";
import InstallActionButton from "./InstallActionButton";
import InstallHelp from "./InstallHelp";
import SetupHeader from "./SetupHeader";
import SetupLogPanel from "./SetupLogPanel";
import SourceSelector from "./SourceSelector";
import { canEnter, deriveRows } from "./setupSelectors";

interface SetupScreenProps {
  logs: ShellLogLine[];
  config: ShellConfig | null;
  status: ShellStatus;
  onConfigChange: (config: ShellConfig) => void;
  onEntered: () => void;
  detectOnMount: boolean;
  logEndRef: RefObject<HTMLDivElement | null>;
}

export default function SetupScreen({
  logs,
  config,
  status,
  onConfigChange,
  onEntered,
  detectOnMount,
  logEndRef,
}: SetupScreenProps) {
  const [setup, setSetup] = useState<SetupState | null>(null);
  const [detectionError, setDetectionError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [themeMode, setThemeMode] = useState<ShellConfig["uiTheme"]>("system");
  const [feedback, setFeedback] = useState<string | null>(null);
  const detectionStarted = useRef(false);

  useEffect(() => {
    if (config) {
      setThemeMode(config.uiTheme);
    }
  }, [config]);

  useEffect(() => {
    let disposed = false;
    void onTheme((theme) => {
      if (disposed) return;
      setThemeMode(theme.mode);
    });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (!detectOnMount || detectionStarted.current) {
      return;
    }
    detectionStarted.current = true;
    let disposed = false;
    setBusy(true);
    void runDetection()
      .then((value) => {
        if (disposed) return;
        setSetup(value);
        setDetectionError(null);
        setFeedback(value.allPassed ? "环境检测通过，可以进入 DeepSeek Harness" : "检测完成，请根据提示处理缺失依赖");
      })
      .catch((error) => {
        if (disposed) return;
        const message = error instanceof Error ? error.message : String(error);
        if (message.includes("检测正在进行中")) {
          return;
        }
        setDetectionError(message);
      })
      .finally(() => {
        if (!disposed) setBusy(false);
      });
    return () => {
      disposed = true;
    };
  }, [detectOnMount]);

  const rows = useMemo(() => deriveRows(setup?.dependencies ?? []), [setup]);
  const activeId = selectedId ?? rows.find((row) => row.rowState !== "passed")?.key ?? rows[0]?.key ?? null;
  const current = rows.find((row) => row.key === activeId) ?? rows[0] ?? null;

  const busyAction = useCallback(async (action: () => Promise<unknown>, successMessage?: string) => {
    setBusy(true);
    setFeedback(null);
    try {
      await action();
      if (successMessage) setFeedback(successMessage);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setFeedback(message);
      setDetectionError(message);
    } finally {
      setBusy(false);
    }
  }, []);

  const handleThemeMode = useCallback(
    (mode: ShellConfig["uiTheme"]) => {
      setThemeMode(mode);
      void busyAction(() => setUiTheme(mode), "外观设置已更新");
    },
    [busyAction],
  );

  const toggleMinimizeToTray = useCallback(async () => {
    if (!config) return;
    const next = await setShellConfig({ minimizeToTray: !config.minimizeToTray });
    onConfigChange(next);
  }, [config, onConfigChange]);

  const handleDetectionResult = useCallback((value: SetupState) => {
    setSetup(value);
    setDetectionError(null);
    setSelectedId(null);
    setFeedback(value.allPassed ? "环境检测通过，可以进入 DeepSeek Harness" : "检测完成，请根据提示处理缺失依赖");
  }, []);

  const handleDetectionError = useCallback((error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes("检测正在进行中")) {
      return;
    }
    setDetectionError(message);
  }, []);

  const reDetect = useCallback(() => {
    detectionStarted.current = true;
    setBusy(true);
    setFeedback("正在重新检测环境…");
    void runDetection()
      .then(handleDetectionResult)
      .catch(handleDetectionError)
      .finally(() => setBusy(false));
  }, [handleDetectionError, handleDetectionResult]);

  const onInstalled = useCallback((value: SetupState) => {
    handleDetectionResult(value);
  }, [handleDetectionResult]);

  const handleEnter = useCallback(async () => {
    setBusy(true);
    setFeedback("正在启动 DeepSeek Harness…");
    try {
      await enterHarness();
      onEntered();
      setFeedback("引擎已启动，正在加载 Web UI…");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setDetectionError(message);
      setBusy(false);
    }
  }, [onEntered]);

  return (
    <main className="flex h-full bg-[var(--md-surface-low)] text-[var(--md-on-surface)]">
      <DependencyChecklist rows={rows} activeId={activeId} onSelect={setSelectedId} loading={busy} />

      <section className="flex min-w-0 flex-1 flex-col p-6">
        <SetupHeader
          setup={setup}
          currentTitle={current?.title ?? "环境检测"}
          themeMode={themeMode}
          onThemeMode={handleThemeMode}
        />

        <div key={current?.key ?? "empty"} className="animate-fade-in-up flex-1 space-y-4">
          {detectionError && (
            <div className="rounded-2xl border border-[var(--md-error)]/40 bg-[var(--md-error)]/10 p-4 text-sm text-[var(--md-error)]">
              检测失败：{detectionError}
            </div>
          )}
          {current && <DependencyDetails row={current} />}
          {current && <InstallHelp row={current} />}
          {current && (
            <InstallActionButton
              row={current}
              busy={busy}
              onBusyChange={setBusy}
              onInstalled={onInstalled}
            />
          )}
          <SourceSelector sourcePolicy={setup?.sourcePolicy ?? null} />
          <SetupLogPanel logs={logs} logEndRef={logEndRef} />

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
            <md-outlined-button onClick={reDetect} disabled={busy}>
              重新检测
            </md-outlined-button>
            <md-outlined-button
              onClick={() => void busyAction(restartEngine, "引擎已重新启动，正在等待 Web UI")}
              disabled={busy}
            >
              重启引擎
            </md-outlined-button>
            <md-text-button onClick={openLogsDir}>打开日志目录</md-text-button>
            <md-text-button onClick={quitApp}>退出</md-text-button>
          </div>
          {(feedback || (status.phase !== "idle" && status.phase !== "error")) && (
            <p className="min-w-0 flex-1 text-right text-xs text-[var(--md-on-surface-variant)]">
              {feedback ?? shellStatusLabel(status)}
            </p>
          )}
          {canEnter(setup, busy) && (
            <md-filled-button
              className="animate-pop-in"
              disabled={busy}
              onClick={() => void handleEnter()}
            >
              进入 DeepSeek Harness →
            </md-filled-button>
          )}
        </footer>
      </section>
    </main>
  );
}
