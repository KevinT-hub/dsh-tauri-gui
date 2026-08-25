import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ErrorScreen from "./features/error/ErrorScreen";
import SetupScreen from "./features/setup/SetupScreen";
import MinimalSplash from "./features/splash/MinimalSplash";
import UpdateOverlay from "./features/updater/UpdateOverlay";
import DshUpdateDialog from "./features/updater/DshUpdateDialog";
import {
  getChecklistState,
  getDshUpdate,
  getShellConfig,
  getShellStatus,
  notifyShellReady,
  onDshUpdate,
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
    code: "initializing",
  });
  const [logs, setLogs] = useState<ShellLogLine[]>([]);
  const [config, setConfig] = useState<ShellConfig | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlistenStatus: (() => void) | undefined;
    let unlistenLog: (() => void) | undefined;
    void getShellStatus().then((value) => {
      if (!disposed) setStatus(value);
    });
    void getShellConfig().then((value) => {
      if (!disposed) setConfig(value);
    });
    void onShellStatus((value) => {
      if (!disposed) setStatus(value);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenStatus = unlisten;
    });
    void onShellLog((value) => {
      if (disposed) return;
      setLogs((prev) => [...prev.slice(-(MAX_LOG_LINES - 1)), value]);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenLog = unlisten;
    });
    return () => {
      disposed = true;
      unlistenStatus?.();
      unlistenLog?.();
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
    let unlisten: (() => void) | undefined;
    void getDshUpdate().then((info) => {
      if (!disposed && info?.available) {
        setDshUpdate(info);
      }
    });
    void onDshUpdate((info) => {
      if (!disposed && info.available) {
        setDshUpdate(info);
      }
    }).then((remove) => {
      if (disposed) remove();
      else unlisten = remove;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

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
    const frame = window.requestAnimationFrame(() => void notifyShellReady());
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
        logs={logs}
        config={config}
        status={status}
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
