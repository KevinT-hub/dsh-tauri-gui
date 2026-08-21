#!/usr/bin/env node
/**
 * Rename Tauri bundle artifacts to the project's release naming convention,
 * rename their `.sig` companions accordingly, and emit a machine-readable
 * `bundle-manifest.json` for the release job to verify against.
 *
 * Usage:
 *   node scripts/rename-bundles.mjs \
 *     --dir <bundle-root> \
 *     --version <X.Y.Z> \
 *     --platform windows|macos|linux \
 *     [--arch x64|aarch64] \
 *     [--target <rust-target>] \
 *     [--dry-run]
 *
 * The bundle root is the Tauri output directory, e.g.
 * `src-tauri/target/<target>/release/bundle`.
 *
 * Strictness guarantees:
 *   - --version must be a valid project SemVer (scripts/lib/semver.mjs).
 *   - Every platform rule must match exactly one artifact; zero or multiple
 *     matches fail the run.
 *   - Existing `.sig` companions are renamed with their artifacts. Installer
 *     signatures produced by a later platform-specific workflow step are
 *     checked by the manifest verification step after that signing completes.
 *   - Target path collisions between different sources fail the run.
 */

import {
  createHash,
} from "node:crypto";
import {
  existsSync,
  readFileSync,
  readdirSync,
  renameSync,
  writeFileSync,
} from "node:fs";
import { dirname, basename, join, relative, resolve } from "node:path";
import { parseVersion } from "./lib/semver.mjs";

function parseArgs(argv) {
  const options = {
    dir: null,
    version: null,
    platform: null,
    arch: null,
    target: null,
    dryRun: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--dir") options.dir = argv[++index];
    else if (arg === "--version") options.version = argv[++index];
    else if (arg === "--platform") options.platform = argv[++index];
    else if (arg === "--arch") options.arch = argv[++index];
    else if (arg === "--target") options.target = argv[++index];
    else if (arg === "--dry-run") options.dryRun = true;
    else if (arg.startsWith("--dir=")) options.dir = arg.slice("--dir=".length);
    else if (arg.startsWith("--version=")) options.version = arg.slice("--version=".length);
    else if (arg.startsWith("--platform=")) options.platform = arg.slice("--platform=".length);
    else if (arg.startsWith("--arch=")) options.arch = arg.slice("--arch=".length);
    else if (arg.startsWith("--target=")) options.target = arg.slice("--target=".length);
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

function sha256File(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function buildRules(options) {
  const version = options.version;
  const rules = [];
  if (options.platform === "windows") {
    rules.push(
      { test: /-setup\.exe$/, name: `dsh-tauri-gui_${version}_Windows_x64-setup.exe`, kind: "installer" },
      { test: /\.msi$/, name: `dsh-tauri-gui_${version}_Windows_x64.msi`, kind: "installer" },
    );
  } else if (options.platform === "macos") {
    const arch = options.arch || "x64";
    rules.push(
      { test: /\.dmg$/, name: `dsh-tauri-gui_${version}_macOS_${arch}.dmg`, kind: "installer" },
      { test: /\.app\.tar\.gz$/, name: `dsh-tauri-gui_${version}_macOS_${arch}.app.tar.gz`, kind: "updater" },
    );
  } else {
    rules.push(
      { test: /\.AppImage$/, name: `dsh-tauri-gui-${version}-Linux-x86_64.AppImage`, kind: "updater" },
      { test: /\.deb$/, name: `dsh-tauri-gui_${version}_Linux_amd64.deb`, kind: "installer" },
      { test: /\.rpm$/, name: `dsh-tauri-gui_${version}_Linux_amd64.rpm`, kind: "installer" },
    );
  }
  return rules;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  // Validate the version up front: reject empty or malformed versions before
  // any filesystem mutation, and reject path-hostile characters by requiring
  // a valid SemVer.
  const parsed = parseVersion(options.version);
  options.version = parsed.version;

  const root = resolve(options.dir);
  if (!existsSync(root)) {
    throw new Error(`bundle dir not found: ${root}`);
  }

  const rules = buildRules(options);
  const files = walkFiles(root).filter((file) => {
    const name = relative(root, file).replace(/\\/g, "/");
    return !/bundle-manifest\.json$/.test(name);
  });

  // Each rule must match exactly one artifact.
  const matchesByRule = rules.map((rule) => ({
    rule,
    sources: files.filter((file) => rule.test.test(relative(root, file).replace(/\\/g, "/"))),
  }));
  for (const { rule, sources } of matchesByRule) {
    if (sources.length === 0) {
      throw new Error(`no artifact matched rule ${rule.name}`);
    }
    if (sources.length > 1) {
      throw new Error(
        `multiple artifacts matched rule ${rule.name}: ${sources.map((file) => relative(root, file)).join(", ")}`,
      );
    }
  }

  const renames = new Map();
  for (const { rule, sources } of matchesByRule) {
    const [from] = sources;
    const to = join(dirname(from), rule.name);
    if (renames.has(from)) {
      throw new Error(`artifact matched multiple rules: ${relative(root, from)}`);
    }
    renames.set(from, to);
  }

  // Attach signatures that Tauri has already produced. macOS DMGs are signed
  // by a dedicated workflow step after this rename step, so their companion
  // may legitimately be created later.
  for (const [from, to] of [...renames]) {
    const sig = `${from}.sig`;
    if (existsSync(sig)) {
      renames.set(sig, `${to}.sig`);
    }
  }

  // Reject target collisions between different sources (idempotent reruns
  // where a source already lives at its target are allowed).
  const targets = new Map();
  for (const [from, to] of renames) {
    const existing = targets.get(to);
    if (existing && existing !== from) {
      throw new Error(`target collision: ${relative(root, to)} (from ${relative(root, existing)} and ${relative(root, from)})`);
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

  // Emit a manifest the publish job can re-verify.
  const manifestEntries = [...renames.entries()]
    .filter(([from]) => !from.endsWith(".sig"))
    .map(([from, to]) => {
      const targetName = relative(root, to).replace(/\\/g, "/");
      const kind = rules.find((rule) => rule.name === basename(targetName))?.kind ?? "installer";
      return {
        artifact: relative(root, to).replace(/\\/g, "/"),
        platform: options.platform,
        target: options.target || options.platform,
        version: options.version,
        kind,
        sha256: options.dryRun ? null : sha256File(to),
        signature: `${relative(root, to).replace(/\\/g, "/")}.sig`,
      };
    })
    .sort((a, b) => a.artifact.localeCompare(b.artifact));

  const manifest = {
    generatedAt: new Date().toISOString(),
    version: options.version,
    platform: options.platform,
    target: options.target || options.platform,
    files: manifestEntries,
  };
  const manifestPath = join(root, "bundle-manifest.json");
  if (!options.dryRun) {
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  }
  console.log(`[rename] manifest written: ${manifestPath}`);
  console.log(JSON.stringify(manifest, null, 2));
}

try {
  main();
} catch (error) {
  console.error(`[rename] FAIL: ${error.message}`);
  process.exit(1);
}
