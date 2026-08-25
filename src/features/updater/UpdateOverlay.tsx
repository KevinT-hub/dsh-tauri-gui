import { useEffect, useRef, useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  applyAppUpdate,
  checkAppUpdate,
  getAppUpdate,
  getDshUpdate,
  getUpdateNotice,
  hideUpdateOverlay,
  installDshUpdate,
  onAppUpdate,
  onDownloadProgress,
  onUpdateNotice,
} from "../../shared/bridge";
import type {
  AppUpdateInfo,
  DshUpdateInfo,
  DownloadProgressEvent,
  UpdateNotice,
} from "../../shared/types";
import { updateNoticeLabel } from "../../shared/status";

interface OverlayState {
  info: AppUpdateInfo | null;
  progress: number | null;
  phase: "idle" | "downloading" | "installing" | "error";
  error: string | null;
}

type DshPhase = "idle" | "installing" | "error";

function noticeTitle(notice: UpdateNotice): string {
  if (notice.phase === "checking") return "正在检查";
  if (notice.phase === "latest") return "检查完成";
  if (notice.phase === "available") return "发现新版本";
  if (notice.phase === "installed") return "更新已安装";
  return "检查失败";
}

export default function UpdateOverlay() {
  const [state, setState] = useState<OverlayState>({
    info: null,
    progress: null,
    phase: "idle",
    error: null,
  });
  const [notice, setNotice] = useState<UpdateNotice | null>(null);
  const [dshInfo, setDshInfo] = useState<DshUpdateInfo | null>(null);
  const [dshPhase, setDshPhase] = useState<DshPhase>("idle");
  const [dshError, setDshError] = useState<string | null>(null);
  const installing = useRef(false);
  const backendChecked = useRef(false);

  useEffect(() => {
    let disposed = false;
    let stopAppUpdate: (() => void) | undefined;
    let stopNotice: (() => void) | undefined;
    let stopProgress: (() => void) | undefined;

    const acceptNotice = (next: UpdateNotice) => {
      if (disposed) return;
      setNotice(next);
      if (next.target === "dsh" && next.phase === "available") {
        void getDshUpdate()
          .then((info) => {
            if (!disposed) setDshInfo(info);
          })
          .catch(() => undefined);
      }
    };

    void onAppUpdate((info) => {
      if (disposed) return;
      backendChecked.current = true;
      setState((prev) => ({
        ...prev,
        info: info.available ? info : null,
      }));
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stopAppUpdate = unlisten;
    });

    void onUpdateNotice(acceptNotice).then((unlisten) => {
      if (disposed) unlisten();
      else stopNotice = unlisten;
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
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stopProgress = unlisten;
    });

    void getUpdateNotice()
      .then((cachedNotice) => {
        if (cachedNotice) acceptNotice(cachedNotice);
      })
      .catch(() => undefined);
    void getAppUpdate()
      .then((info) => {
        if (!disposed && info?.available) {
          setState((prev) => ({ ...prev, info }));
        }
      })
      .catch(() => undefined);

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
      stopAppUpdate?.();
      stopNotice?.();
      stopProgress?.();
    };
  }, []);

  const startAppUpdate = async () => {
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

  const startDshUpdate = async () => {
    setDshPhase("installing");
    setDshError(null);
    try {
      await installDshUpdate();
      setNotice({
        target: "dsh",
        phase: "installed",
      });
      window.setTimeout(() => {
        void relaunch().catch((reason) => {
          setDshPhase("error");
          setDshError(reason instanceof Error ? reason.message : String(reason));
        });
      }, 500);
    } catch (error) {
      setDshPhase("error");
      setDshError(error instanceof Error ? error.message : String(error));
    }
  };

  const dismiss = () => {
    setNotice(null);
    setDshInfo(null);
    void hideUpdateOverlay();
  };

  const appAvailable =
    (notice?.target === "app" && notice.phase === "available") ||
    (!notice && state.info?.available === true);
  const dshAvailable = notice?.target === "dsh" && notice.phase === "available";
  const showResult = notice && !appAvailable && !dshAvailable;
  const showAvailableMessage =
    notice?.phase === "available" &&
    ((notice.target === "app" && !state.info) || (notice.target === "dsh" && !dshInfo));

  if (!notice && !state.info?.available) return null;

  return (
    <div className="animate-fade-in flex h-full w-full flex-col gap-2 rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)]/95 p-3 text-[var(--md-on-surface)] shadow-2xl backdrop-blur">
      <header className="flex items-center gap-2 text-sm font-semibold">
        <span
          className={`h-2 w-2 rounded-full ${
            notice?.phase === "error"
              ? "bg-[var(--md-error)]"
              : notice?.phase === "latest"
                ? "bg-[var(--md-ok)]"
                : "bg-[var(--md-primary)] shadow-[0_0_8px_var(--md-primary)]"
          }`}
        />
        {notice ? `${notice.target === "dsh" ? "dsh" : "应用"}更新` : "应用更新"}
        {notice && <span className="font-normal text-[var(--md-on-surface-variant)]">· {noticeTitle(notice)}</span>}
        {appAvailable && state.info && (
          <span className="ml-auto font-mono text-xs text-[var(--md-on-surface-variant)]">
            v{state.info.version}
          </span>
        )}
        <button
          onClick={dismiss}
          className="grid h-6 w-6 place-items-center rounded-md text-[var(--md-on-surface-variant)] hover:bg-[var(--md-surface-low)]"
          aria-label="关闭"
        >
          ×
        </button>
      </header>

      {notice && (showResult || showAvailableMessage) && (
        <div className="flex items-center gap-2 text-xs text-[var(--md-on-surface-variant)]">
          {notice.phase === "checking" && <md-circular-progress indeterminate className="h-5 w-5" />}
          <span
            className={
              notice.phase === "error"
                ? "text-[var(--md-error)]"
                : notice.phase === "latest"
                  ? "text-[var(--md-ok)]"
                  : ""
            }
          >
            {updateNoticeLabel(notice)}
          </span>
        </div>
      )}

      {appAvailable && state.info && (
        <>
          {state.info.notes && (
            <p className="line-clamp-2 text-xs text-[var(--md-on-surface-variant)]" title={state.info.notes}>
              {state.info.notes}
            </p>
          )}
          {state.phase === "downloading" && (
            <div className="flex items-center gap-2 text-xs text-[var(--md-on-surface-variant)]">
              <md-linear-progress value={(state.progress ?? 0) / 100} className="flex-1" />
              下载中 {state.progress ?? 0}%
            </div>
          )}
          {state.phase === "installing" && <p className="text-xs text-[var(--md-ok)]">正在安装…</p>}
          {state.phase === "error" && (
            <p className="line-clamp-2 text-xs text-[var(--md-error)]">更新失败：{state.error}</p>
          )}
        </>
      )}

      {dshAvailable && (
        <>
          {dshInfo && (
            <p className="text-xs text-[var(--md-on-surface-variant)]">
              当前 v{dshInfo.currentVersion}，最新 v{dshInfo.latestVersion}
            </p>
          )}
          {dshPhase === "installing" && <p className="text-xs text-[var(--md-primary)]">正在安装 dsh 更新…</p>}
          {dshPhase === "error" && <p className="line-clamp-2 text-xs text-[var(--md-error)]">更新失败：{dshError}</p>}
        </>
      )}

      <footer className="mt-auto flex justify-end gap-2">
        <md-outlined-button onClick={dismiss}>稍后</md-outlined-button>
        {appAvailable && state.info && (
          <md-filled-button
            onClick={() => void startAppUpdate()}
            disabled={state.phase !== "idle" && state.phase !== "error"}
          >
            立即更新
          </md-filled-button>
        )}
        {dshAvailable && (
          <md-filled-button onClick={() => void startDshUpdate()} disabled={dshPhase === "installing"}>
            {dshPhase === "error" ? "重试安装" : "立即更新"}
          </md-filled-button>
        )}
      </footer>
    </div>
  );
}
