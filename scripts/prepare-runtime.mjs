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
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as tar from "tar";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SRC_TAURI_RESOURCES = join(ROOT, "src-tauri", "resources");
const RUNTIME_VERSIONS_PATH = join(ROOT, "scripts", "runtime-versions.json");

const RUNTIME_VERSIONS = JSON.parse(readFileSync(RUNTIME_VERSIONS_PATH, "utf8"));

const NODE_VERSION = process.env.DSH_TAURI_NODE_VERSION ?? RUNTIME_VERSIONS.node;
const PNPM_VERSION = process.env.DSH_TAURI_PNPM_VERSION ?? RUNTIME_VERSIONS.pnpm;
const DSH_VERSION = process.env.DSH_TAURI_DSH_VERSION ?? RUNTIME_VERSIONS.dsh;
const REGISTRY =
  process.env.DSH_TAURI_NPM_REGISTRY ?? "https://registry.npmmirror.com";
const FALLBACK_REGISTRY = "https://registry.npmjs.org";
const NODE_MIRROR =
  process.env.DSH_TAURI_NODE_MIRROR ?? "https://npmmirror.com/mirrors/node";
const OFFICIAL_NODE_BASE = "https://nodejs.org/dist";
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

function nodeDistInfo() {
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
    ...info,
    base: `node-v${NODE_VERSION}-${info.dir}`,
    url: `${NODE_MIRROR}/v${NODE_VERSION}/node-v${NODE_VERSION}-${info.dir}.${info.file}`,
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
    target: null,
    keep: false,
    refresh: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--dev") options.dev = true;
    else if (arg === "--package") options.package = true;
    else if (arg === "--prune") options.prune = true;
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
  if ([options.dev, options.package, options.prune].filter(Boolean).length > 1) {
    fail("--dev, --package and --prune are mutually exclusive");
  }
  if (options.package) {
    options.target ??= join(tmpdir(), `dsh-runtime-prepare-${process.pid}`);
  }
  if (options.prune && !options.target) {
    fail("--prune requires --target <runtime dir>");
  }
  return options;
}

function extractArchive(archivePath, extractDir) {
  if (archivePath.endsWith(".zip")) {
    // Windows ships bsdtar which reads zip archives natively; passing paths
    // as argv avoids shell-quoting issues entirely.
    run("tar", ["-xf", archivePath, "-C", extractDir]);
  } else {
    run("tar", ["-xzf", archivePath, "-C", extractDir]);
  }
}

function nodeArchiveUrls(info) {
  const name = `${info.base}.${info.file}`;
  return [
    `${NODE_MIRROR}/v${NODE_VERSION}/${name}`,
    `${OFFICIAL_NODE_BASE}/v${NODE_VERSION}/${name}`,
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
    `${OFFICIAL_NODE_BASE}/v${NODE_VERSION}/SHASUMS256.txt`,
    `${NODE_MIRROR}/v${NODE_VERSION}/SHASUMS256.txt`,
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
  fail(`cannot obtain SHASUMS256.txt for node v${NODE_VERSION}`);
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

async function prepareNode(target, cacheDir) {
  const info = nodeDistInfo();
  const nodeRoot = join(target, "node");
  if (existsSync(nodeBinary(nodeRoot))) {
    console.log(`[runtime] node already prepared at ${nodeRoot}`);
    return nodeRoot;
  }
  const cachePath = join(cacheDir, `${info.base}.${info.file}`);
  await download(nodeArchiveUrls(info), cachePath, info.base);
  await verifyNodeArchive(cachePath, info);
  const extractDir = join(target, ".node-extract");
  removePath(extractDir);
  mkdirSync(extractDir, { recursive: true });
  extractArchive(cachePath, extractDir);
  const extracted = join(extractDir, info.base);
  if (!existsSync(extracted)) fail(`node archive did not contain ${info.base}`);
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

function installDsh(nodeRoot, target, refresh) {
  const appDir = join(target, "app");
  if (
    !refresh &&
    existsSync(join(appDir, "node_modules", "@deepseek-ai", "dsh", "package.json"))
  ) {
    console.log(`[runtime] @deepseek-ai/dsh already prepared at ${appDir}`);
    return;
  }
  mkdirSync(appDir, { recursive: true });
  const npmArgs = [
    npmCli(nodeRoot),
    "install",
    "--prefix",
    appDir,
    `@deepseek-ai/dsh@${DSH_VERSION}`,
    "--registry",
    REGISTRY,
    "--no-audit",
    "--no-fund",
    "--no-update-notifier",
    "--loglevel=error",
  ];
  installWithRegistryFallback(nodeBinary(nodeRoot), npmArgs, { cwd: appDir }, "@deepseek-ai/dsh");
}

function installPnpm(nodeRoot, target, refresh) {
  const toolsDir = join(target, "tools");
  if (
    !refresh &&
    existsSync(join(toolsDir, "node_modules", "pnpm", "package.json"))
  ) {
    console.log(`[runtime] pnpm already prepared at ${toolsDir}`);
    return;
  }
  mkdirSync(toolsDir, { recursive: true });
  const npmArgs = [
    npmCli(nodeRoot),
    "install",
    "--prefix",
    toolsDir,
    `pnpm@${PNPM_VERSION}`,
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
    const error = spawnResult(nodeBinaryPath, args, options);
    if (!error) return;
    lastError = `${registry}: ${error}`;
    console.warn(`[runtime] ${label} install failed via ${registry}: ${error}`);
  }
  fail(`${label} install failed through all registries: ${lastError}`);
}

function pruneNodePty(packageDir) {
  const prebuilds = join(packageDir, "prebuilds");
  if (existsSync(prebuilds)) {
    for (const entry of readdirSync(prebuilds, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      if (entry.name !== PLATFORM_KEY) {
        removePath(join(prebuilds, entry.name));
      }
    }
  }
  for (const name of ["build", "deps", "third_party", "src", "scripts", "typings"]) {
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
  for (const name of ["README.md", "CHANGELOG.md", "install_tools.bat", "corepack", "corepack.cmd", "corepack.ps1"]) {
    removePath(join(target, "node", name));
  }
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
      gzip: true,
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
  const target = options.target ?? defaultTarget();
  if (options.prune) {
    pruneRuntime(target);
    return;
  }
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
  const nodeRoot = await prepareNode(target, cacheDir);
  installDsh(nodeRoot, target, options.refresh);
  installPnpm(nodeRoot, target, options.refresh);
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
