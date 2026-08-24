import type { SourcePolicy } from "../../shared/types";

interface SourceSelectorProps {
  sourcePolicy: SourcePolicy | null;
}

const REGION_LABEL: Record<SourcePolicy["region"], string> = {
  cn: "国内（镜像源）",
  world: "境外（官方源）",
  unknown: "未识别（官方源）",
};

/** 官方源/镜像源选择和 geo 结果展示 */
export default function SourceSelector({ sourcePolicy }: SourceSelectorProps) {
  if (!sourcePolicy) return null;
  return (
    <div className="space-y-2 rounded-xl border border-[var(--md-outline)] bg-[var(--md-surface)] p-4 text-xs text-[var(--md-on-surface-variant)]">
      <p className="font-medium text-[var(--md-on-surface)]">软件源策略</p>
      <p>
        区域：{REGION_LABEL[sourcePolicy.region]}
        {sourcePolicy.region === "unknown" && "（geo 检测不可用，默认使用官方源，可手动切换镜像）"}
      </p>
      <p className="break-all">npm registry：{sourcePolicy.npmRegistry}</p>
      <p className="break-all">Node 镜像：{sourcePolicy.nodeMirror}</p>
    </div>
  );
}
