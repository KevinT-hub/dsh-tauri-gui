#!/usr/bin/env node
/**
 * Rename Tauri bundle artifacts to the project's release naming convention
 * and rename their `.sig` companions accordingly.
 *
 * Usage:
 *   node scripts/rename-bundles.mjs \
 *     --dir <bundle-root> \
 *     --version <X.Y.Z> \
 *     --platform windows|macos|linux \
 *     [--arch x64|aarch64] \
 *     [--dry-run]
 *
 * The bundle root is the Tauri output directory, e.g.
 * `src-tauri/target/<target>/release/bundle`.
 */

import { existsSync, readdirSync, renameSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

function parseArgs(argv) {
  const options = {
    dir: null,
    version: null,
    platform: null,
    arch: null,
    dryRun: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--dir") options.dir = argv[++index];
    else if (arg === "--version") options.version = argv[++index];
    else if (arg === "--platform") options.platform = argv[++index];
    else if (arg === "--arch") options.arch = argv[++index];
    else if (arg === "--dry-run") options.dryRun = true;
    else if (arg.startsWith("--dir=")) options.dir = arg.slice("--dir=".length);
    else if (arg.startsWith("--version=")) options.version = arg.slice("--version=".length);
    else if (arg.startsWith("--platform=")) options.platform = arg.slice("--platform=".length);
    else if (arg.startsWith("--arch=")) options.arch = arg.slice("--arch=".length);
    else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!options.dir || !options.version || !options.platform) {
    throw new Error("--dir, --version and --platform are required");
  }
  if (!["windows", "macos", "linux"].includes(options.platform)) {
    throw new Error(`unsupported platform: ${options.platform}`);
  }
  return options;
}

function walkFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkFiles(full));
    } else if (entry.isFile()) {
      files.push(full);
    }
  }
  return files;
}

function buildRules(options) {
  const version = options.version;
  const rules = [];
  if (options.platform === "windows") {
    rules.push({
      test: /-setup\.exe$/,
      name: `dsh-tauri-gui_${version}_Windows_x64-setup.exe`,
    });
    rules.push({
      test: /\.msi$/,
      name: `dsh-tauri-gui_${version}_Windows_x64.msi`,
    });
  } else if (options.platform === "macos") {
    const arch = options.arch || "x64";
    rules.push({
      test: /\.dmg$/,
      name: `dsh-tauri-gui_${version}_macOS_${arch}.dmg`,
    });
    rules.push({
      test: /\.app\.tar\.gz$/,
      name: `dsh-tauri-gui_${version}_macOS_${arch}.app.tar.gz`,
    });
  } else {
    rules.push({
      test: /\.AppImage$/,
      name: `dsh-tauri-gui-${version}-Linux-x86_64.AppImage`,
    });
    rules.push({
      test: /\.deb$/,
      name: `dsh-tauri-gui_${version}_Linux_amd64.deb`,
    });
    rules.push({
      test: /\.rpm$/,
      name: `dsh-tauri-gui_${version}_Linux_amd64.rpm`,
    });
  }
  return rules;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const root = resolve(options.dir);
  if (!existsSync(root)) {
    throw new Error(`bundle dir not found: ${root}`);
  }

  const rules = buildRules(options);
  const renames = new Map();
  for (const file of walkFiles(root)) {
    const rel = relative(root, file).replace(/\\/g, "/");
    for (const rule of rules) {
      if (rule.test.test(rel)) {
        if (renames.has(file)) {
          throw new Error(`multiple rules matched ${rel}`);
        }
        renames.set(file, join(dirname(file), rule.name));
        break;
      }
    }
  }

  // Attach `.sig` companions (updater signatures) to their artifacts.
  for (const [from, to] of [...renames]) {
    const sig = `${from}.sig`;
    if (existsSync(sig)) {
      renames.set(sig, `${to}.sig`);
    }
  }

  const targets = new Map();
  for (const [from, to] of renames) {
    if (targets.has(to) && targets.get(to) !== from) {
      throw new Error(`target collision: ${to}`);
    }
    targets.set(to, from);
  }

  const sorted = [...renames.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  for (const [from, to] of sorted) {
    const relFrom = relative(root, from).replace(/\\/g, "/");
    const relTo = relative(root, to).replace(/\\/g, "/");
    console.log(`[rename] ${relFrom} -> ${relTo}`);
    if (!options.dryRun && from !== to) {
      renameSync(from, to);
    }
  }
  console.log(`[rename] ${sorted.length} file(s) processed (${options.dryRun ? "dry run" : "done"})`);
}

try {
  main();
} catch (error) {
  console.error(`[rename] FAIL: ${error.message}`);
  process.exit(1);
}
