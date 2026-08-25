/**
 * 发行物禁止携带 runtime 资源的契约测试。
 *
 * 验证仓库状态满足「不内置运行时」约束：
 * 1. `src-tauri/resources/` 不含 runtime / Node / pnpm / dsh 文件；
 * 2. `tauri.conf.json` 不声明 runtime resources；
 * 3. `package.json` / `Cargo.toml` 不含 runtime 脚本或解压依赖；
 * 4. `scripts/` 不含 prepare/smoke-runtime 脚本；
 * 5. 源码不含 runtime 热更新模块。
 *
 * Run: node --test tests/release-no-runtime.test.mjs
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

const RUNTIME_PATTERNS = /runtime\.tar|runtime\.json|prepare-runtime|smoke-runtime|runtimeMode|runtime_update|bundled/i;

function read(relativePath) {
  const full = join(ROOT, relativePath);
  if (!existsSync(full)) return null;
  return readFileSync(full, "utf8");
}

function walk(dir) {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...walk(full));
    } else {
      out.push(full);
    }
  }
  return out;
}

test("src-tauri/resources contains no runtime payload", () => {
  const dir = join(ROOT, "src-tauri/resources");
  const files = walk(dir).map((p) => p.replace(/\\/g, "/"));
  for (const file of files) {
    assert.doesNotMatch(file, /runtime|node|pnpm|dsh/i, `runtime payload found: ${file}`);
  }
});

test("tauri.conf.json declares no runtime resources", () => {
  const config = JSON.parse(read("src-tauri/tauri.conf.json"));
  const resources = config.bundle?.resources ?? [];
  for (const resource of resources) {
    assert.doesNotMatch(resource, /runtime|node|pnpm|dsh/i, `runtime resource declared: ${resource}`);
  }
});

test("package.json has no runtime scripts and no tar tooling", () => {
  const pkg = JSON.parse(read("package.json"));
  const scripts = Object.keys(pkg.scripts ?? {});
  for (const name of scripts) {
    assert.doesNotMatch(name, /runtime/, `runtime script still present: ${name}`);
  }
  const devDeps = Object.keys(pkg.devDependencies ?? {});
  assert.ok(!devDeps.includes("tar"), "tar devDependency should be removed");
});

test("Cargo.toml has no tar/flate2 runtime-extraction deps", () => {
  const cargo = read("src-tauri/Cargo.toml");
  assert.doesNotMatch(cargo, /^tar\s*=|^flate2\s*=/m, "tar/flate2 must not be dependencies");
});

test("no runtime scripts remain under scripts/", () => {
  const files = walk(join(ROOT, "scripts")).map((p) => p.replace(/\\/g, "/"));
  for (const file of files) {
    assert.doesNotMatch(file, /prepare-runtime|smoke-runtime|runtime-versions/i);
  }
});

test("source tree has no runtime hot-update module", () => {
  const files = walk(join(ROOT, "src-tauri/src")).map((p) => p.replace(/\\/g, "/"));
  for (const file of files) {
    assert.doesNotMatch(file, /runtime_update|commands\/runtime/i);
    assert.doesNotMatch(
      file,
      /(?:^|\/)commands\/bootstrap\.rs$/i,
      "legacy command bootstrap must be gone",
    );
  }
});

test("no bundled-runtime wording leaks into the detection contract", () => {
  const types = read("src/shared/types.ts");
  assert.doesNotMatch(types, /RuntimeMode|runtimeModeSelected/i);
});
