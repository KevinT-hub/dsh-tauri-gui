import type { SetupRow } from "./setupSelectors";
import { statusLabel } from "./setupSelectors";

function StatusIcon({ state }: { state: SetupRow["rowState"] }) {
  if (state === "checking") {
    return <md-circular-progress indeterminate className="h-5 w-5" />;
  }
  if (state === "passed") {
    return (
      <span className="grid h-5 w-5 place-items-center rounded-full bg-[var(--md-ok)] text-[11px] font-bold text-white">
        ✓
      </span>
    );
  }
  if (state === "missing" || state === "unsupported") {
    return (
      <span className="grid h-5 w-5 place-items-center rounded-full bg-[var(--md-error)] text-[11px] font-bold text-white">
        !
      </span>
    );
  }
  return <span className="h-2.5 w-2.5 rounded-full border-2 border-[var(--md-outline)]" />;
}

interface DependencyRowProps {
  row: SetupRow;
  active: boolean;
  onClick: () => void;
  index: number;
}

export default function DependencyRow({ row, active, onClick, index }: DependencyRowProps) {
  return (
    <md-list-item
      type="button"
      onClick={onClick}
      className={`animate-fade-in-up rounded-xl ${
        active
          ? "bg-[var(--md-surface)] shadow-sm ring-1 ring-[var(--md-outline)]"
          : "hover:bg-[var(--md-surface)]/60"
      }`}
      style={{ animationDelay: `${index * 55}ms` }}
    >
      <span slot="start">
        <StatusIcon state={row.rowState} />
      </span>
      <span slot="headline">{row.title}</span>
      <span slot="supporting-text">{statusLabel(row.status)}</span>
    </md-list-item>
  );
}
