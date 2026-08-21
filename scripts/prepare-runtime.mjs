#!/usr/bin/env node
/**
 * Prepare the self-contained dsh runtime:
 *
 *   node scripts/prepare-runtime.mjs --dev
 *     Installs Node + @deepseek-ai/dsh + pnpm into ~/.dsh-tauri-gui/runtime
 *     so `tauri dev` behaves exactly like a packaged build.
 *
 *   node scripts/prepare-runtime.mjs --package
 *     Prepares a fresh runtime and packs it into
 *     src-tauri/resources/runtime.tar.gz + runtime.json for the installer.
 *
 *   node scripts/prepare-runtime.mjs --test-resource
 *     Writes explicit compile-only placeholders into src-tauri/resources
 *     (runtime.json marked `testMode: true` and a minimal valid gzip file).
 *     Used by CI for `cargo check`/`cargo test` only; the release workflow
 *     always replaces them with a real packed runtime, and release asserts
 *     the placeholders were not carried over.
 *
 * The packed runtime is generated on the target OS/arch (native modules such
 * as node-pty ship per-platform binaries), so CI builds one installer per
 * platform from this script.
 */

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  createReadStream,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  rmdirSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import * as tar from "tar";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SRC_TAURI_RESOURCES = join(ROOT, "src-tauri", "resources");
const RUNTIME_VERSIONS_PATH = join(ROOT, "scripts", "runtime-versions.json");

const RUNTIME_VERSIONS = JSON.parse(readFileSync(RUNTIME_VERSIONS_PATH, "utf8"));

const NODE_VERSION_SPEC = String(
  process.env.DSH_TAURI_NODE_VERSION ?? RUNTIME_VERSIONS.node,
).trim();
const PNPM_VERSION = process.env.DSH_TAURI_PNPM_VERSION ?? RUNTIME_VERSIONS.pnpm;
const DSH_VERSION_SPEC = String(
  process.env.DSH_TAURI_DSH_VERSION ?? RUNTIME_VERSIONS.dsh,
).trim();
const REGISTRY =
  process.env.DSH_TAURI_NPM_REGISTRY ?? "https://registry.npmmirror.com";
const FALLBACK_REGISTRY = "https://registry.npmjs.org";
const NODE_MIRROR =
  process.env.DSH_TAURI_NODE_MIRROR ?? "https://npmmirror.com/mirrors/node";
const OFFICIAL_NODE_BASE = "https://nodejs.org/dist";
const NPM_FETCH_RETRY_ARGS = [
  "--fetch-retries=5",
  "--fetch-retry-factor=2",
  "--fetch-retry-mintimeout=1000",
  "--fetch-retry-maxtimeout=30000",
];
// npm resolves the dsh dependency graph in the bundled Node process. The
// default V8 heap is too small for that graph on GitHub's macOS runners.
const NPM_NODE_OPTIONS =
  process.env.DSH_TAURI_NPM_NODE_OPTIONS ?? "--max-old-space-size=4096";
const PLATFORM_KEY = `${process.platform}-${process.arch}`;
const PRUNE_DIRS = new Set([
  "examples",
  "example",
  "benchmark",
  "benchmarks",
  "coverage",
  "test",
  "tests",
  "__tests__",
  "spec",
  "specs",
  ".cache",
]);
const PRUNE_FILE = /\.(map|pdb|tsbuildinfo|d\.ts|d\.mts|d\.cts|ts|mts|cts)$/i;
const LICENSE_NAME = /^(license|notice|copying|patents|unlicense)/i;

function fail(message) {
  console.error(`\n[runtime] ${message}`);
  process.exit(1);
}

function spawnResult(command, args, options = {}) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    env: process.env,
    ...options,
  });
  if (result.error) return `failed to run ${command}: ${result.error.message}`;
  if (result.status !== 0) return `${command} exited with code ${result.status}`;
  return null;
}

function run(command, args, options = {}) {
  console.log(`\n[runtime] $ ${command} ${args.join(" ")}`);
  const error = spawnResult(command, args, options);
  if (error) fail(error);
}

/**
 * Remove a file or directory tree. `fs.rmSync` is a no-op on some Windows
 * setups (e.g. when a sandbox or antivirus intercepts it), so fall back to
 * explicit unlink/rmdir traversal that works everywhere.
 */
function removePath(full) {
  try {
    rmSync(full, { recursive: true, force: true });
  } catch {
    // fall through to the manual path
  }
  if (!existsSync(full)) return;
  try {
    const stat = statSync(full);
    if (stat.isDirectory()) {
      for (const entry of readdirSync(full, { withFileTypes: true })) {
        removePath(join(full, entry.name));
      }
      try {
        rmdirSync(full);
      } catch {
        rmSync(full, { recursive: true, force: true });
      }
    } else {
      try {
        unlinkSync(full);
      } catch {
        try {
          chmodSync(full, 0o666);
          unlinkSync(full);
        } catch {
          rmSync(full, { force: true });
        }
      }
    }
  } catch {
    // Best effort; callers treat leftovers as non-fatal.
  }
}

async function fetchJson(url) {
  const response = await fetch(url, {
    headers: { accept: "application/json" },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

function isPackageVersion(value) {
  return /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(String(value));
}

async function resolveNodeVersion() {
  if (/^\d+\.\d+\.\d+$/.test(NODE_VERSION_SPEC)) {
    return NODE_VERSION_SPEC;
  }

  const majorMatch = /^(?:v)?(\d+)(?:\.x)?$/.exec(NODE_VERSION_SPEC);
  if (!majorMatch && NODE_VERSION_SPEC !== "latest") {
    fail(`invalid Node version spec "${NODE_VERSION_SPEC}" (use 22, latest, or x.y.z)`);
  }

  const urls = [
    `${OFFICIAL_NODE_BASE}/index.json`,
    `${NODE_MIRROR}/index.json`,
  ];
  let releases = null;
  let lastError = null;
  for (const url of [...new Set(urls)]) {
    try {
      releases = await fetchJson(url);
      break;
    } catch (error) {
      lastError = `${url}: ${error?.message ?? error}`;
      console.warn(`[runtime] Node release index fetch failed: ${lastError}`);
    }
  }
  if (!Array.isArray(releases)) {
    fail(`cannot resolve Node version from release indexes: ${lastError}`);
  }

  const candidates = releases.filter((release) => {
    const version = String(release?.version ?? "");
    if (!/^v\d+\.\d+\.\d+$/.test(version)) return false;
    if (majorMatch && !version.startsWith(`v${majorMatch[1]}.`)) return false;
    return NODE_VERSION_SPEC === "latest" ? Boolean(release.lts) : true;
  });
  const selected = candidates.find((release) => release.lts) ?? candidates[0];
  if (!selected) {
    fail(`no stable Node release matches "${NODE_VERSION_SPEC}"`);
  }
  const version = String(selected.version).replace(/^v/, "");
  console.log(`[runtime] resolved Node ${NODE_VERSION_SPEC} -> ${version}`);
  return version;
}

function npmDistTagsUrl(registry, packageName) {
  const encodedName = packageName.replace("/", "%2f");
  return `${registry.replace(/\/+$/, "")}/-/package/${encodedName}/dist-tags`;
}

async function resolveDshVersion() {
  if (isPackageVersion(DSH_VERSION_SPEC)) return DSH_VERSION_SPEC;
  if (!/^[A-Za-z][A-Za-z0-9._-]*$/.test(DSH_VERSION_SPEC)) {
    fail(`invalid dsh version spec "${DSH_VERSION_SPEC}"`);
  }

  let lastError = null;
  for (const registry of npmRegistries()) {
    try {
      const tags = await fetchJson(
        npmDistTagsUrl(registry, "@deepseek-ai/dsh"),
      );
      const version = tags?.[DSH_VERSION_SPEC];
      if (!isPackageVersion(version)) {
        throw new Error(`dist-tag ${DSH_VERSION_SPEC} is not a package version`);
      }
      console.log(
        `[runtime] resolved @deepseek-ai/dsh ${DSH_VERSION_SPEC} -> ${version}`,
      );
      return version;
    } catch (error) {
      lastError = `${registry}: ${error?.message ?? error}`;
      console.warn(`[runtime] dsh dist-tag fetch failed: ${lastError}`);
    }
  }
  fail(`cannot resolve @deepseek-ai/dsh@${DSH_VERSION_SPEC}: ${lastError}`);
}

function nodeDistInfo(nodeVersion) {
  const platform = process.platform;
  const arch = process.arch;
  const map = {
    "win32-x64": { dir: "win-x64", file: "zip" },
    "win32-arm64": { dir: "win-arm64", file: "zip" },
    "darwin-x64": { dir: "darwin-x64", file: "tar.gz" },
    "darwin-arm64": { dir: "darwin-arm64", file: "tar.gz" },
    "linux-x64": { dir: "linux-x64", file: "tar.gz" },
    "linux-arm64": { dir: "linux-arm64", file: "tar.gz" },
  };
  const info = map[`${platform}-${arch}`];
  if (!info) fail(`unsupported platform/arch: ${platform}/${arch}`);
  return {
    version: nodeVersion,
    ...info,
    base: `node-v${nodeVersion}-${info.dir}`,
    url: `${NODE_MIRROR}/v${nodeVersion}/node-v${nodeVersion}-${info.dir}.${info.file}`,
  };
}

function defaultTarget() {
  const home =
    process.env[process.platform === "win32" ? "USERPROFILE" : "HOME"] ??
    process.env.HOME ??
    ".";
  return join(home, ".dsh-tauri-gui", "runtime");
}

function parseArgs(argv) {
  const options = {
    dev: false,
    package: false,
    prune: false,
    testResource: false,
    target: null,
    keep: false,
    refresh: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--dev") options.dev = true;
    else if (arg === "--package") options.package = true;
    else if (arg === "--prune") options.prune = true;
    else if (arg === "--test-resource") options.testResource = true;
    else if (arg === "--keep") options.keep = true;
    else if (arg === "--refresh") options.refresh = true;
    else if (arg === "--target") {
      const value = argv[++index];
      if (!value) fail("--target requires a path");
      options.target = resolve(value);
    } else if (arg.startsWith("--target=")) {
      options.target = resolve(arg.slice("--target=".length));
    } else if (arg === "--") {
      // npm/pnpm `run` may forward the literal `--` separator; ignore it.
    } else {
      fail(`unknown argument: ${arg}`);
    }
  }
  if ([options.dev, options.package, options.prune, options.testResource].filter(Boolean).length > 1) {
    fail("--dev, --package, --prune and --test-resource are mutually exclusive");
  }
  if (options.package) {
    options.target ??= join(tmpdir(), `dsh-runtime-prepare-${process.pid}`);
  }
  if (options.prune && !options.target) {
    fail("--prune requires --target <runtime dir>");
  }
  return options;
}

/**
 * Compile-only resource placeholders for CI (cargo check / cargo test).
 * These must never reach a release: the release workflow always runs
 * `runtime:package` and additionally asserts `runtime.json` has no
 * `testMode` marker before building installers.
 */
function writeTestResources() {
  mkdirSync(SRC_TAURI_RESOURCES, { recursive: true });
  const manifest = {
    testMode: true,
    createdAt: new Date().toISOString(),
    note: "compile-only placeholder; replaced by runtime:package in release builds",
  };
  writeFileSync(
    join(SRC_TAURI_RESOURCES, "runtime.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  // A minimal valid gzip stream (empty tar payload), still a real gzip file.
  writeFileSync(
    join(SRC_TAURI_RESOURCES, "runtime.tar.gz"),
    gzipSync(Buffer.alloc(0)),
  );
  console.log(
    `[runtime] wrote compile-only placeholders to ${SRC_TAURI_RESOURCES} (testMode=true)`,
  );
}

function extractArchive(archivePath, extractDir) {
  const archiveName = basename(archivePath);
  const archiveCwd = dirname(archivePath);
  if (archivePath.endsWith(".zip")) {
    if (process.platform === "win32") {
      const quote = (value) => `'${value.replaceAll("'", "''")}'`;
      const command = `Expand-Archive -LiteralPath ${quote(archivePath)} -DestinationPath ${quote(extractDir)} -Force`;
      run("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", command]);
    } else {
      run("tar", ["-xf", archiveName, "-C", extractDir], { cwd: archiveCwd });
    }
  } else {
    run("tar", ["-xzf", archiveName, "-C", extractDir], { cwd: archiveCwd });
  }
}

function nodeArchiveUrls(info) {
  const name = `${info.base}.${info.file}`;
  return [
    ...new Set([
      `${NODE_MIRROR}/v${info.version}/${name}`,
      `${OFFICIAL_NODE_BASE}/v${info.version}/${name}`,
    ]),
  ];
}

function sha256File(file) {
  return new Promise((resolvePromise, reject) => {
    const hash = createHash("sha256");
    const stream = createReadStream(file);
    stream.on("error", reject);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("end", () => resolvePromise(hash.digest("hex")));
  });
}

async function fetchChecksumUrl(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(20000) });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.text();
}

async function expectedNodeChecksum(info) {
  const fileName = `${info.base}.${info.file}`;
  const urls = [
    ...new Set([
      `${OFFICIAL_NODE_BASE}/v${info.version}/SHASUMS256.txt`,
      `${NODE_MIRROR}/v${info.version}/SHASUMS256.txt`,
    ]),
  ];
  for (const url of urls) {
    try {
      const text = await fetchChecksumUrl(url);
      const line = text
        .split(/\r?\n/)
        .find((candidate) => candidate.includes(`  ${fileName}`) || candidate.includes(` *${fileName}`));
      if (line) {
        const hash = line.trim().split(/\s+/)[0];
        if (/^[0-9a-f]{64}$/i.test(hash)) return hash;
      }
    } catch (error) {
      console.warn(`[runtime] checksum fetch failed (${url}): ${error?.message ?? error}`);
    }
  }
  fail(`cannot obtain SHASUMS256.txt for node ${info.base}`);
  return "";
}

async function verifyNodeArchive(archivePath, info) {
  if (process.env.DSH_TAURI_SKIP_CHECKSUM === "1") {
    console.warn("[runtime] SKIPPING node archive checksum (DSH_TAURI_SKIP_CHECKSUM=1)");
    return;
  }
  const expected = await expectedNodeChecksum(info);
  const actual = await sha256File(archivePath);
  if (actual.toLowerCase() !== expected.toLowerCase()) {
    fail(`node archive checksum mismatch: expected ${expected}, got ${actual}`);
  }
  console.log(`[runtime] node archive checksum OK (${expected.slice(0, 12)}...)`);
}

async function download(urls, dest, label) {
  if (existsSync(dest)) {
    console.log(`[runtime] cache hit: ${dest}`);
    return;
  }
  mkdirSync(dirname(dest), { recursive: true });
  const tmp = `${dest}.part`;
  let lastError = null;
  for (const url of urls) {
    console.log(`[runtime] downloading ${label} from ${url}`);
    const error = spawnResult("curl", [
      "-#",
      "-L",
      "--fail",
      "--retry",
      "5",
      "--retry-delay",
      "2",
      "--retry-max-time",
      "120",
      "--retry-connrefused",
      "--connect-timeout",
      "20",
      "-o",
      tmp,
      url,
    ]);
    if (!error) {
      removePath(dest);
      const renameError = spawnResult("node", [
        "-e",
        "require('fs').renameSync(process.argv[1], process.argv[2])",
        tmp,
        dest,
      ]);
      if (!renameError) return;
      lastError = renameError;
    } else {
      lastError = error;
    }
  }
  removePath(tmp);
  fail(`failed to download ${label}: ${lastError}`);
}

async function prepareNode(target, cacheDir, nodeVersion) {
  const info = nodeDistInfo(nodeVersion);
  const nodeRoot = join(target, "node");
  if (existsSync(nodeBinary(nodeRoot))) {
    const existingVersion = readNodeVersion(nodeRoot);
    if (existingVersion === nodeVersion) {
      console.log(`[runtime] node ${existingVersion} already prepared at ${nodeRoot}`);
      return nodeRoot;
    }
    console.log(
      `[runtime] replacing node ${existingVersion} with requested ${nodeVersion}`,
    );
    removePath(nodeRoot);
  }
  const cachePath = join(cacheDir, `${info.base}.${info.file}`);
  await download(nodeArchiveUrls(info), cachePath, info.base);
  await verifyNodeArchive(cachePath, info);
  const extractDir = join(target, ".node-extract");
  removePath(extractDir);
  mkdirSync(extractDir, { recursive: true });
  extractArchive(cachePath, extractDir);
  const extracted = join(extractDir, info.base);
  if (!existsSync(extracted)) {
    fail(`node archive did not contain ${info.base}`);
  }
  removePath(nodeRoot);
  run("node", [
    "-e",
    "require('fs').renameSync(process.argv[1], process.argv[2])",
    extracted,
    nodeRoot,
  ]);
  removePath(extractDir);
  return nodeRoot;
}

function npmCli(nodeRoot) {
  const candidates =
    process.platform === "win32"
      ? [join(nodeRoot, "node_modules", "npm", "bin", "npm-cli.js")]
      : [
          join(nodeRoot, "lib", "node_modules", "npm", "bin", "npm-cli.js"),
          join(nodeRoot, "node_modules", "npm", "bin", "npm-cli.js"),
        ];
  const found = candidates.find((candidate) => existsSync(candidate));
  if (found) return found;
  // Return the platform-default candidate so callers surface a clear error.
  return candidates[0];
}

function npmRegistries() {
  return REGISTRY === FALLBACK_REGISTRY ? [REGISTRY] : [REGISTRY, FALLBACK_REGISTRY];
}

function nodeBinary(nodeRoot) {
  return process.platform === "win32"
    ? join(nodeRoot, "node.exe")
    : join(nodeRoot, "bin", "node");
}

function installDsh(nodeRoot, target, dshVersion, refresh) {
  const appDir = join(target, "app");
  const dshPackage = join(
    appDir,
    "node_modules",
    "@deepseek-ai",
    "dsh",
    "package.json",
  );
  if (!refresh && existsSync(dshPackage)) {
    const installedVersion = readVersion(dshPackage);
    if (installedVersion === dshVersion) {
      console.log(
        `[runtime] @deepseek-ai/dsh ${installedVersion} already prepared at ${appDir}`,
      );
      return;
    }
    console.log(
      `[runtime] replacing @deepseek-ai/dsh ${installedVersion} with ${dshVersion}`,
    );
    removePath(appDir);
  }
  if (!existsSync(appDir)) {
    mkdirSync(appDir, { recursive: true });
  }
  const npmArgs = [
    npmCli(nodeRoot),
    "install",
    "--prefix",
    appDir,
    `@deepseek-ai/dsh@${dshVersion}`,
    ...NPM_FETCH_RETRY_ARGS,
    "--registry",
    REGISTRY,
    "--no-audit",
    "--no-fund",
    "--no-update-notifier",
    "--loglevel=error",
  ];
  installWithRegistryFallback(nodeBinary(nodeRoot), npmArgs, { cwd: appDir }, "@deepseek-ai/dsh");
}

function installPnpm(nodeRoot, target, pnpmVersion, refresh) {
  const toolsDir = join(target, "tools");
  const pnpmPackage = join(toolsDir, "node_modules", "pnpm", "package.json");
  if (!refresh && existsSync(pnpmPackage)) {
    const installedVersion = readVersion(pnpmPackage);
    if (installedVersion === pnpmVersion) {
      console.log(`[runtime] pnpm ${installedVersion} already prepared at ${toolsDir}`);
      return;
    }
    console.log(
      `[runtime] replacing pnpm ${installedVersion} with ${pnpmVersion}`,
    );
    removePath(toolsDir);
  }
  if (!existsSync(toolsDir)) {
    mkdirSync(toolsDir, { recursive: true });
  }
  const npmArgs = [
    npmCli(nodeRoot),
    "install",
    "--prefix",
    toolsDir,
    `pnpm@${pnpmVersion}`,
    ...NPM_FETCH_RETRY_ARGS,
    "--registry",
    REGISTRY,
    "--no-audit",
    "--no-fund",
    "--no-update-notifier",
    "--loglevel=error",
  ];
  installWithRegistryFallback(nodeBinary(nodeRoot), npmArgs, { cwd: toolsDir }, "pnpm");
}

function installWithRegistryFallback(nodeBinaryPath, npmArgs, options, label) {
  let lastError = null;
  for (const registry of npmRegistries()) {
    const args = [...npmArgs];
    const registryIndex = args.indexOf("--registry");
    if (registryIndex >= 0) args.splice(registryIndex, 2);
    args.push("--registry", registry);
    const error = spawnResult(nodeBinaryPath, args, {
      ...options,
      env: {
        ...process.env,
        ...(options.env ?? {}),
        NODE_OPTIONS: options.env?.NODE_OPTIONS ?? NPM_NODE_OPTIONS,
      },
    });
    if (!error) return;
    lastError = `${registry}: ${error}`;
    console.warn(`[runtime] ${label} install failed via ${registry}: ${error}`);
  }
  fail(`${label} install failed through all registries: ${lastError}`);
}

function pruneNodePty(packageDir) {
  const prebuilds = join(packageDir, "prebuilds");
  let hasCurrentPrebuild = false;
  if (existsSync(prebuilds)) {
    for (const entry of readdirSync(prebuilds, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      if (entry.name !== PLATFORM_KEY) {
        removePath(join(prebuilds, entry.name));
      } else if (
        readdirSync(join(prebuilds, entry.name)).some((name) =>
          name.endsWith(".node"),
        )
      ) {
        hasCurrentPrebuild = true;
      }
    }
  }
  const buildDir = join(packageDir, "build");
  if (existsSync(buildDir)) {
    if (hasCurrentPrebuild) {
      removePath(buildDir);
    } else {
      // Some platforms (e.g. Linux without a usable node-pty prebuild)
      // compile the native binary into build/Release instead. Keep only the
      // final .node output and drop the rest of the build tree.
      for (const entry of readdirSync(buildDir, { withFileTypes: true })) {
        if (entry.name !== "Release") {
          removePath(join(buildDir, entry.name));
        }
      }
      const releaseDir = join(buildDir, "Release");
      if (existsSync(releaseDir)) {
        for (const entry of readdirSync(releaseDir, { withFileTypes: true })) {
          if (entry.isDirectory() || !entry.name.endsWith(".node")) {
            removePath(join(releaseDir, entry.name));
          }
        }
      }
    }
  }
  for (const name of ["deps", "third_party", "src", "scripts", "typings"]) {
    removePath(join(packageDir, name));
  }
}

function pruneImg(scopeDir) {
  if (!existsSync(scopeDir)) return;
  for (const entry of readdirSync(scopeDir, { withFileTypes: true })) {
    if (entry.isDirectory() && entry.name === "sharp-wasm32") {
      removePath(join(scopeDir, entry.name));
    }
  }
}

function pruneNodeModules(root) {
  if (!existsSync(root)) return;
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const full = join(root, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "@types") {
        removePath(full);
        continue;
      }
      if (PRUNE_DIRS.has(entry.name)) {
        removePath(full);
        continue;
      }
      if (entry.name === "node-pty") {
        pruneNodePty(full);
        continue;
      }
      if (entry.name === "@img") {
        pruneImg(full);
        continue;
      }
      pruneNodeModules(full);
    } else if (entry.isFile()) {
      const lower = entry.name.toLowerCase();
      if (PRUNE_FILE.test(entry.name)) {
        removePath(full);
        continue;
      }
      if (lower.endsWith(".md") && !LICENSE_NAME.test(lower)) {
        removePath(full);
        continue;
      }
      if (/^(readme|changelog|history|authors|contributing|security|code_of_conduct)/i.test(lower)) {
        removePath(full);
      }
    }
  }
}

function pruneRuntime(target) {
  console.log("[runtime] pruning non-essential files (docs/maps/types/debug symbols/foreign prebuilds)");
  const started = Date.now();
  pruneNodeModules(join(target, "app", "node_modules"));
  pruneNodeModules(join(target, "tools", "node_modules"));
  pruneNodeModules(join(target, "node", "node_modules"));
  for (const name of [
    "README.md",
    "CHANGELOG.md",
    "install_tools.bat",
    "corepack",
    "corepack.cmd",
    "corepack.ps1",
    "node_modules/corepack",
    "node_modules/npm/docs",
    "node_modules/npm/man",
    "node_modules/npm/html",
    "node_modules/npm/changelogs",
    "node_modules/npm/test",
    "node_modules/npm/tests",
  ]) {
    removePath(join(target, "node", name));
  }
  // These lockfiles describe the build-time install and are never read by
  // the desktop engine or its staging updater.
  removePath(join(target, "app", "package-lock.json"));
  removePath(join(target, "tools", "package-lock.json"));
  const seconds = ((Date.now() - started) / 1000).toFixed(1);
  console.log(`[runtime] prune finished in ${seconds}s`);
}

function readVersion(manifestPath) {
  try {
    return JSON.parse(readFileSync(manifestPath, "utf8")).version ?? "unknown";
  } catch {
    return "unknown";
  }
}

function readNodeVersion(nodeRoot) {
  const result = spawnSync(nodeBinary(nodeRoot), ["--version"], {
    encoding: "utf8",
  });
  return result.status === 0 ? result.stdout.trim().replace(/^v/, "") : "unknown";
}

function writeManifest(target) {
  const manifest = {
    dshVersion: readVersion(
      join(target, "app", "node_modules", "@deepseek-ai", "dsh", "package.json"),
    ),
    nodeVersion: readNodeVersion(join(target, "node")),
    pnpmVersion: readVersion(
      join(target, "tools", "node_modules", "pnpm", "package.json"),
    ),
    createdAt: new Date().toISOString(),
  };
  writeFileSync(
    join(target, "runtime.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  return manifest;
}

async function packageRuntime(target) {
  const manifest = JSON.parse(readFileSync(join(target, "runtime.json"), "utf8"));
  mkdirSync(SRC_TAURI_RESOURCES, { recursive: true });
  const archivePath = join(SRC_TAURI_RESOURCES, "runtime.tar.gz");
  removePath(archivePath);
  console.log(`[runtime] packing ${target} -> ${archivePath}`);
  await tar.c(
    {
      cwd: target,
      file: archivePath,
      gzip: { level: 9 },
      portable: true,
      noMtime: true,
      filter: (path) =>
        !path.includes(".node-extract") &&
        !path.startsWith(".runtime-") &&
        !path.includes(".cache"),
    },
    ["node", "app", "tools", "runtime.json"],
  );
  writeFileSync(
    join(SRC_TAURI_RESOURCES, "runtime.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  const sizeMb = (
    (await import("node:fs")).statSync(archivePath).size /
    1024 /
    1024
  ).toFixed(1);
  console.log(`\n[runtime] packed ${archivePath} (${sizeMb} MB)`);
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.testResource) {
    writeTestResources();
    return;
  }
  const target = options.target ?? defaultTarget();
  if (options.prune) {
    pruneRuntime(target);
    return;
  }
  const nodeVersion = await resolveNodeVersion();
  const dshVersion = await resolveDshVersion();
  mkdirSync(target, { recursive: true });
  const cacheDir = join(
    process.env.DSH_TAURI_NODE_CACHE ??
      join(
        process.env[process.platform === "win32" ? "USERPROFILE" : "HOME"] ??
          ".",
        ".cache",
        "dsh-tauri-gui",
      ),
  );
  const nodeRoot = await prepareNode(target, cacheDir, nodeVersion);
  installDsh(nodeRoot, target, dshVersion, options.refresh);
  installPnpm(nodeRoot, target, PNPM_VERSION, options.refresh);
  pruneRuntime(target);
  const manifest = writeManifest(target);
  console.log(
    `\n[runtime] ready at ${target}\n` +
      `  node  v${manifest.nodeVersion}\n` +
      `  dsh   v${manifest.dshVersion}\n` +
      `  pnpm  v${manifest.pnpmVersion}`,
  );
  if (options.package) {
    await packageRuntime(target);
    if (!options.keep && !options.target) {
      removePath(target);
    }
  }
}

main().catch((error) => fail(error.stack ?? String(error)));
