import type { DependencyId, SetupState } from "../../shared/types";
import { installDependency } from "../../shared/bridge";
import type { SetupRow } from "./setupSelectors";

interface InstallActionButtonProps {
  row: SetupRow;
  busy: boolean;
  onBusyChange: (busy: boolean) => void;
  onInstalled: (setup: SetupState) => void;
}

const ACTIONS: Partial<Record<DependencyId, { label: string; hint: string }>> = {
  pnpm: { label: "启用 corepack", hint: "运行 corepack enable" },
  dsh: { label: "安装 dsh", hint: "npm install -g @deepseek-ai/dsh" },
};

export default function InstallActionButton({
  row,
  busy,
  onBusyChange,
  onInstalled,
}: InstallActionButtonProps) {
  const action = ACTIONS[row.id];
  if (!action || row.rowState === "passed" || row.rowState === "checking") {
    return null;
  }

  const runInstall = async () => {
    onBusyChange(true);
    try {
      const setup = await installDependency(row.id);
      onInstalled(setup);
    } catch (error) {
      console.error("install failed", error);
    } finally {
      onBusyChange(false);
    }
  };

  return (
    <div className="flex flex-wrap items-center gap-3">
      <md-filled-button disabled={busy} onClick={() => void runInstall()}>
        {action.label}
      </md-filled-button>
      <span className="text-xs text-[var(--md-on-surface-variant)]">{action.hint}</span>
    </div>
  );
}
