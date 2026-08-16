import { useEffect, useRef, useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  applyAppUpdate,
  checkAppUpdate,
  hideUpdateOverlay,
  onAppUpdate,
  onDownloadProgress,
} from "../../shared/bridge";
import type { AppUpdateInfo, DownloadProgressEvent } from "../../shared/types";

interface OverlayState {
  info: AppUpdateInfo | null;
  progress: number | null;
  phase: "idle" | "downloading" | "installing" | "error";
  error: string | null;
}

export default function UpdateOverlay() {
  const [state, setState] = useState<OverlayState>({
    info: null,
    progress: null,
    phase: "idle",
    error: null,
  });
  const installing = useRef(false);
  const backendChecked = useRef(false);

  useEffect(() => {
    let disposed = false;
    void onAppUpdate((info) => {
      if (disposed) return;
      backendChecked.current = true;
      if (!info.available) return;
      setState((prev) => ({ ...prev, info }));
    });
    void onDownloadProgress((event: DownloadProgressEvent) => {
      if (disposed) return;
      if (event.event === "Finished") {
        installing.current = true;
        setState((prev) => ({ ...prev, phase: "installing", progress: 100 }));
        return;
      }
      setState((prev) => ({
        ...prev,
        phase: "downloading",
        progress: event.percentage ?? null,
      }));
    });
    const fallbackTimer = window.setTimeout(() => {
      if (disposed || backendChecked.current) return;
      void checkAppUpdate()
        .then((info) => {
          if (disposed || !info.available) return;
          setState((prev) => ({ ...prev, info }));
        })
        .catch(() => undefined);
    }, 6000);
    return () => {
      disposed = true;
      window.clearTimeout(fallbackTimer);
    };
  }, []);

  const startUpdate = async () => {
    setState((prev) => ({ ...prev, phase: "downloading", progress: 0, error: null }));
    try {
      await applyAppUpdate();
      if (!installing.current) {
        installing.current = true;
        setState((prev) => ({ ...prev, phase: "installing", progress: 100 }));
      }
      window.setTimeout(() => void relaunch(), 800);
    } catch (error) {
      setState((prev) => ({
        ...prev,
        phase: "error",
        error: error instanceof Error ? error.message : String(error),
      }));
    }
  };

  const info = state.info;
  if (!info) return null;

  return (
    <div className="animate-fade-in flex h-full w-full flex-col gap-2 rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)]/95 p-3 text-[var(--md-on-surface)] shadow-2xl backdrop-blur">
      <header className="flex items-center gap-2 text-sm font-semibold">
        <span className="h-2 w-2 rounded-full bg-[var(--md-primary)] shadow-[0_0_8px_var(--md-primary)]" />
        新版本可用
        <span className="ml-auto font-mono text-xs text-[var(--md-on-surface-variant)]">
          v{info.version}
        </span>
        <button
          onClick={() => void hideUpdateOverlay()}
          className="grid h-6 w-6 place-items-center rounded-md text-[var(--md-on-surface-variant)] hover:bg-[var(--md-surface-low)]"
          aria-label="关闭"
        >
          ×
        </button>
      </header>
      {info.notes && (
        <p className="line-clamp-2 text-xs text-[var(--md-on-surface-variant)]" title={info.notes}>
          {info.notes}
        </p>
      )}
      {state.phase === "downloading" && (
        <div className="flex items-center gap-2 text-xs text-[var(--md-on-surface-variant)]">
          <md-linear-progress value={(state.progress ?? 0) / 100} className="flex-1" />
          下载中 {state.progress ?? 0}%
        </div>
      )}
      {state.phase === "installing" && (
        <p className="text-xs text-[var(--md-ok)]">正在安装…</p>
      )}
      {state.phase === "error" && (
        <p className="line-clamp-2 text-xs text-[var(--md-error)]">更新失败：{state.error}</p>
      )}
      <footer className="mt-auto flex justify-end gap-2">
        <md-outlined-button onClick={() => void hideUpdateOverlay()}>稍后</md-outlined-button>
        <md-filled-button
          onClick={() => void startUpdate()}
          disabled={state.phase !== "idle" && state.phase !== "error"}
        >
          立即更新
        </md-filled-button>
      </footer>
    </div>
  );
}
