import type { SetupState, ThemeState } from "../../shared/types";
import { dependencyTitle } from "./setupSelectors";

interface SetupHeaderProps {
  setup: SetupState | null;
  currentTitle: string;
  themeMode: ThemeState["mode"];
  onThemeMode: (mode: "light" | "dark" | "system") => void;
}

/** 版本、区域和当前阶段摘要 */
export default function SetupHeader({
  setup,
  currentTitle,
  themeMode,
  onThemeMode,
}: SetupHeaderProps) {
  const appVersion = setup?.appVersion ?? "";
  const regionLabel =
    setup?.geo.region === "cn" ? "国内" : setup?.geo.region === "world" ? "境外" : "未知区域";

  return (
    <header className="animate-fade-in mb-5 flex items-start justify-between gap-4">
      <div>
        <p className="text-xs font-medium tracking-wide text-[var(--md-on-surface-variant)] uppercase">
          {appVersion ? `v${appVersion}` : ""} · 环境检测 · {regionLabel}
        </p>
        <h1 className="mt-1 text-2xl font-semibold">{currentTitle}</h1>
        <p className="mt-1 text-sm text-[var(--md-on-surface-variant)]">
          {setup ? `${dependencyTitle(setup.dependencies[0]?.id ?? "node")} 等外部依赖检测` : "正在加载…"}
        </p>
      </div>
      <div className="flex shrink-0 gap-1 rounded-full bg-[var(--md-surface)] p-1 ring-1 ring-[var(--md-outline)]">
        {(["light", "dark", "system"] as const).map((mode) => (
          <button
            key={mode}
            onClick={() => onThemeMode(mode)}
            className={`rounded-full px-3 py-1.5 text-xs transition-colors ${
              themeMode === mode
                ? "bg-[var(--md-primary)] font-semibold text-white"
                : "text-[var(--md-on-surface-variant)] hover:bg-[var(--md-surface-low)]"
            }`}
          >
            {mode === "light" ? "亮色" : mode === "dark" ? "暗色" : "跟随系统"}
          </button>
        ))}
      </div>
    </header>
  );
}
