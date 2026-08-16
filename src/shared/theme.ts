import { useEffect, useState } from "react";
import { getThemeState, onTheme } from "./bridge";
import type { ThemeState } from "./types";

const darkQuery = () =>
  typeof window !== "undefined"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;

export function resolveTheme(state: ThemeState): "light" | "dark" {
  if (state.effective === "light" || state.effective === "dark") {
    return state.effective;
  }
  return darkQuery()?.matches ? "dark" : "light";
}

export function applyTheme(state: ThemeState): void {
  const theme = resolveTheme(state);
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
}

/**
 * Shell theme state with live updates:
 * - shell mode (light/dark/system) from desktop config
 * - web UI preference from `$DSH_HOME/settings.yaml` (mapped by Rust)
 * - OS preference when both are "system"
 */
export function useTheme(): ThemeState | null {
  const [theme, setTheme] = useState<ThemeState | null>(null);

  useEffect(() => {
    let disposed = false;
    void getThemeState().then((state) => {
      if (disposed) return;
      setTheme(state);
      applyTheme(state);
    });
    void onTheme((state) => {
      if (disposed) return;
      setTheme(state);
      applyTheme(state);
    });
    const query = darkQuery();
    const onChange = () => {
      if (disposed) return;
      setTheme((prev) => {
        if (!prev || prev.effective !== "system") return prev;
        applyTheme(prev);
        return { ...prev };
      });
    };
    query?.addEventListener("change", onChange);
    return () => {
      disposed = true;
      query?.removeEventListener("change", onChange);
    };
  }, []);

  return theme;
}
