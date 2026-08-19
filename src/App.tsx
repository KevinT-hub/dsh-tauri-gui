import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import BootScreen from "./features/boot/BootScreen";
import RuntimeChoiceDialog from "./features/boot/RuntimeChoiceDialog";
import ErrorScreen from "./features/error/ErrorScreen";
import MinimalSplash from "./features/splash/MinimalSplash";
import UpdateOverlay from "./features/updater/UpdateOverlay";
import {
  beginBootstrap,
  getChecklistState,
  getShellConfig,
  getShellStatus,
  notifyShellReady,
  onShellLog,
  onShellStatus,
} from "./shared/bridge";
import { useTheme } from "./shared/theme";
import type {
  ChecklistState,
  ShellConfig,
  ShellLogLine,
  ShellStatus,
} from "./shared/types";

const MAX_LOG_LINES = 120;

function useShellState() {
  const [status, setStatus] = useState<ShellStatus>({
    phase: "idle",
    message: "正在连接桌面壳…",
  });
  const [logs, setLogs] = useState<ShellLogLine[]>([]);
  const [config, setConfig] = useState<ShellConfig | null>(null);

  useEffect(() => {
    let disposed = false;
    void getShellStatus().then((value) => {
      if (!disposed) setStatus(value);
    });
    void getShellConfig().then((value) => {
      if (!disposed) setConfig(value);
    });
    void onShellStatus((value) => {
      if (!disposed) setStatus(value);
    });
    void onShellLog((value) => {
      if (disposed) return;
      setLogs((prev) => [...prev.slice(-(MAX_LOG_LINES - 1)), value]);
    });
    return () => {
      disposed = true;
    };
  }, []);

  return { status, logs, config, setConfig };
}

export default function App() {
  const { status, logs, config, setConfig } = useShellState();
  const theme = useTheme();
  const logEndRef = useRef<HTMLDivElement | null>(null);
  const [windowLabel] = useState(() => getCurrentWindow().label);
  const [checklist, setChecklist] = useState<ChecklistState | null>(null);
  const bootstrapRef = useRef(false);

  // Begin the environment detection / engine bootstrap exactly once. The
  // runtime mode must be settled first (see RuntimeChoiceDialog), so this is
  // triggered either after the one-time choice or immediately on later runs.
  const triggerBootstrap = useCallback(() => {
    if (bootstrapRef.current) return;
    bootstrapRef.current = true;
    void beginBootstrap();
  }, []);

  useEffect(() => {
    let disposed = false;
    void getChecklistState().then((value) => {
      if (!disposed) setChecklist(value);
    });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    // On runs where the runtime was already chosen, start detection as soon
    // as the config is known. The first-run dialog triggers it itself.
    if (config?.runtimeModeSelected) {
      triggerBootstrap();
    }
  }, [config, triggerBootstrap]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ block: "end" });
  }, [logs]);

  useEffect(() => {
    if (windowLabel === "update-overlay") return;
    // Tell Rust the shell page has actually painted; only then may the
    // checklist window be revealed (no black flash).
    const frame = window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        void notifyShellReady();
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [windowLabel]);

  if (windowLabel === "update-overlay") {
    return <UpdateOverlay />;
  }

  if (status.phase === "error") {
    return <ErrorScreen status={status} />;
  }

  if (checklist === null || checklist.required) {
    // The one-time runtime chooser appears *before* detection on first run.
    if (config && !config.runtimeModeSelected) {
      return (
        <RuntimeChoiceDialog
          config={config}
          appVersion={checklist?.appVersion ?? ""}
          onConfigChange={setConfig}
          onConfirm={triggerBootstrap}
        />
      );
    }
    return (
      <BootScreen
        status={status}
        logs={logs}
        config={config}
        theme={theme}
        appVersion={checklist?.appVersion ?? ""}
        onConfigChange={setConfig}
        logEndRef={logEndRef}
      />
    );
  }

  return <MinimalSplash status={status} />;
}
