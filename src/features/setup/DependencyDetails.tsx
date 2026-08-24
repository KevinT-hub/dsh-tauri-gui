import type { SetupRow } from "./setupSelectors";
import { statusLabel } from "./setupSelectors";

interface DependencyDetailsProps {
  row: SetupRow;
}

/** 单项依赖的路径、版本、错误和诊断信息 */
export default function DependencyDetails({ row }: DependencyDetailsProps) {
  return (
    <div className="rounded-2xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-5">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-medium">{row.title}</p>
        <span
          className={`rounded-full px-2.5 py-1 text-[11px] font-semibold ${
            row.rowState === "passed"
              ? "bg-[var(--md-ok)]/15 text-[var(--md-ok)]"
              : row.rowState === "checking"
                ? "bg-[var(--md-surface-low)] text-[var(--md-on-surface-variant)]"
                : "bg-[var(--md-error)]/15 text-[var(--md-error)]"
          }`}
        >
          {statusLabel(row.status)}
        </span>
      </div>

      {row.path && (
        <p className="mt-3 break-all font-mono text-xs text-[var(--md-on-surface-variant)]">
          路径：{row.path}
        </p>
      )}
      {row.version && (
        <p className="mt-1 text-xs text-[var(--md-on-surface-variant)]">
          版本：{row.version}
        </p>
      )}
      {row.error && <p className="mt-2 text-sm text-[var(--md-error)]">{row.error}</p>}
      {row.installHint && (
        <p className="mt-2 text-xs text-[var(--md-warn)]">{row.installHint}</p>
      )}
      {row.rowState === "checking" && (
        <div className="mt-3 flex items-center gap-3 text-sm text-[var(--md-on-surface-variant)]">
          <md-circular-progress indeterminate className="h-6 w-6" />
          正在检查，请稍候…
        </div>
      )}
    </div>
  );
}
