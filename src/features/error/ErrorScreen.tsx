import { useEffect, useState } from "react";
import { getDiagnostics, openLogsDir, quitApp, restartEngine } from "../../shared/bridge";
import type { Diagnostics, ShellStatus } from "../../shared/types";

interface ErrorScreenProps {
  status: ShellStatus;
}

function buildReport(diag: Diagnostics): string {
  const lines = [
    "DeepSeek Harness Desktop 错误报告",
    "================================================",
    `应用版本: ${diag.appVersion}`,
    `dsh 核心: ${diag.dshVersion ?? "未知"}`,
    `Node 版本: ${diag.nodeVersion ?? "未知"}`,
    `桌面壳目录: ${diag.shellHome}`,
    `引擎数据目录: ${diag.engineHome}`,
    `日志目录: ${diag.logsDir}`,
    `WebUI 端口: ${diag.webuiPort}`,
    "",
    "状态:",
    `  phase: ${diag.status.phase}`,
    `  message: ${diag.status.message}`,
    `  detail: ${diag.status.detail ?? ""}`,
    `  url: ${diag.status.url ?? ""}`,
    "",
    "最近日志:",
    ...(diag.logTail.length > 0 ? diag.logTail : ["（暂无日志）"]),
  ];
  return lines.join("\n");
}

export default function ErrorScreen({ status }: ErrorScreenProps) {
  const [diag, setDiag] = useState<Diagnostics | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let disposed = false;
    void getDiagnostics().then((value) => {
      if (!disposed) setDiag(value);
    });
    return () => {
      disposed = true;
    };
  }, []);

  const report = diag ? buildReport(diag) : "";

  const copyReport = async () => {
    if (!report) return;
    try {
      await navigator.clipboard.writeText(report);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      const textarea = document.createElement("textarea");
      textarea.value = report;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    }
  };

  return (
    <main className="flex h-full items-center justify-center bg-[var(--md-surface-low)] p-6 text-[var(--md-on-surface)]">
      <section className="animate-fade-in-up w-full max-w-2xl rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-6 shadow-xl">
        <header className="mb-4 flex items-center gap-3">
          <span className="grid h-10 w-10 place-items-center rounded-full bg-[var(--md-error)]/15 text-lg font-bold text-[var(--md-error)]">
            !
          </span>
          <div>
            <h1 className="text-lg font-semibold">软件遇到问题</h1>
            <p className="text-xs text-[var(--md-on-surface-variant)]">
              请复制下方错误报告，提交 ISSUE 时附带
            </p>
          </div>
        </header>

        <p className="mb-4 text-sm text-[var(--md-error)]">
          {status.detail ?? status.message}
        </p>

        <pre className="max-h-72 overflow-auto rounded-xl bg-[var(--md-surface-low)] p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all text-[var(--md-on-surface-variant)]">
          {report || "正在收集诊断信息…"}
        </pre>

        <footer className="mt-5 flex flex-wrap items-center justify-end gap-3">
          <md-outlined-button onClick={() => void copyReport()} disabled={!report}>
            {copied ? "已复制 ✓" : "复制错误报告"}
          </md-outlined-button>
          <md-outlined-button onClick={() => void restartEngine()}>
            重启引擎
          </md-outlined-button>
          <md-text-button onClick={openLogsDir}>打开日志</md-text-button>
          <md-text-button onClick={quitApp}>退出</md-text-button>
        </footer>
      </section>
    </main>
  );
}
