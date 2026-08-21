#!/usr/bin/env node
/**
 * End-to-end smoke test for the bundled dsh runtime.
 *
 * Usage:
 *   node scripts/smoke-runtime.mjs --runtime <dir>
 *   node scripts/smoke-runtime.mjs --archive <runtime.tar.gz>
 *
 * Verifies that a (possibly pruned) runtime can actually serve the official
 * web UI: Node/dsh versions resolve, `dsh web --no-open --port 0` starts, the ready
 * line is emitted, HTTP answers with `__DSH_BOOT__`, and the port is
 * released after the process tree is terminated.
 */

import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import net from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as tar from "tar";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function parseArgs(argv) {
  const options = { runtime: null, archive: null, timeoutMs: 120000, keep: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--runtime") options.runtime = resolve(argv[++index]);
    else if (arg === "--archive") options.archive = resolve(argv[++index]);
    else if (arg === "--timeout-ms") options.timeoutMs = Number(argv[++index]);
    else if (arg === "--keep") options.keep = true;
    else if (arg.startsWith("--runtime=")) options.runtime = resolve(arg.slice("--runtime=".length));
    else if (arg.startsWith("--archive=")) options.archive = resolve(arg.slice("--archive=".length));
    else {
      console.error(`[smoke] unknown argument: ${arg}`);
      process.exit(2);
    }
  }
  if (!options.runtime && !options.archive) {
    console.error("[smoke] pass --runtime <dir> or --archive <file>");
    process.exit(2);
  }
  return options;
}

function nodeBinary(runtimeDir) {
  return process.platform === "win32"
    ? join(runtimeDir, "node", "node.exe")
    : join(runtimeDir, "node", "bin", "node");
}

function resolveDshBin(runtimeDir) {
  const packageDir = join(runtimeDir, "app", "node_modules", "@deepseek-ai", "dsh");
  const manifest = JSON.parse(readFileSync(join(packageDir, "package.json"), "utf8"));
  const bin = manifest.bin;
  const relative =
    typeof bin === "string"
      ? bin
      : bin?.dsh ?? Object.values(bin ?? {})[0];
  if (!relative || typeof relative !== "string") {
    throw new Error("cannot resolve @deepseek-ai/dsh bin field");
  }
  return join(packageDir, relative);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function sleepSync(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function killTree(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  if (process.platform === "win32") {
    const result = spawnSync(
      "taskkill",
      ["/PID", String(child.pid), "/T", "/F"],
      { encoding: "utf8" },
    );
    if (result.status !== 0) {
      // Sandboxes and restricted tokens can deny taskkill; TerminateProcess
      // through the child handle is the reliable fallback.
      child.kill();
    }
    const deadline = Date.now() + 3000;
    while (Date.now() < deadline && child.exitCode === null) {
      sleepSync(100);
    }
    if (child.exitCode === null) child.kill();
  } else {
    child.kill("SIGTERM");
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline) {
      if (child.exitCode !== null || child.signalCode !== null) return;
      sleepSync(100);
    }
    child.kill("SIGKILL");
  }
}

async function portIsFree(port) {
  return new Promise((resolvePromise) => {
    const socket = net.connect({ host: "127.0.0.1", port });
    const done = (free) => {
      socket.destroy();
      resolvePromise(free);
    };
    socket.setTimeout(1000);
    socket.once("connect", () => done(false));
    socket.once("timeout", () => done(true));
    socket.once("error", () => done(true));
  });
}

async function waitForPortRelease(port, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await portIsFree(port)) return true;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 300));
  }
  return false;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const tempRoot = mkdtempSync(join(tmpdir(), "dsh-smoke-"));
  let runtimeDir = options.runtime;
  let cleanupDir = null;

  try {
    if (options.archive) {
      runtimeDir = join(tempRoot, "runtime");
      mkdirSync(runtimeDir, { recursive: true });
      await tar.x({ file: options.archive, cwd: runtimeDir });
      cleanupDir = tempRoot;
    } else if (options.keep) {
      cleanupDir = null;
    } else {
      cleanupDir = tempRoot;
    }

    const node = nodeBinary(runtimeDir);
    const dshBin = resolveDshBin(runtimeDir);
    assert(existsSync(node), `missing bundled node at ${node}`);
    assert(existsSync(dshBin), `missing dsh entry at ${dshBin}`);

    const version = spawnSync(node, ["--version"], { encoding: "utf8" });
    assert(version.status === 0, `node --version failed: ${version.stderr}`);
    const nodeVersion = version.stdout.trim().replace(/^v/, "");
    const [major, minor] = nodeVersion.split(".").map(Number);
    assert(
      (major === 22 && minor >= 19) || major >= 24,
      `bundled node ${nodeVersion} does not satisfy dsh requirement (^22.19.0 || >=24)`,
    );
    console.log(`[smoke] node ${nodeVersion} OK`);

    const dshHome = join(tempRoot, "dsh-home");
    mkdirSync(dshHome, { recursive: true });
    const cwd = join(tempRoot, "workspace");
    mkdirSync(cwd, { recursive: true });

    console.log("[smoke] launching dsh web --no-open --port 0 ...");
    const child = spawn(node, [dshBin, "web", "--no-open", "--port", "0"], {
      cwd,
      env: {
        ...process.env,
        DSH_HOME: dshHome,
        DSH_TELEMETRY_DISABLED: "1",
        NO_COLOR: "1",
      },
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });

    const readyPattern = /dsh web: http:\/\/127\.0\.0\.1:(\d+)/;
    const deadline = Date.now() + options.timeoutMs;
    let readyUrl = null;
    while (Date.now() < deadline) {
      if (child.exitCode !== null || child.signalCode !== null) {
        throw new Error(`dsh exited early (code ${child.exitCode}) before becoming ready.\n${stderr.slice(-4000)}`);
      }
      const match = readyPattern.exec(stdout);
      if (match) {
        readyUrl = match[0];
        break;
      }
      await new Promise((resolvePromise) => setTimeout(resolvePromise, 500));
    }
    assert(readyUrl, `timed out waiting for ready line.\n--- stdout ---\n${stdout.slice(-4000)}\n--- stderr ---\n${stderr.slice(-4000)}`);
    const port = Number(readyPattern.exec(readyUrl)[1]);
    console.log(`[smoke] ready: ${readyUrl}`);

    const response = await fetch(`http://127.0.0.1:${port}`, {
      signal: AbortSignal.timeout(10000),
    });
    assert(response.ok, `HTTP probe returned ${response.status}`);
    const body = await response.text();
    assert(body.includes("__DSH_BOOT__"), "HTTP probe did not find __DSH_BOOT__ marker");
    console.log("[smoke] HTTP probe OK (200 + __DSH_BOOT__)");

    killTree(child);
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 800));
    let released = await waitForPortRelease(port, 20000);
    if (!released && child.exitCode === null) {
      child.kill();
      released = await waitForPortRelease(port, 5000);
      if (released) console.log("[smoke] port released after fallback kill");
    }
    assert(released, `port ${port} was not released after engine termination`);
    console.log(`[smoke] port ${port} released OK`);
    console.log("[smoke] PASS");
  } finally {
    if (cleanupDir && existsSync(cleanupDir) && !options.keep) {
      rmSync(cleanupDir, { recursive: true, force: true });
    }
  }
}

main().catch((error) => {
  console.error(`[smoke] FAIL: ${error.message}`);
  process.exit(1);
});
