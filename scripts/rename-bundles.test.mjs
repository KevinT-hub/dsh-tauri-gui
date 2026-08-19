/**
 * Tests for scripts/rename-bundles.mjs run as a subprocess against a
 * temporary bundle directory. Run: node --test scripts/rename-bundles.test.mjs
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = join(ROOT, "scripts", "rename-bundles.mjs");

function nodeBin() {
  return process.execPath;
}

function runRename(args, cwd) {
  return execFileSync(nodeBin(), [SCRIPT, ...args], {
    encoding: "utf8",
    cwd,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function makeBundleDir(platform, version, withSig = true) {
  const dir = mkdtempSync(join(tmpdir(), "dsh-rename-"));
  if (platform === "windows") {
    const nsis = join(dir, "nsis");
    const msi = join(dir, "msi");
    mkdirSync(nsis, { recursive: true });
    mkdirSync(msi, { recursive: true });
    writeFileSync(join(nsis, `dsh-tauri-gui_${version}_x64-setup.exe`), "exe");
    writeFileSync(join(msi, `dsh-tauri-gui_${version}_x64.msi`), "msi");
    if (withSig) {
      writeFileSync(join(nsis, `dsh-tauri-gui_${version}_x64-setup.exe.sig`), "sig");
      writeFileSync(join(msi, `dsh-tauri-gui_${version}_x64.msi.sig`), "sig");
    }
  } else if (platform === "linux") {
    const appimage = join(dir, "appimage");
    const deb = join(dir, "deb");
    const rpm = join(dir, "rpm");
    mkdirSync(appimage, { recursive: true });
    mkdirSync(deb, { recursive: true });
    mkdirSync(rpm, { recursive: true });
    writeFileSync(join(appimage, `dsh-tauri-gui_${version}_amd64.AppImage`), "appimage");
    writeFileSync(join(deb, `dsh-tauri-gui_${version}_amd64.deb`), "deb");
    writeFileSync(join(rpm, `dsh-tauri-gui_${version}_amd64.rpm`), "rpm");
    if (withSig) {
      writeFileSync(join(appimage, `dsh-tauri-gui_${version}_amd64.AppImage.sig`), "sig");
      writeFileSync(join(deb, `dsh-tauri-gui_${version}_amd64.deb.sig`), "sig");
      writeFileSync(join(rpm, `dsh-tauri-gui_${version}_amd64.rpm.sig`), "sig");
    }
  } else {
    const dmg = join(dir, "dmg");
    const macos = join(dir, "macos");
    mkdirSync(dmg, { recursive: true });
    mkdirSync(macos, { recursive: true });
    writeFileSync(join(dmg, `dsh-tauri-gui_${version}_x64.dmg`), "dmg");
    writeFileSync(join(macos, `dsh-tauri-gui_${version}_x64.app.tar.gz`), "tar");
    if (withSig) {
      writeFileSync(join(dmg, `dsh-tauri-gui_${version}_x64.dmg.sig`), "sig");
      writeFileSync(join(macos, `dsh-tauri-gui_${version}_x64.app.tar.gz.sig`), "sig");
    }
  }
  return dir;
}

test("renames windows bundles and emits manifest", () => {
  const dir = makeBundleDir("windows", "1.2.3");
  try {
    const stdout = runRename(["--dir", dir, "--version", "1.2.3", "--platform", "windows"], ROOT);
    assert.ok(stdout.includes("Windows_x64-setup.exe"));
    const manifestPath = join(dir, "bundle-manifest.json");
    assert.ok(existsSync(manifestPath));
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    assert.equal(manifest.version, "1.2.3");
    assert.equal(manifest.files.length, 2);
    for (const entry of manifest.files) {
      assert.ok(
        /(^|\/)dsh-tauri-gui_1\.2\.3_Windows_x64(?:-setup\.exe|\.msi)$/.test(entry.artifact),
        `unexpected artifact ${entry.artifact}`,
      );
      assert.match(entry.sha256, /^[0-9a-f]{64}$/);
      assert.ok(existsSync(join(dir, entry.signature)), `missing ${entry.signature}`);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("missing .sig companion fails the run", () => {
  const dir = makeBundleDir("linux", "1.2.3", false);
  try {
    assert.throws(
      () => runRename(["--dir", dir, "--version", "1.2.3", "--platform", "linux"], ROOT),
      /missing \.sig companion/,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("duplicate artifact for one rule fails the run", () => {
  const dir = makeBundleDir("linux", "1.2.3");
  try {
    // Add a second .AppImage so the rule matches twice.
    writeFileSync(join(dir, "appimage", "dsh-tauri-gui_1.2.3_extra.AppImage"), "x");
    writeFileSync(join(dir, "appimage", "dsh-tauri-gui_1.2.3_extra.AppImage.sig"), "sig");
    assert.throws(
      () => runRename(["--dir", dir, "--version", "1.2.3", "--platform", "linux"], ROOT),
      /multiple artifacts matched/,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("invalid version is rejected before any rename", () => {
  const dir = makeBundleDir("windows", "1.2.3");
  try {
    assert.throws(
      () => runRename(["--dir", dir, "--version", "1.2.3-rcandidate", "--platform", "windows"], ROOT),
      /not a supported channel/,
    );
    assert.throws(
      () => runRename(["--dir", dir, "--version", "", "--platform", "windows"], ROOT),
      /--dir, --version and --platform are required/,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("macos with arch renames app.tar.gz and dmg", () => {
  const dir = makeBundleDir("macos", "1.2.3");
  try {
    runRename(["--dir", dir, "--version", "1.2.3", "--platform", "macos", "--arch", "aarch64"], ROOT);
    const manifest = JSON.parse(readFileSync(join(dir, "bundle-manifest.json"), "utf8"));
    const names = manifest.files.map((entry) => entry.artifact);
    assert.ok(names.some((name) => name.includes("macOS_aarch64.dmg")));
    assert.ok(names.some((name) => name.includes("macOS_aarch64.app.tar.gz")));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("dry run does not rename or write manifest", () => {
  const dir = makeBundleDir("windows", "1.2.3");
  try {
    runRename(["--dir", dir, "--version", "1.2.3", "--platform", "windows", "--dry-run"], ROOT);
    assert.ok(existsSync(join(dir, "nsis", "dsh-tauri-gui_1.2.3_x64-setup.exe")), "source untouched");
    assert.ok(!existsSync(join(dir, "bundle-manifest.json")), "no manifest on dry run");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
