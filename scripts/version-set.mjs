#!/usr/bin/env node
/**
 * Set the project version across all four version sources:
 *   package.json / src-tauri/Cargo.toml / src-tauri/tauri.conf.json / src-tauri/Cargo.lock
 *
 * Usage:
 *   node scripts/version-set.mjs <new-version> [--dry-run]
 *
 * <new-version> may be `1.2.3`, `v1.2.3`, `1.2.3-rc.1`, `1.2.3+build.2` ...
 * The stored version always drops the `v` prefix (files hold X.Y.Z[-pre][+b]).
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseVersion } from "./lib/semver.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function updateJsonVersion(file, version) {
  const original = readFileSync(file, "utf8");
  const replaced = original.replace(
    /^(\s*"version"\s*:\s*)"[^"]*"(\s*,?\s*)$/m,
    (_match, prefix, suffix) => `${prefix}"${version}"${suffix}`,
  );
  if (replaced === original) {
    throw new Error(`cannot locate top-level "version" in ${file}`);
  }
  return replaced;
}

function updateCargoTomlVersion(file, version) {
  const original = readFileSync(file, "utf8");
  const lines = original.split(/\r?\n/);
  let inPackage = false;
  for (let index = 0; index < lines.length; index += 1) {
    const trimmed = lines[index].trim();
    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      inPackage = trimmed === "[package]";
      continue;
    }
    if (inPackage && /^version\s*=\s*"/.test(trimmed)) {
      lines[index] = lines[index].replace(
        /^(version\s*=\s*)"[^"]*"/,
        `$1"${version}"`,
      );
      return lines.join("\n");
    }
  }
  throw new Error(`cannot locate [package] version in ${file}`);
}

function updateCargoLockVersion(file, version) {
  const original = readFileSync(file, "utf8");
  const pattern = new RegExp(
    String.raw`(\[\[package\]\]\r?\nname = "dsh-tauri-gui"\r?\nversion = ")[^"]*(")`,
  );
  if (!pattern.test(original)) {
    throw new Error(`cannot locate "dsh-tauri-gui" package in ${file}`);
  }
  return original.replace(
    pattern,
    (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
  );
}

function main(argv = process.argv.slice(2)) {
  const args = [];
  let dryRun = false;
  for (const arg of argv) {
    if (arg === "--dry-run") dryRun = true;
    else args.push(arg);
  }
  if (args.length !== 1) {
    throw new Error("usage: version-set <new-version> [--dry-run]");
  }
  const version = parseVersion(args[0]).version;

  const files = [
    join(ROOT, "package.json"),
    join(ROOT, "src-tauri", "tauri.conf.json"),
    join(ROOT, "src-tauri", "Cargo.toml"),
    join(ROOT, "src-tauri", "Cargo.lock"),
  ];

  for (const file of files) {
    const next = file.endsWith("Cargo.toml")
      ? updateCargoTomlVersion(file, version)
      : file.endsWith("Cargo.lock")
        ? updateCargoLockVersion(file, version)
        : updateJsonVersion(file, version);
    if (!dryRun) writeFileSync(file, next);
    console.log(
      `[version-set] ${dryRun ? "would update" : "updated"} ${resolve(file)} -> ${version}`,
    );
  }

  console.log(`\n[version-set] ${dryRun ? "dry run:" : "done:"} version is now ${version}`);
}

try {
  main();
} catch (error) {
  console.error(`[version-set] ${error.message}`);
  process.exit(1);
}
