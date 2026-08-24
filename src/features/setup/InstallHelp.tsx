import type { SetupRow } from "./setupSelectors";
import { dependencyDescription, missingMessage } from "./setupCopy";

interface InstallHelpProps {
  row: SetupRow;
}

export default function InstallHelp({ row }: InstallHelpProps) {
  if (row.rowState === "passed" || row.rowState === "checking") {
    return null;
  }
  return (
    <div className="space-y-2 rounded-xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-4 text-xs text-[var(--md-on-surface-variant)]">
      <p className="font-medium text-[var(--md-on-surface)]">
        {row.title} / {dependencyDescription(row.id)}
      </p>
      <p>{missingMessage(row.id)}</p>
      {row.installHint && <p className="text-[var(--md-warn)]">{row.installHint}</p>}
    </div>
  );
}
