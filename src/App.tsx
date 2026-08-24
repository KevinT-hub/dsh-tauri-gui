import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ErrorScreen from "./features/error/ErrorScreen";
import SetupScreen from "./features/setup/SetupScreen";
import MinimalSplash from "./features/splash/MinimalSplash";
import UpdateOverlay from "./features/updater/UpdateOverlay";
import DshUpdateDialog from "./features/updater/DshUpdateDialog";
import {
  beginSetup,
  getChecklistState,
  getShellConfig,
  getShellStatus,
  notifyShellReady,
  onDshUpdate,
  onSetupRequested,
  onShellLog,
  onShellStatus,
} from "./shared/bridge";
import type {
  ChecklistState,
  DshUpdateInfo,
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
  const logEndRef = useRef<HTMLDivElement | null>(null);
  const [windowLabel] = useState(() => getCurrentWindow().label);
  const [checklist, setChecklist] = useState<ChecklistState | null>(null);
  const [dshUpdate, setDshUpdate] = useState<DshUpdateInfo | null>(null);
  const [setupRevision, setSetupRevision] = useState(0);
  const checklistResolved = useRef(false);

  useEffect(() => {
    let disposed = false;
    void getChecklistState().then((value) => {
      if (!disposed && !checklistResolved.current) {
        setChecklist(value);
      }
    });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    void onSetupRequested(() => {
      if (disposed) return;
      checklistResolved.current = true;
      setSetupRevision((value) => value + 1);
      setChecklist((current) => ({
        required: true,
        appVersion: current?.appVersion ?? "",
      }));
    });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    void onDshUpdate((info) => {
      if (!disposed && info.available) {
        setDshUpdate(info);
      }
    });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (checklist === null || checklistResolved.current) {
      return;
    }
    checklistResolved.current = true;
    if (!checklist.required) {
      void beginSetup();
    }
  }, [checklist]);

  const handleSetupEntered = useCallback(() => {
    checklistResolved.current = true;
    setChecklist((current) => ({
      required: false,
      appVersion: current?.appVersion ?? "",
    }));
  }, []);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ block: "end" });
  }, [logs]);

  useEffect(() => {
    if (windowLabel === "update-overlay") return;
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

  let content: ReactNode;
  if (status.phase === "error") {
    content = <ErrorScreen status={status} />;
  } else if (checklist === null || checklist.required) {
    content = (
      <SetupScreen
        key={setupRevision}
        logs={logs}
        config={config}
        onConfigChange={setConfig}
        onEntered={handleSetupEntered}
        detectOnMount={Boolean(checklist?.required)}
        logEndRef={logEndRef}
      />
    );
  } else {
    content = <MinimalSplash status={status} />;
  }

  return (
    <>
      {content}
      {dshUpdate && <DshUpdateDialog info={dshUpdate} onClose={() => setDshUpdate(null)} />}
    </>
  );
}
