import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import BootScreen from "./features/boot/BootScreen";
import ErrorScreen from "./features/error/ErrorScreen";
import MinimalSplash from "./features/splash/MinimalSplash";
import UpdateOverlay from "./features/updater/UpdateOverlay";
import {
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
