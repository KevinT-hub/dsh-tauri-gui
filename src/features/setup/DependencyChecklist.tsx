import type { SetupRow } from "./setupSelectors";
import DependencyRow from "./DependencyRow";

interface DependencyChecklistProps {
  rows: SetupRow[];
  activeId: string | null;
  onSelect: (id: string) => void;
  loading?: boolean;
}

/** Node/npm/pnpm/dsh 检测列表 */
export default function DependencyChecklist({
  rows,
  activeId,
  onSelect,
  loading,
}: DependencyChecklistProps) {
  return (
    <aside
      aria-busy={loading}
      className="flex w-64 shrink-0 flex-col gap-1 border-r border-[var(--md-outline)] bg-[var(--md-surface)]/60 p-3"
    >
      <md-list
        className={
          "flex flex-col gap-1 overflow-y-auto" + (loading ? " opacity-95" : "")
        }
      >
        {rows.map((row, index) => (
          <DependencyRow
            key={row.key}
            row={row}
            index={index}
            active={row.key === activeId}
            onClick={() => onSelect(row.key)}
          />
        ))}
      </md-list>
    </aside>
  );
}
