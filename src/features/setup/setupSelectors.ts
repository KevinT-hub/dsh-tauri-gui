// 设置页派生状态与按钮可用性判断（纯函数，便于测试）

import type {
  CheckStatus,
  DependencyInfo,
  DependencyId,
  SetupState,
} from "../../shared/types";

/** 检测列表行的展示状态 */
export type RowState =
  | "checking"
  | "passed"
  | "missing"
  | "unsupported"
  | "unknown";

export interface SetupRow extends DependencyInfo {
  key: string;
  title: string;
  rowState: RowState;
}

/** 将 Rust 检测结果映射为带展示状态的列表行 */
export function deriveRows(dependencies: DependencyInfo[]): SetupRow[] {
  const byId = new Map<DependencyId, DependencyInfo>(
    dependencies.map((dependency) => [dependency.id, dependency]),
  );
  const node = byId.get("node") ?? emptyDependency("node");
  const npm = byId.get("npm") ?? emptyDependency("npm");
  const pnpm = byId.get("pnpm") ?? emptyDependency("pnpm");
  const dsh = byId.get("dsh") ?? emptyDependency("dsh");

  return [
    toRow(node, "node", dependencyTitle("node")),
    aggregatePackageManagerRow(npm, pnpm),
    toRow(dsh, "dsh", dependencyTitle("dsh")),
  ];
}

/** 状态文案（用于列表 supporting-text 与详情区） */
export function statusLabel(status: CheckStatus): string {
  switch (status) {
    case "checking":
      return "检查中…";
    case "passed":
      return "已通过";
    case "missing":
      return "未找到";
    case "unsupported":
      return "版本不满足";
    case "unknown":
      return "状态未知";
  }
}

/** 依赖项是否处于"可安装/可帮助"状态（缺失或版本不支持） */
export function needsInstallHelp(row: SetupRow): boolean {
  return row.rowState === "missing" || row.rowState === "unsupported";
}

/** "进入"按钮可用性：全部通过且没有进行中的操作 */
export function canEnter(setup: SetupState | null, busy: boolean): boolean {
  return Boolean(setup?.allPassed) && !busy;
}

/** 渲染用的依赖标题 */
export function dependencyTitle(id: DependencyId): string {
  switch (id) {
    case "node":
      return "Node.js";
    case "npm":
      return "npm";
    case "pnpm":
      return "pnpm";
    case "dsh":
      return "DeepSeek Harness";
  }
}

/** 检测结果摘要（诊断/日志用） */
export function summarize(dependencies: DependencyInfo[]): string {
  const parts = dependencies.map((dep) => {
    const version = dep.version ? `v${dep.version}` : dep.status;
    return `${dep.id}=${version}`;
  });
  return parts.join(" ");
}

function emptyDependency(id: DependencyId): DependencyInfo {
  return {
    id,
    status: "checking",
    path: null,
    version: null,
    error: null,
    installHint: null,
  };
}

function toRow(dep: DependencyInfo, key: string, title: string): SetupRow {
  return {
    ...dep,
    key,
    title,
    rowState: dep.status as RowState,
  };
}

function aggregatePackageManagerRow(npm: DependencyInfo, pnpm: DependencyInfo): SetupRow {
  const rowState = combineStatus([npm.status, pnpm.status]);
  const passed = [npm, pnpm].find((item) => item.status === "passed");
  const representative = passed ?? (pnpm.status !== "passed" ? pnpm : npm);
  return {
    ...representative,
    id: representative.id,
    key: "package-manager",
    title: "npm / pnpm",
    rowState,
    path: passed?.path ?? representative.path,
    version: passed?.version ?? representative.version,
    error: rowState === "passed" ? null : combineErrors(npm, pnpm),
    installHint: rowState === "passed" ? null : pnpm.installHint ?? npm.installHint,
  };
}

function combineStatus(statuses: CheckStatus[]): RowState {
  if (statuses.some((status) => status === "checking")) {
    return "checking";
  }
  if (statuses.some((status) => status === "passed")) {
    return "passed";
  }
  if (statuses.some((status) => status === "unsupported")) {
    return "unsupported";
  }
  if (statuses.some((status) => status === "unknown")) {
    return "unknown";
  }
  return "missing";
}

function combineErrors(...dependencies: DependencyInfo[]): string | null {
  const messages = dependencies
    .map((dependency) => dependency.error)
    .filter((message): message is string => Boolean(message));
  if (messages.length === 0) {
    return null;
  }
  return messages.join("；");
}
