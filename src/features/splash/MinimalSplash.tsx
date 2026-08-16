import { openLogsDir, quitApp, restartEngine } from "../../shared/bridge";
import type { ShellStatus } from "../../shared/types";

interface MinimalSplashProps {
  status: ShellStatus;
}

export default function MinimalSplash({ status }: MinimalSplashProps) {
  const failed = status.phase === "error";

  return (
    <main className="flex h-full flex-col items-center justify-center gap-5 bg-[var(--md-surface-low)] text-[var(--md-on-surface)]">
      <div className="animate-pop-in grid h-16 w-16 place-items-center rounded-2xl bg-gradient-to-br from-[#1f6feb] to-[#4dabf7] text-lg font-extrabold tracking-wide text-white shadow-xl">
        dsh
      </div>
      <h1 className="animate-fade-in text-lg font-semibold">
        DeepSeek Harness Desktop
      </h1>
      {failed ? (
        <div className="animate-fade-in-up flex max-w-md flex-col items-center gap-3 text-center">
          <p className="text-sm text-[var(--md-error)]">
            {status.detail ?? status.message}
          </p>
          <div className="flex gap-2">
            <md-outlined-button onClick={() => void restartEngine()}>重启引擎</md-outlined-button>
            <md-text-button onClick={openLogsDir}>打开日志</md-text-button>
            <md-text-button onClick={quitApp}>退出</md-text-button>
          </div>
        </div>
      ) : (
        <div className="animate-fade-in-up flex flex-col items-center gap-4">
          <p className="text-sm text-[var(--md-on-surface-variant)]">{status.message}</p>
          <md-circular-progress indeterminate className="h-8 w-8" />
        </div>
      )}
    </main>
  );
}
