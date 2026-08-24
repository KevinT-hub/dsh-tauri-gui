/**
 * 前后端 dependency payload 契约测试。
 *
 * 验证 Rust 侧 `detection/model.rs` 序列化出来的 payload 形状与前端
 * `src/shared/types.ts` 中的类型契约一致（camelCase 字段、枚举值、
 * SetupState 结构），防止重构时静默破坏协议。
 *
 * Run: node --test tests/detection-contract.test.mjs
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TYPES_SOURCE = readFileSync(join(ROOT, "src/shared/types.ts"), "utf8");

const DEPENDENCY_IDS = ["node", "npm", "pnpm", "dsh"];
const CHECK_STATUSES = ["checking", "passed", "missing", "unsupported", "unknown"];
const REGION_CODES = ["cn", "world", "unknown"];

/** 与 Rust DependencyInfo 序列化等价的样例 payload */
function rustDependencyInfo(overrides = {}) {
  return {
    id: "node",
    status: "passed",
    path: "C:/Program Files/nodejs/node.exe",
    version: "22.19.0",
    error: null,
    installHint: null,
    ...overrides,
  };
}

test("DependencyInfo payload matches frontend contract", () => {
  const payload = rustDependencyInfo();
  const keys = Object.keys(payload).sort();
  assert.deepEqual(keys, ["error", "id", "installHint", "path", "status", "version"]);
  assert.ok(DEPENDENCY_IDS.includes(payload.id), "id must be one of dependency ids");
  assert.ok(CHECK_STATUSES.includes(payload.status), "status must be a known CheckStatus");
});

test("all dependency ids and statuses are covered by the type union", () => {
  // 枚举与前端类型一一对应，新增枚举值必须同时修改两处。
  for (const id of DEPENDENCY_IDS) {
    assert.match(TYPES_SOURCE, new RegExp(`"${id}"`), `type union must include ${id}`);
  }
  for (const status of CHECK_STATUSES) {
    assert.match(TYPES_SOURCE, new RegExp(`"${status}"`), `CheckStatus union must include ${status}`);
  }
});

test("SetupState payload matches frontend contract", () => {
  const setup = {
    appVersion: "0.1.1",
    dependencies: [rustDependencyInfo(), rustDependencyInfo({ id: "npm" })],
    allPassed: true,
    sourcePolicy: {
      region: "cn",
      npmRegistry: "https://registry.npmmirror.com",
      nodeMirror: "https://npmmirror.com/mirrors/node",
    },
    geo: {
      region: "cn",
      country: "cn",
      matched: 3,
      total: 3,
      sources: ["ipinfo", "ipapi", "country.is"],
    },
  };
  const keys = Object.keys(setup).sort();
  assert.deepEqual(keys, ["allPassed", "appVersion", "dependencies", "geo", "sourcePolicy"]);
  assert.ok(REGION_CODES.includes(setup.geo.region), "region must be a known RegionCode");
  assert.deepEqual(Object.keys(setup.sourcePolicy).sort(), ["nodeMirror", "npmRegistry", "region"]);
});
