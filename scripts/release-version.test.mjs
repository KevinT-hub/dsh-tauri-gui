/**
 * Unit tests for scripts/lib/semver.mjs and scripts/release-version.mjs.
 * Run: node --test scripts/release-version.test.mjs
 */

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  compareVersions,
  maxVersion,
  parseVersion,
  sortVersions,
} from "./lib/semver.mjs";
import { readProjectVersions } from "./release-version.mjs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

test("v1.2.3 is stable and eligible to be Latest", () => {
  const parsed = parseVersion("v1.2.3");
  assert.equal(parsed.isPrerelease, false);
  assert.equal(parsed.channel, "stable");
  assert.equal(parsed.version, "1.2.3");
  assert.equal(parsed.tag, "v1.2.3");
  assert.equal(parsed.baseVersion, "1.2.3");
});

test("beta/rc prereleases cannot be stable or Latest", () => {
  for (const tag of ["v1.2.3-beta.1", "v1.2.3-rc.1", "v1.2.3-alpha.1", "v1.2.3-preview.1", "v1.2.3-dev.1"]) {
    const parsed = parseVersion(tag);
    assert.equal(parsed.isPrerelease, true, tag);
    assert.equal(parsed.channel, tag.split("-")[1].split(".")[0], tag);
  }
});

test("v1.2.3-rcandidate is rejected, never misclassified as rc", () => {
  assert.throws(() => parseVersion("v1.2.3-rcandidate"), /not a supported channel/);
});

test("bare channel identifiers are accepted (rc, beta, alpha as full identifier)", () => {
  // `v1.2.3-rc` has a single identifier "rc" which is a known channel.
  const parsed = parseVersion("v1.2.3-rc");
  assert.equal(parsed.channel, "rc");
  assert.equal(parsed.isPrerelease, true);
});

test("v1.10.0 sorts higher than v1.9.9 (no string comparison)", () => {
  assert.equal(compareVersions("v1.10.0", "v1.9.9"), 1);
  assert.equal(compareVersions("v1.9.9", "v1.10.0"), -1);
  assert.equal(maxVersion(["v1.9.9", "v1.10.0", "v1.2.3"]), "v1.10.0");
  assert.deepEqual(sortVersions(["v1.9.9", "v1.10.0", "v1.2.3"]), ["v1.2.3", "v1.9.9", "v1.10.0"]);
});

test("build metadata does not affect precedence; v1.2.3+build.1 is stable", () => {
  assert.equal(compareVersions("v1.2.3", "v1.2.3+build.1"), 0);
  const parsed = parseVersion("v1.2.3+build.1");
  assert.equal(parsed.isPrerelease, false);
  assert.equal(parsed.channel, "stable");
  assert.equal(parsed.build, "build.1");
});

test("prerelease ordering follows SemVer", () => {
  const ordered = ["v1.0.0-alpha", "v1.0.0-alpha.1", "v1.0.0-alpha.beta", "v1.0.0-beta", "v1.0.0-beta.2", "v1.0.0-beta.11", "v1.0.0-rc.1", "v1.0.0"];
  for (let i = 0; i < ordered.length - 1; i += 1) {
    assert.equal(compareVersions(ordered[i], ordered[i + 1]), -1, `${ordered[i]} < ${ordered[i + 1]}`);
  }
});

test("rerun of an older stable release never wins over a newer one", () => {
  assert.equal(maxVersion(["v1.3.0", "v1.2.3"], { stableOnly: true }), "v1.3.0");
  assert.equal(maxVersion(["v1.2.3", "v1.3.0", "v1.2.3-rc.2"], { stableOnly: true }), "v1.3.0");
});

test("project version files agree with each other", () => {
  const versions = readProjectVersions(ROOT);
  const unique = new Set(Object.values(versions));
  assert.equal(unique.size, 1, JSON.stringify(versions));
  assert.match(versions.package, /^\d+\.\d+\.\d+/);
});

test("maxVersion rejects empty / all-prerelease inputs when stableOnly", () => {
  assert.throws(() => maxVersion([], { stableOnly: true }), /no stable versions/);
  assert.throws(() => maxVersion(["v1.0.0-rc.1"], { stableOnly: true }), /no stable versions/);
});
