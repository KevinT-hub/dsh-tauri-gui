#!/usr/bin/env node
/**
 * Centralized GitHub Release state machine for the release workflow.
 * Replaces ad-hoc `gh release create/edit ... || true` chains so that every
 * real error (permissions, network, invalid args, conflicting state) fails
 * the run instead of being swallowed.
 *
 * Sub-commands:
 *   ensure-draft --tag <t> [--prerelease true|false] [--title <title>]
 *                [--notes <text>] [--notes-file <file>] [--target <sha>]
 *                [--output <file>]
 *       Create a draft release if missing; read it if present. Validates
 *       existing state (never treats unexpected states as "already exists").
 *       Emits `release_id=...` (and `release_exists=true|false`).
 *
 *   upload-assets --tag <t> --release-id <id> <files...>
 *       Upload assets to the given release with --clobber. Fails on error.
 *
 *   publish --tag <t> --release-id <id> [--prerelease true|false]
 *       Publish the draft (draft=false) with an explicit prerelease flag and
 *       make_latest=false. Fails unless the release is currently a draft.
 *
 *   select-latest-stable [--output <file>]
 *       Print the highest stable (non-draft, non-prerelease) vX.Y.Z tag.
 *
 *   set-latest --tag <t>
 *       Make <t> the GitHub Latest (it must be the highest stable release),
 *       demote every other stable release, then assert
 *       `/releases/latest.tag_name == <t>`.
 *
 *   sync-updater --tag <t> --latest-json <file> [--target <sha>]
 *       Create/read the fixed `update` release (prerelease=true,
 *       make_latest=false), upload <file> as latest.json, re-download and
 *       verify content, then assert isPrerelease=true and that `update` is not
 *       the repository's `/releases/latest` release.
 *
 *   assert-state --tag <t> [--draft true|false] [--prerelease true|false]
 *       Assert release properties; exit non-zero on mismatch.
 *
 * Required env: GH_TOKEN, GITHUB_OWNER, GITHUB_REPO (or GITHUB_REPOSITORY).
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { maxVersion, parseVersion } from "./lib/semver.mjs";

const GITHUB_OWNER =
  process.env.GITHUB_OWNER ||
  (process.env.GITHUB_REPOSITORY || "KevinT-hub/dsh-tauri-gui").split("/")[0];
const GITHUB_REPO =
  process.env.GITHUB_REPO ||
  (process.env.GITHUB_REPOSITORY || "KevinT-hub/dsh-tauri-gui").split("/")[1];
const UPDATE_RELEASE = process.env.UPDATE_RELEASE || "update";

function fail(message) {
  throw new Error(message);
}

function gh(argsList, options = {}) {
  try {
    return execFileSync("gh", argsList, {
      encoding: "utf8",
      stdio: options.quiet ? "pipe" : ["ignore", "pipe", "pipe"],
      env: { ...process.env, GH_TOKEN: process.env.GH_TOKEN },
    }).trim();
  } catch (error) {
    const stderr = String(error.stderr || "").trim();
    const status = error.status ?? "?";
    const message = stderr || error.message || "gh failed";
    const wrapped = new Error(`gh ${argsList[0] ?? ""} failed (exit ${status}): ${message}`);
    wrapped.status = status;
    wrapped.stderr = stderr;
    throw wrapped;
  }
}

function isNotFound(error) {
  return /not found|404/i.test(error.stderr || "") || error.status === 404;
}

function api(route, jq) {
  const args = ["api", `repos/${GITHUB_OWNER}/${GITHUB_REPO}/${route}`, "--jq", jq];
  return gh(args);
}

function latestReleaseTag() {
  try {
    return api("releases/latest", ".tag_name");
  } catch (error) {
    if (isNotFound(error)) return null;
    throw error;
  }
}

function patchRelease(releaseId, fields) {
  const args = [
    "api",
    "--method",
    "PATCH",
    `repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/${releaseId}`,
  ];
  for (const [key, value] of Object.entries(fields)) {
    if (typeof value === "boolean") {
      args.push("-F", `${key}=${value}`);
    } else {
      args.push("-f", `${key}=${value}`);
    }
  }
  args.push("--silent");
  gh(args);
}

function releaseView(tag) {
  // Returns the release object, or null when the release does not exist.
  // Any other failure propagates so the caller can distinguish real errors.
  try {
    const raw = JSON.parse(
      gh(["api", `repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/tags/${tag}`]),
    );
    return {
      databaseId: raw.id,
      tagName: raw.tag_name,
      name: raw.name,
      isDraft: raw.draft,
      isPrerelease: raw.prerelease,
      publishedAt: raw.published_at,
      url: raw.html_url,
      assets: raw.assets || [],
    };
  } catch (error) {
    if (isNotFound(error)) return null;
    throw error;
  }
}

function emit(outputFile, obj) {
  const lines = Object.entries(obj)
    .map(([key, value]) => `${key}=${value ?? ""}`)
    .join("\n");
  if (outputFile) {
    writeFileSync(outputFile, `${lines}\n`, { flag: "a" });
  }
  process.stdout.write(`${lines}\n`);
}

function parseBool(value, name) {
  if (value === undefined) return undefined;
  if (value === "true") return true;
  if (value === "false") return false;
  throw new Error(`${name} must be "true" or "false", got "${value}"`);
}

function argValue(args, name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

function requireArg(args, name) {
  const value = argValue(args, name);
  if (!value) throw new Error(`missing required argument ${name}`);
  return value;
}

function positionalArgs(args) {
  return args.filter((arg, index) => {
    if (arg.startsWith("-")) return false;
    const previous = args[index - 1];
    return !(
      previous &&
      ["--tag", "--release-id", "--title", "--notes", "--notes-file", "--target", "--output", "--prerelease", "--draft", "--latest-json"].includes(previous)
    );
  });
}

function cmdEnsureDraft(args) {
  const tag = requireArg(args, "--tag");
  const parsed = parseVersion(tag);
  const prerelease = parseBool(argValue(args, "--prerelease"), "--prerelease") ?? false;
  const title = argValue(args, "--title") || `DeepSeek Harness Tauri Desktop ${parsed.tag}`;
  const notes = argValue(args, "--notes");
  const notesFile = argValue(args, "--notes-file");
  const target = argValue(args, "--target");
  const output = argValue(args, "--output");

  const existing = releaseView(parsed.tag);
  if (existing) {
    // Idempotent rerun: reuse the existing release (draft or published). The
    // Git tag has already been resolved and validated by the workflow. Do not
    // compare targetCommitish here: GitHub stores a branch name such as "main"
    // in that field even when the tag resolves to a commit SHA.
    if (existing.tagName !== parsed.tag) {
      throw new Error(
        `release lookup returned tag ${existing.tagName}, expected ${parsed.tag}`,
      );
    }
    if (existing.isPrerelease !== prerelease) {
      throw new Error(
        `release ${parsed.tag} has prerelease=${existing.isPrerelease}, expected ${prerelease}`,
      );
    }
    emit(output, { release_id: existing.databaseId, release_exists: "true", tag: parsed.tag });
    return;
  }

  const createArgs = [
    "release",
    "create",
    parsed.tag,
    "--draft",
    "--title",
    title,
    "--latest=false",
    `--prerelease=${prerelease}`,
    "--verify-tag",
  ];
  if (notesFile) createArgs.push("--notes-file", notesFile);
  else if (notes) createArgs.push("--notes", notes);
  else createArgs.push("--notes", `Release ${parsed.tag}`);
  if (target) createArgs.push("--target", target);
  gh(createArgs);

  const created = releaseView(parsed.tag);
  if (!created || created.isDraft !== true) {
    throw new Error(`failed to create draft release ${parsed.tag}`);
  }
  emit(output, { release_id: created.databaseId, release_exists: "false", tag: parsed.tag });
}

function cmdUploadAssets(args) {
  const tag = requireArg(args, "--tag");
  const parsed = parseVersion(tag);
  const releaseId = requireArg(args, "--release-id");
  const files = positionalArgs(args);
  if (files.length === 0) {
    throw new Error("upload-assets requires at least one file");
  }
  gh(["release", "upload", parsed.tag, ...files, "--clobber"]);
  const names = JSON.parse(api(`releases/${releaseId}/assets`, "[.[] | .name]"));
  process.stdout.write(
    `[upload] ${files.length} asset(s) uploaded to ${parsed.tag}; ${names.length} asset(s) on release\n`,
  );
}

function cmdPublish(args) {
  const tag = requireArg(args, "--tag");
  const parsed = parseVersion(tag);
  const releaseId = requireArg(args, "--release-id");
  const prerelease = parseBool(argValue(args, "--prerelease"), "--prerelease") ?? parsed.isPrerelease;

  const existing = releaseView(parsed.tag);
  if (!existing) {
    throw new Error(`release ${parsed.tag} does not exist; nothing to publish`);
  }
  if (existing.databaseId !== Number(releaseId)) {
    throw new Error(
      `release id mismatch for ${parsed.tag}: expected ${releaseId}, got ${existing.databaseId}`,
    );
  }
  if (existing.isDraft !== true) {
    // Idempotent rerun of an already published release: only allowed when the
    // prerelease flag matches; nothing to do.
    if (existing.isPrerelease !== prerelease) {
      throw new Error(
        `release ${parsed.tag} is already published with prerelease=${existing.isPrerelease}, expected ${prerelease}`,
      );
    }
    console.warn(`[publish] ${parsed.tag} already published; skipping (idempotent rerun)`);
    return;
  }

  patchRelease(existing.databaseId, {
    draft: false,
    prerelease,
    make_latest: "false",
  });
  const after = releaseView(parsed.tag);
  if (!after || after.isDraft !== false) {
    throw new Error(`publish of ${parsed.tag} did not take effect`);
  }
  process.stdout.write(
    `[publish] ${parsed.tag} published (prerelease=${after.isPrerelease})\n`,
  );
}

function collectStableReleases() {
  const pages = JSON.parse(
    gh([
      "api",
      `repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases`,
      "--paginate",
      "--slurp",
    ]),
  );
  const releases = (Array.isArray(pages) ? pages.flat() : pages).map((release) => ({
    tag_name: release.tag_name,
    draft: release.draft,
    prerelease: release.prerelease,
  }));
  const stable = [];
  for (const release of releases) {
    if (release.draft || release.prerelease) continue;
    try {
      stable.push(parseVersion(release.tag_name).tag);
    } catch {
      // Not a project version (e.g. `update`); not a candidate.
    }
  }
  return stable;
}

function cmdSelectLatestStable(args) {
  const output = argValue(args, "--output");
  const stable = collectStableReleases();
  if (stable.length === 0) {
    throw new Error("no stable vX.Y.Z releases found");
  }
  const selected = maxVersion(stable, { stableOnly: true });
  emit(output, { stable_tag: selected });
}

function cmdSetLatest(args) {
  const tag = requireArg(args, "--tag");
  const parsed = parseVersion(tag);
  if (parsed.isPrerelease) {
    throw new Error(`cannot set Latest to prerelease ${parsed.tag}`);
  }
  const stable = collectStableReleases();
  const selected = maxVersion(stable, { stableOnly: true });
  if (selected !== parsed.tag) {
    // Reruns of older stable releases are allowed; Latest must NOT regress.
    console.warn(
      `[set-latest] ${parsed.tag} is not the highest stable release; keeping Latest at ${selected}`,
    );
  }
  for (const candidate of stable) {
    const release = releaseView(candidate);
    if (!release) throw new Error(`stable release ${candidate} disappeared during Latest reconciliation`);
    patchRelease(release.databaseId, { make_latest: candidate === selected ? "true" : "false" });
  }
  const latestTag = api("releases/latest", ".tag_name");
  if (latestTag !== selected) {
    throw new Error(`Latest assertion failed: /releases/latest is ${latestTag}, expected ${selected}`);
  }
  process.stdout.write(`[set-latest] ${selected} is now GitHub Latest\n`);
}

function cmdSyncUpdater(args) {
  const tag = requireArg(args, "--tag");
  const latestJson = requireArg(args, "--latest-json");
  const target = argValue(args, "--target");
  const parsed = parseVersion(tag);
  if (parsed.isPrerelease) {
    throw new Error(`stable updater channel rejects prerelease ${parsed.tag}`);
  }
  const content = readFileSync(latestJson, "utf8");
  const payload = JSON.parse(content);
  if (payload.version !== parsed.version) {
    throw new Error(`latest.json version ${payload.version} != release version ${parsed.version}`);
  }

  const existing = releaseView(UPDATE_RELEASE);
  if (!existing) {
    const createArgs = [
      "release",
      "create",
      UPDATE_RELEASE,
      "--title",
      "updater",
      "--notes",
      "Release for maintaining latest.json.",
      "--prerelease",
      "--latest=false",
    ];
    if (target) createArgs.push("--target", target);
    gh(createArgs);
  } else {
    patchRelease(existing.databaseId, {
      draft: false,
      prerelease: true,
      make_latest: "false",
    });
  }

  gh(["release", "upload", UPDATE_RELEASE, latestJson, "--clobber"]);

  const after = releaseView(UPDATE_RELEASE);
  if (!after) throw new Error(`update release vanished after sync`);
  const latestTag = latestReleaseTag();
  if (after.isPrerelease !== true || latestTag === UPDATE_RELEASE) {
    throw new Error(
      `update release state wrong: isPrerelease=${after.isPrerelease}, repository latest=${latestTag ?? "<none>"}`,
    );
  }
  const asset = after.assets.find((entry) => entry.name === "latest.json");
  if (!asset) {
    throw new Error("update release has no latest.json asset");
  }

  // Re-download through the GitHub CLI's release endpoint and compare
  // byte-for-byte. `gh release view --json assets` exposes a browser download
  // URL as `url` on current CLI versions, not necessarily the REST asset API
  // URL; passing that URL to `gh api` can return a redirect or binary body
  // instead of the expected `{content: ...}` JSON. `gh release download` is
  // the stable, version-independent way to read the uploaded asset.
  const tempDir = mkdtempSync(join(tmpdir(), "dsh-updater-verify-"));
  const downloaded = join(tempDir, "latest.json");
  try {
    gh([
      "release",
      "download",
      UPDATE_RELEASE,
      "--pattern",
      "latest.json",
      "--output",
      downloaded,
      "--clobber",
    ]);
    const actual = readFileSync(downloaded, "utf8");
    if (actual.trim() !== content.trim()) {
      throw new Error("update/latest.json content differs from the uploaded file");
    }
  } catch (error) {
    throw new Error(
      `failed to download and verify update/latest.json: ${error.message}`,
    );
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
  process.stdout.write(`[sync-updater] ${UPDATE_RELEASE}/latest.json -> ${parsed.tag} OK\n`);
}

function cmdAssertState(args) {
  const tag = requireArg(args, "--tag");
  const draft = parseBool(argValue(args, "--draft"), "--draft");
  const prerelease = parseBool(argValue(args, "--prerelease"), "--prerelease");
  const release = releaseView(tag);
  if (!release) throw new Error(`release ${tag} does not exist`);
  if (draft !== undefined && release.isDraft !== draft) {
    throw new Error(`assert-state: ${tag} isDraft=${release.isDraft}, expected ${draft}`);
  }
  if (prerelease !== undefined && release.isPrerelease !== prerelease) {
    throw new Error(`assert-state: ${tag} isPrerelease=${release.isPrerelease}, expected ${prerelease}`);
  }
  const latestTag = latestReleaseTag();
  process.stdout.write(`[assert-state] ${tag}: draft=${release.isDraft}, prerelease=${release.isPrerelease}, latest=${latestTag === tag}, assets=${release.assets.length}\n`);
}

function main() {
  const argv = process.argv.slice(2);
  if (argv.length === 0) {
    throw new Error("usage: reconcile-release-state <ensure-draft|upload-assets|publish|select-latest-stable|set-latest|sync-updater|assert-state> ...");
  }
  const [command, ...args] = argv;
  switch (command) {
    case "ensure-draft": return cmdEnsureDraft(args);
    case "upload-assets": return cmdUploadAssets(args);
    case "publish": return cmdPublish(args);
    case "select-latest-stable": return cmdSelectLatestStable(args);
    case "set-latest": return cmdSetLatest(args);
    case "sync-updater": return cmdSyncUpdater(args);
    case "assert-state": return cmdAssertState(args);
    default: throw new Error(`unknown sub-command: ${command}`);
  }
}

const isMain =
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href;

if (isMain) {
  try {
    main();
  } catch (error) {
    console.error(`[reconcile-release-state] ${error.message}`);
    process.exit(1);
  }
}
