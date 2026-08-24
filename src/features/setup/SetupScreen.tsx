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
import type { ShellConfig, ShellLogLine, SetupState } from "../../shared/types";
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
  onConfigChange: (config: ShellConfig) => void;
  onEntered: () => void;
  detectOnMount: boolean;
  logEndRef: RefObject<HTMLDivElement | null>;
}

export default function SetupScreen({
  logs,
  config,
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

  const busyAction = useCallback(async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
    } finally {
      setBusy(false);
    }
  }, []);

  const handleThemeMode = useCallback(
    (mode: ShellConfig["uiTheme"]) => {
      setThemeMode(mode);
      void busyAction(() => setUiTheme(mode));
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
    try {
      await enterHarness();
      onEntered();
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
            <md-outlined-button onClick={() => void busyAction(restartEngine)} disabled={busy}>
              重启引擎
            </md-outlined-button>
            <md-text-button onClick={openLogsDir}>打开日志目录</md-text-button>
            <md-text-button onClick={quitApp}>退出</md-text-button>
          </div>
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
