/**
 * Fixture 完整性测试：检测/geo fixture 必须保持有效 JSON 且结构稳定，
 * 供 detection-contract 与未来的 geo 契约测试复用。
 *
 * Run: node --test tests/fixtures.test.mjs
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURES = join(ROOT, "tests/fixtures");

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
}

test("fixtures directory exists with expected subdirectories", () => {
  const dirs = readdirSync(FIXTURES);
  for (const expected of ["detection", "geo", "releases"]) {
    assert.ok(dirs.includes(expected), `missing fixtures/${expected}`);
  }
});

test("all JSON fixtures parse and stay structurally sound", () => {
  for (const file of walk(FIXTURES).filter((f) => f.endsWith(".json"))) {
    const parsed = JSON.parse(readFileSync(file, "utf8"));
    assert.ok(parsed && typeof parsed === "object", `${file} must parse to an object`);
  }
});

test("detection fixture covers every dependency id", () => {
  const fixture = JSON.parse(
    readFileSync(join(FIXTURES, "detection/version-outputs.json"), "utf8"),
  );
  for (const id of ["node", "npm", "pnpm", "dsh"]) {
    assert.ok(fixture[id], `missing detection fixture for ${id}`);
  }
});

test("geo fixture covers the three fixed endpoints", () => {
  const fixture = JSON.parse(
    readFileSync(join(FIXTURES, "geo/endpoint-responses.json"), "utf8"),
  );
  for (const endpoint of ["ipinfo", "ipapi", "country.is"]) {
    assert.ok(fixture[endpoint], `missing geo fixture for ${endpoint}`);
  }
});
