import { useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import { installDshUpdate } from "../../shared/bridge";
import type { DshUpdateInfo } from "../../shared/types";

interface DshUpdateDialogProps {
  info: DshUpdateInfo;
  onClose: () => void;
}

export default function DshUpdateDialog({ info, onClose }: DshUpdateDialogProps) {
  const [phase, setPhase] = useState<"idle" | "installing" | "error">("idle");
  const [error, setError] = useState<string | null>(null);

  const install = async () => {
    setPhase("installing");
    setError(null);
    try {
      await installDshUpdate();
      window.setTimeout(() => {
        void relaunch().catch((reason) => {
          setPhase("error");
          setError(reason instanceof Error ? reason.message : String(reason));
        });
      }, 500);
    } catch (reason) {
      setPhase("error");
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/35 p-6">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="dsh-update-title"
        className="w-full max-w-lg rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-6 text-[var(--md-on-surface)] shadow-2xl"
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-[var(--md-primary)]">
              DeepSeek Harness 更新
            </p>
            <h2 id="dsh-update-title" className="mt-1 text-xl font-semibold">
              发现新版本 v{info.latestVersion}
            </h2>
          </div>
          <button
            type="button"
            aria-label="关闭"
            className="grid h-8 w-8 place-items-center rounded-md text-lg text-[var(--md-on-surface-variant)] hover:bg-[var(--md-surface-low)]"
            onClick={onClose}
            disabled={phase === "installing"}
          >
            ×
          </button>
        </div>

        <div className="mt-5 space-y-3 text-sm text-[var(--md-on-surface-variant)]">
          <p>
            当前版本 <span className="font-mono text-[var(--md-on-surface)]">v{info.currentVersion}</span>
          </p>
          <p>安装成功后软件会自动重启，并重新检测 Node.js、npm/pnpm 和 DeepSeek Harness 版本。</p>
          <code className="block overflow-x-auto rounded-lg bg-[var(--md-surface-low)] p-3 font-mono text-xs text-[var(--md-on-surface)]">
            {info.installCommand}
          </code>
        </div>

        {phase === "installing" && (
          <p className="mt-4 text-sm text-[var(--md-primary)]">正在安装更新，完成后自动重启…</p>
        )}
        {error && <p className="mt-4 text-sm text-[var(--md-error)]">更新失败：{error}</p>}

        <footer className="mt-6 flex justify-end gap-3">
          <md-outlined-button onClick={onClose} disabled={phase === "installing"}>
            稍后
          </md-outlined-button>
          <md-filled-button onClick={() => void install()} disabled={phase === "installing"}>
            {phase === "error" ? "重试安装" : "确定更新"}
          </md-filled-button>
        </footer>
      </section>
    </div>
  );
}
