#!/usr/bin/env node
/**
 * Generates the Tauri updater `latest.json` for the GitHub channel.
 *
 * Usage:
 *   node scripts/update-latest.mjs --from-dir <dir> --version <v> --tag <t> [--notes <notes>] [--channel stable|prerelease] [--base-url <url>] [--output <file>] [--pub-date <iso>]
 *   node scripts/update-latest.mjs --from-release [--tag <t>] [--channel stable|prerelease] [--output <file>]
 *
 * Modes:
 *   --from-dir    Build latest.json from a local artifacts directory (release workflow).
 *   --from-release
 *                 Build latest.json from GitHub release assets. Without --tag it
 *                 selects the highest stable, non-draft, non-prerelease release by
 *                 SemVer precedence (it never trusts `/releases/latest`).
 *
 * Channel semantics:
 *   stable      only `vX.Y.Z` (no prerelease) is accepted; intended for the
 *               stable `update` release channel.
 *   prerelease  any valid project SemVer is accepted; the output must NOT be
 *               uploaded to the stable `update` release.
 *
 * Required env (--from-release mode): GH_TOKEN, GITHUB_OWNER, GITHUB_REPO.
 */

import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { maxVersion, parseVersion } from "./lib/semver.mjs";

const args = process.argv.slice(2);
const argValue = (name) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};

const GITHUB_OWNER =
  process.env.GITHUB_OWNER || (process.env.GITHUB_REPOSITORY || "KevinT-hub/dsh-tauri-gui").split("/")[0];
const GITHUB_REPO =
  process.env.GITHUB_REPO ||
  (process.env.GITHUB_REPOSITORY || "KevinT-hub/dsh-tauri-gui").split("/")[1];
const BASE_URL = argValue("--base-url");
const OUTPUT = argValue("--output");
const PUB_DATE = argValue("--pub-date");
const CHANNEL = (argValue("--channel") || "stable").toLowerCase();

/**
 * Updater platform rules. `required` entries MUST be present in the final
 * latest.json (the Tauri updater needs them); `optional` entries (e.g. the
 * .deb, which is a download artifact but not an updater target) are included
 * when present but their absence is not an error.
 */
export const PLATFORM_RULES = [
  { re: /_Windows_x64-setup\.exe$/, key: "windows-x86_64", required: true },
  { re: /_macOS_x64\.app\.tar\.gz$/, key: "darwin-x86_64", required: true },
  { re: /_macOS_aarch64\.app\.tar\.gz$/, key: "darwin-aarch64", required: true },
  { re: /-Linux-x86_64\.AppImage$/, key: "linux-x86_64", required: true },
  { re: /_Linux_amd64\.deb$/, key: "linux-x86_64-deb", required: false },
];

export const REQUIRED_PLATFORM_KEYS = PLATFORM_RULES.filter(
  (rule) => rule.required,
).map((rule) => rule.key);

export function detectPlatform(fileName) {
  const hits = PLATFORM_RULES.filter((rule) => rule.re.test(fileName));
  if (hits.length > 1) {
    throw new Error(
      `file "${fileName}" matches multiple platform rules: ${hits.map((rule) => rule.key).join(", ")}`,
    );
  }
  return hits[0] ?? null;
}

function releaseDownloadUrl(tag, fileName, baseUrl) {
  const base = (
    baseUrl || `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases/download`
  ).replace(/\/+$/, "");
  return `${base}/${tag}/${encodeURIComponent(fileName)}`;
}

function assertChannel(parsed, channel) {
  if (channel !== "stable" && channel !== "prerelease") {
    throw new Error(`invalid --channel "${channel}" (expected stable|prerelease)`);
  }
  if (channel === "stable" && parsed.isPrerelease) {
    throw new Error(
      `channel "stable" rejects prerelease version ${parsed.tag}; use --channel prerelease`,
    );
  }
}

function walkFiles(dir) {
  const results = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkFiles(fullPath));
    } else if (entry.isFile()) {
      results.push(fullPath);
    }
  }
  return results;
}

/**
 * Atomically write `latest.json`: write to a temp file in the same directory
 * and rename over the target, so a failed run never leaves a half-written file.
 */
export function writeOutputs(payload, outputPath = "latest.json") {
  const resolved = path.resolve(outputPath);
  fs.mkdirSync(path.dirname(resolved), { recursive: true });
  const tmp = `${resolved}.${process.pid}.${Date.now()}.tmp`;
  const content = `${JSON.stringify(payload, null, 2)}\n`;
  fs.writeFileSync(tmp, content);
  try {
    fs.renameSync(tmp, resolved);
  } catch (error) {
    fs.rmSync(tmp, { force: true });
    throw error;
  }
  console.log(`Generated ${resolved}:`);
  console.log(JSON.stringify(payload, null, 2));
}

function finalizePayload(version, notes, platforms, outputPath, pubDate) {
  const requiredMissing = REQUIRED_PLATFORM_KEYS.filter(
    (key) => !platforms[key],
  );
  if (requiredMissing.length > 0) {
    throw new Error(
      `missing required updater platforms: ${requiredMissing.join(", ")}`,
    );
  }
  for (const [key, entry] of Object.entries(platforms)) {
    if (!entry.url || !entry.signature || !entry.sha256) {
      throw new Error(
        `platform ${key} is missing url/signature/sha256: ${JSON.stringify(entry)}`,
      );
    }
  }
  writeOutputs(
    {
      version,
      notes: notes || `Release ${version}`,
      pub_date: pubDate,
      platforms,
    },
    outputPath,
  );
}

export function buildPlatformsFromFiles(files, tag, baseUrl) {
  const platforms = {};
  for (const fullPath of files) {
    const file = path.basename(fullPath);
    if (file.endsWith(".sig") || file === "latest.json") continue;
    const platform = detectPlatform(file);
    if (!platform) continue;
    if (platforms[platform.key]) {
      throw new Error(
        `duplicate asset for platform ${platform.key}: ${file} (already matched by ${platforms[platform.key].file})`,
      );
    }
    const sigPath = `${fullPath}.sig`;
    if (!fs.existsSync(sigPath)) {
      throw new Error(`Missing signature file for ${file}: ${sigPath}`);
    }
    const signature = fs.readFileSync(sigPath, "utf8").trim();
    if (!signature) {
      throw new Error(`Signature file is empty for ${file}: ${sigPath}`);
    }
    const sha256 = createHash("sha256")
      .update(fs.readFileSync(fullPath))
      .digest("hex");
    const url = releaseDownloadUrl(tag, file, baseUrl);
    assertTagInUrl(url, tag);
    platforms[platform.key] = {
      signature,
      sha256,
      url,
      file,
    };
    console.log(`[ok] ${platform.key} <- ${file}`);
  }
  return platforms;
}

export function buildFromDir(dir, version, tag, notes, baseUrl, outputPath, pubDate, channel = CHANNEL) {
  if (!fs.existsSync(dir)) {
    throw new Error(`Artifacts directory not found: ${dir}`);
  }
  const parsed = parseVersion(tag);
  assertChannel(parsed, channel);
  if (parsed.version !== parseVersion(version).version) {
    throw new Error(`--tag ${tag} and --version ${version} disagree`);
  }
  const platforms = buildPlatformsFromFiles(
    walkFiles(dir).sort(),
    parsed.tag,
    baseUrl,
  );
  if (Object.keys(platforms).length === 0) {
    throw new Error("No updater artifacts found in directory");
  }
  finalizePayload(parsed.version, notes, platforms, outputPath, pubDate);
}

function gh(argsList) {
  return execFileSync("gh", argsList, {
    encoding: "utf8",
    env: { ...process.env, GH_TOKEN: process.env.GH_TOKEN },
  }).trim();
}

/**
 * Fetch a signature asset with a timeout and content sanity checks. HTTP
 * errors are surfaced with the full status/URL so failures are diagnosable.
 */
export async function fetchSignature(url) {
  const response = await fetch(url, {
    headers: {
      Authorization: `Bearer ${process.env.GH_TOKEN}`,
      Accept: "application/octet-stream",
    },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) {
    throw new Error(
      `failed to download signature ${url}: HTTP ${response.status} ${response.statusText}`,
    );
  }
  const contentType = response.headers.get("content-type") || "";
  if (contentType.includes("text/html")) {
    throw new Error(
      `signature download returned an HTML error page (content-type ${contentType}): ${url}`,
    );
  }
  const text = (await response.text()).trim();
  if (!text) {
    throw new Error(`downloaded empty signature from ${url}`);
  }
  return text;
}

export function assertTagInUrl(url, tag) {
  if (!url.includes(`/${tag}/`)) {
    throw new Error(
      `generated URL does not reference release tag ${tag}: ${url}`,
    );
  }
}

function releaseAssetDigest(asset) {
  const digest = asset.digest;
  if (!digest) {
    throw new Error(`missing sha256 digest for asset ${asset.name}`);
  }
  const match = /^(sha256:)?([0-9a-fA-F]{64})$/.exec(String(digest).trim());
  if (!match) {
    throw new Error(
      `asset ${asset.name} digest "${digest}" is not a sha256 digest; refusing to fall back to an unverified value`,
    );
  }
  return match[2].toLowerCase();
}

/**
 * Map release assets to updater platforms. `signatures` maps asset name ->
 * signature text (already downloaded). Pure function, unit-testable.
 */
export function buildPlatformsFromAssets(assets, { tag, baseUrl, signatures = {} }) {
  const platforms = {};
  const matched = [];
  for (const asset of assets) {
    if (asset.name.endsWith(".sig")) continue;
    const platform = detectPlatform(asset.name);
    if (!platform) continue;
    matched.push({ asset, platform });
  }
  for (const { asset, platform } of matched) {
    if (platforms[platform.key]) {
      throw new Error(
        `duplicate asset for platform ${platform.key}: ${asset.name} (already matched by ${platforms[platform.key].file})`,
      );
    }
    const signature = signatures[asset.name];
    if (!signature) {
      throw new Error(`missing signature asset for ${asset.name}`);
    }
    const sha256 = releaseAssetDigest(asset);
    const url = releaseDownloadUrl(tag, asset.name, baseUrl);
    assertTagInUrl(url, tag);
    platforms[platform.key] = { signature, sha256, url, file: asset.name };
    console.log(`[ok] ${platform.key} <- ${asset.name}`);
  }
  return platforms;
}

async function fetchRelease(tag) {
  const json = gh([
    "api",
    `repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/tags/${tag}`,
    "--jq",
    "{tag_name, draft, prerelease, published_at, body, assets: [.assets[] | {name, browser_download_url, digest}]}",
  ]);
  return JSON.parse(json);
}

/**
 * Select the highest stable (non-draft, non-prerelease) release by SemVer.
 * Never trusts `/releases/latest`.
 */
export function selectStableTagFromReleases(releases) {
  const candidates = [];
  for (const release of releases) {
    if (release.draft || release.prerelease) continue;
    try {
      candidates.push(parseVersion(release.tag_name).tag);
    } catch {
      // Non-project tags (e.g. `update`, legacy names) are not candidates.
    }
  }
  if (candidates.length === 0) {
    throw new Error("no stable vX.Y.Z releases found");
  }
  return maxVersion(candidates, { stableOnly: true });
}

function listReleases() {
  const json = gh([
    "api",
    `repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases`,
    "--paginate",
    "--jq",
    ".[] | {tag_name, draft, prerelease}",
  ]);
  const lines = json.trim().split("\n").filter(Boolean);
  return lines.map((line) => JSON.parse(line));
}

async function buildFromRelease(tag, channel = CHANNEL) {
  if (!process.env.GH_TOKEN) {
    throw new Error("GH_TOKEN is required in --from-release mode");
  }

  let resolvedTag = tag;
  if (!resolvedTag) {
    resolvedTag = selectStableTagFromReleases(listReleases());
    console.log(`[resolve] selected highest stable release: ${resolvedTag}`);
  }

  const parsed = parseVersion(resolvedTag);
  assertChannel(parsed, channel);

  const release = await fetchRelease(parsed.tag);
  if (release.draft) {
    throw new Error(`release ${parsed.tag} is a draft; refusing to generate latest.json`);
  }
  if (channel === "stable" && release.prerelease) {
    throw new Error(`release ${parsed.tag} is marked prerelease; stable channel rejects it`);
  }
  if (!release.published_at) {
    throw new Error(`release ${parsed.tag} has no published_at; refusing to generate latest.json`);
  }
  if (release.assets.length === 0) {
    throw new Error(`release ${parsed.tag} has no assets`);
  }

  const signatures = {};
  for (const asset of release.assets) {
    if (asset.name.endsWith(".sig")) {
      signatures[asset.name.slice(0, -4)] = await fetchSignature(asset.browser_download_url);
    }
  }

  const platforms = buildPlatformsFromAssets(release.assets, {
    tag: parsed.tag,
    baseUrl: BASE_URL,
    signatures,
  });
  if (Object.keys(platforms).length === 0) {
    throw new Error("No updater assets found on the release");
  }

  finalizePayload(
    parsed.version,
    release.body || undefined,
    platforms,
    OUTPUT,
    PUB_DATE || release.published_at || new Date().toISOString(),
  );
}

async function main() {
  if (CHANNEL !== "stable" && CHANNEL !== "prerelease") {
    throw new Error(`invalid --channel "${CHANNEL}" (expected stable|prerelease)`);
  }
  if (args.includes("--from-dir")) {
    const dir = argValue("--from-dir");
    const version = argValue("--version");
    const tag = argValue("--tag");
    const notes = argValue("--notes");
    if (!dir || !version || !tag) {
      throw new Error("--from-dir requires --from-dir/--version/--tag");
    }
    buildFromDir(dir, version, tag, notes, BASE_URL, OUTPUT, PUB_DATE, CHANNEL);
  } else if (args.includes("--from-release")) {
    await buildFromRelease(argValue("--tag"), CHANNEL);
  } else {
    throw new Error(
      "Usage: --from-dir <dir> --version <v> --tag <t> [--channel stable|prerelease] [--base-url <url>] [--output <file>] | --from-release [--tag <t>] [--channel stable|prerelease] [--output <file>]",
    );
  }
}

const isMain =
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href;

if (isMain) {
  main().catch((error) => {
    console.error(error.stack || error.message || error);
    process.exit(1);
  });
}
