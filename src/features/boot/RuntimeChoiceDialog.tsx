import { useState } from "react";
import { setShellConfig } from "../../shared/bridge";
import type { RuntimeMode, ShellConfig } from "../../shared/types";

interface RuntimeChoiceDialogProps {
  config: ShellConfig;
  appVersion: string;
  onConfigChange: (config: ShellConfig) => void;
  onConfirm: () => void;
}

const OPTIONS: Array<{
  mode: RuntimeMode;
  title: string;
  tag: string;
  description: string;
}> = [
  {
    mode: "bundled",
    title: "内置运行时",
    tag: "推荐",
    description:
      "使用应用内置的 Node.js 与 dsh，开箱即用、版本固定，稳定且无需额外安装。",
  },
  {
    mode: "system",
    title: "系统运行时",
    tag: "高级",
    description:
      "使用系统中已安装的 Node、npm 与 dsh（来自 PATH）。适合已自行管理环境的进阶用户。",
  },
];

export default function RuntimeChoiceDialog({
  config,
  appVersion,
  onConfigChange,
  onConfirm,
}: RuntimeChoiceDialogProps) {
  const [selected, setSelected] = useState<RuntimeMode>(
    config.runtimeMode ?? "bundled",
  );
  const [submitting, setSubmitting] = useState(false);

  const submit = async () => {
    if (submitting) return;
    setSubmitting(true);
    try {
      const next = await setShellConfig({
        runtimeMode: selected,
        runtimeModeSelected: true,
      });
      onConfigChange(next);
      onConfirm();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <main className="fixed inset-0 z-50 flex items-center justify-center bg-[var(--md-surface-low)] p-6 text-[var(--md-on-surface)]">
      <section className="animate-fade-in-up w-full max-w-lg rounded-3xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-7 shadow-2xl">
        <p className="text-xs font-medium tracking-wide text-[var(--md-on-surface-variant)] uppercase">
          {appVersion ? `v${appVersion} · ` : ""}首次启动
        </p>
        <h1 className="mt-1 text-2xl font-semibold">选择运行时</h1>
        <p className="mt-2 text-sm text-[var(--md-on-surface-variant)]">
          在正式检测环境之前，请选择 dsh 引擎所使用的运行时。该选项仅需在首次启动时确认一次，之后可随时通过系统托盘菜单切换。
        </p>

        <div className="mt-5 space-y-3">
          {OPTIONS.map((option) => {
            const active = selected === option.mode;
            return (
              <button
                key={option.mode}
                type="button"
                onClick={() => setSelected(option.mode)}
                className={`flex w-full items-start gap-3 rounded-2xl border p-4 text-left transition-colors ${
                  active
                    ? "border-[var(--md-primary)] bg-[var(--md-primary)]/10"
                    : "border-[var(--md-outline)] hover:bg-[var(--md-surface-low)]"
                }`}
              >
                <span
                  className={`mt-0.5 grid h-5 w-5 shrink-0 place-items-center rounded-full border-2 ${
                    active
                      ? "border-[var(--md-primary)]"
                      : "border-[var(--md-outline)]"
                  }`}
                >
                  {active && (
                    <span className="h-2.5 w-2.5 rounded-full bg-[var(--md-primary)]" />
                  )}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="flex items-center gap-2">
                    <span className="font-medium">{option.title}</span>
                    <span
                      className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${
                        option.mode === "bundled"
                          ? "bg-[var(--md-primary)]/15 text-[var(--md-primary)]"
                          : "bg-[var(--md-surface-low)] text-[var(--md-on-surface-variant)]"
                      }`}
                    >
                      {option.tag}
                    </span>
                  </span>
                  <span className="mt-1 block text-xs leading-relaxed text-[var(--md-on-surface-variant)]">
                    {option.description}
                  </span>
                </span>
              </button>
            );
          })}
        </div>

        <footer className="mt-6 flex justify-end">
          <md-filled-button
            className="w-full sm:w-auto"
            disabled={submitting}
            onClick={() => void submit()}
          >
            {submitting ? "正在准备…" : "开始检测 →"}
          </md-filled-button>
        </footer>
      </section>
    </main>
  );
}
