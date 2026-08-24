// 设置页文案与错误提示映射

import type { DependencyId } from "../../shared/types";

/** 每个依赖项的检测描述 */
export function dependencyDescription(id: DependencyId): string {
  switch (id) {
    case "node":
      return "Node.js 22.19+ 或 24+（官方 dsh 要求）";
    case "npm":
      return "npm 包管理器（随 Node.js 安装）";
    case "pnpm":
      return "pnpm 包管理器（可选，与 npm 二选一）";
    case "dsh":
      return "官方 @deepseek-ai/dsh CLI";
  }
}

/** 依赖缺失时的安装指引主文案 */
export function missingMessage(id: DependencyId): string {
  switch (id) {
    case "node":
      return "未检测到 Node.js。请安装 Node.js 官方发行版（LTS 22.x 或 24.x）。";
    case "npm":
      return "未检测到 npm。npm 随 Node.js 官方发行版一同安装。";
    case "pnpm":
      return "未检测到 pnpm。可使用 corepack（随 Node.js 提供）启用，或按官方文档安装。";
    case "dsh":
      return "未检测到 dsh。请安装官方 CLI 包：npm install -g @deepseek-ai/dsh";
  }
}

/** 通用错误提示兜底 */
export function genericError(): string {
  return "环境检测遇到问题，请查看日志或稍后重试。";
}

/** 进入按钮文案 */
export function enterLabel(allPassed: boolean): string {
  return allPassed ? "进入 Harness →" : "环境未就绪";
}
