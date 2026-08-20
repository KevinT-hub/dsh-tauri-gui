/**
 * Unit tests for scripts/update-latest.mjs platform mapping, stable selection
 * and latest.json generation. Run: node --test tests/update-latest.test.mjs
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildPlatformsFromAssets,
  buildPlatformsFromFiles,
  detectPlatform,
  PLATFORM_RULES,
  selectStableTagFromReleases,
  writeOutputs,
} from "../scripts/update-latest.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURES = join(ROOT, "tests", "fixtures", "releases");

function fixture(name) {
  return JSON.parse(readFileSync(join(FIXTURES, name), "utf8"));
}

test("platform detection maps release naming to updater keys", () => {
  assert.equal(detectPlatform("dsh-tauri-gui_1.2.3_Windows_x64-setup.exe").key, "windows-x86_64");
  assert.equal(detectPlatform("dsh-tauri-gui_1.2.3_macOS_x64.app.tar.gz").key, "darwin-x86_64");
  assert.equal(detectPlatform("dsh-tauri-gui_1.2.3_macOS_aarch64.app.tar.gz").key, "darwin-aarch64");
  assert.equal(detectPlatform("dsh-tauri-gui-1.2.3-Linux-x86_64.AppImage").key, "linux-x86_64");
  assert.equal(detectPlatform("dsh-tauri-gui_1.2.3_Linux_amd64.deb").key, "linux-x86_64-deb");
  assert.equal(detectPlatform("README.md"), null);
  assert.equal(detectPlatform("dsh-tauri-gui_1.2.3_Linux_amd64.rpm"), null); // not an updater artifact
});

function signaturesFromAssets(assets) {
  // Mirrors buildFromRelease: the signature map is derived from .sig assets.
  const signatures = {};
  for (const asset of assets) {
    if (asset.name.endsWith(".sig")) {
      signatures[asset.name.slice(0, -4)] = `sig:${asset.name}`;
    }
  }
  return signatures;
}

test("stable release maps all required platforms + optional deb", () => {
  const release = fixture("release-stable.json");
  const platforms = buildPlatformsFromAssets(release.assets, {
    tag: "v1.2.3",
    signatures: signaturesFromAssets(release.assets),
  });
  const required = PLATFORM_RULES.filter((rule) => rule.required).map((rule) => rule.key);
  for (const key of required) {
    assert.ok(platforms[key], `missing required platform ${key}`);
  }
  assert.ok(platforms["linux-x86_64-deb"], "optional deb should be included when present");
  for (const entry of Object.values(platforms)) {
    assert.ok(entry.url.includes("/v1.2.3/"), `url must reference v1.2.3: ${entry.url}`);
    assert.ok(entry.signature, "signature must be non-empty");
    assert.match(entry.sha256, /^[0-9a-f]{64}$/);
  }
});

test("stable selector ignores beta/rc/update/draft and picks highest stable", () => {
  const releases = fixture("releases-list.json");
  assert.equal(selectStableTagFromReleases(releases), "v1.10.0");
});

test("stable selector rejects when only prereleases exist", () => {
  assert.throws(
    () => selectStableTagFromReleases([{ tag_name: "v1.0.0-rc.1", draft: false, prerelease: true }]),
    /no stable vX\.Y\.Z releases/,
  );
});

test("missing signature asset fails generation", () => {
  const release = fixture("release-stable.json");
  const assets = release.assets.filter((asset) => asset.name !== "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe.sig");
  assert.throws(
    () => buildPlatformsFromAssets(assets, { tag: "v1.2.3", signatures: signaturesFromAssets(assets) }),
    /missing signature/,
  );
});

test("missing sha256 digest fails instead of falling back", () => {
  const release = fixture("release-stable.json");
  const assets = release.assets.map((asset) =>
    asset.name === "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe" ? { ...asset, digest: null } : asset,
  );
  assert.throws(
    () => buildPlatformsFromAssets(assets, { tag: "v1.2.3", signatures: signaturesFromAssets(assets) }),
    /missing sha256 digest/,
  );
});

test("non-sha256 digest fails", () => {
  const release = fixture("release-stable.json");
  const assets = release.assets.map((asset) =>
    asset.name === "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe"
      ? { ...asset, digest: "md5:deadbeef" }
      : asset,
  );
  assert.throws(
    () => buildPlatformsFromAssets(assets, { tag: "v1.2.3", signatures: signaturesFromAssets(assets) }),
    /not a sha256 digest/,
  );
});

test("duplicate platform assets fail", () => {
  const release = fixture("release-stable.json");
  const assets = [
    ...release.assets,
    {
      name: "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe",
      browser_download_url: "https://example.com/dup",
      digest: "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    },
  ];
  assert.throws(
    () => buildPlatformsFromAssets(assets, { tag: "v1.2.3", signatures: signaturesFromAssets(assets) }),
    /duplicate asset for platform windows-x86_64/,
  );
});

test("from-dir generation requires .sig companions and writes atomically", () => {
  const dir = mkdtempSync(join(tmpdir(), "dsh-update-latest-"));
  try {
    const files = {
      "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe": "exe-content",
      "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe.sig": "sig",
      "dsh-tauri-gui-1.2.3-Linux-x86_64.AppImage": "appimage",
      "dsh-tauri-gui-1.2.3-Linux-x86_64.AppImage.sig": "sig2",
      "dsh-tauri-gui_1.2.3_macOS_x64.app.tar.gz": "tar",
      "dsh-tauri-gui_1.2.3_macOS_x64.app.tar.gz.sig": "sig3",
      "dsh-tauri-gui_1.2.3_macOS_aarch64.app.tar.gz": "tar2",
      "dsh-tauri-gui_1.2.3_macOS_aarch64.app.tar.gz.sig": "sig4",
    };
    for (const [name, content] of Object.entries(files)) {
      writeFileSync(join(dir, name), content);
    }
    const platforms = buildPlatformsFromFiles(
      Object.keys(files).map((name) => join(dir, name)),
      "v1.2.3",
    );
    assert.equal(Object.keys(platforms).length, 4);
    assert.ok(platforms["windows-x86_64"]);

    const output = join(dir, "latest.json");
    writeOutputs({ version: "1.2.3", notes: "n", pub_date: "d", platforms }, output);
    const written = JSON.parse(readFileSync(output, "utf8"));
    assert.equal(written.version, "1.2.3");
    assert.ok(!readFileSync(output, "utf8").includes(".tmp"), "no temp file residue");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("from-dir rejects missing signature", () => {
  const dir = mkdtempSync(join(tmpdir(), "dsh-update-latest-"));
  try {
    writeFileSync(join(dir, "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe"), "x");
    assert.throws(
      () => buildPlatformsFromFiles([join(dir, "dsh-tauri-gui_1.2.3_Windows_x64-setup.exe")], "v1.2.3"),
      /Missing signature file/,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
