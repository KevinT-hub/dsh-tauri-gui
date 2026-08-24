import type { RefObject } from "react";
import type { ShellLogLine } from "../../shared/types";

interface SetupLogPanelProps {
  logs: ShellLogLine[];
  logEndRef: RefObject<HTMLDivElement | null>;
}

/** 检测与安装实时日志 */
export default function SetupLogPanel({ logs, logEndRef }: SetupLogPanelProps) {
  return (
    <div className="rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-4">
      <p className="mb-2 text-xs font-medium text-[var(--md-on-surface-variant)]">最近日志</p>
      <div className="max-h-48 overflow-y-auto font-mono text-xs leading-relaxed text-[var(--md-on-surface-variant)]">
        {logs.length === 0 ? (
          <p>暂无日志</p>
        ) : (
          logs.map((entry, index) => (
            <pre
              key={index}
              className={`whitespace-pre-wrap break-all ${
                entry.level === "error"
                  ? "text-[var(--md-error)]"
                  : entry.level === "warn"
                    ? "text-[var(--md-warn)]"
                    : ""
              }`}
            >
              {entry.line}
            </pre>
          ))
        )}
        <div ref={logEndRef} />
      </div>
    </div>
  );
}
