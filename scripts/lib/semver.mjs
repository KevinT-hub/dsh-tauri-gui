/**
 * Strict SemVer parsing and comparison for project release versions.
 *
 * Version policy (see docs/GITHUB_ACTIONS_HARDENING_PLAN.md §1):
 *   stable       vX.Y.Z
 *   prerelease   vX.Y.Z-alpha.N / vX.Y.Z-beta.N / vX.Y.Z-rc.N
 *   other        vX.Y.Z-preview.N / vX.Y.Z-dev.N
 *   build meta   vX.Y.Z+build.N (allowed, does not affect precedence)
 *
 * Only `vX.Y.Z` without a prerelease is considered stable. A prerelease's
 * first dot-separated identifier MUST be one of the known channels below;
 * anything else (e.g. `v1.2.3-rcandidate`) is rejected at parse time so it
 * can never be misclassified as rc.
 */

export const SEMVER_RE =
  /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

export const KNOWN_CHANNELS = ["alpha", "beta", "rc", "dev", "preview"];

/**
 * Parse a version string (with or without the `v` prefix) into a structured
 * object. Throws on any input that is not a valid project SemVer.
 *
 * @param {string} input
 * @returns {{raw: string, tag: string, version: string, major: number,
 *   minor: number, patch: number, prerelease: string, build: string,
 *   channel: string, isPrerelease: boolean, baseVersion: string}}
 */
export function parseVersion(input) {
  if (typeof input !== "string" || input.trim().length === 0) {
    throw new Error(`invalid version: ${JSON.stringify(input)}`);
  }
  const raw = input.trim();
  const match = SEMVER_RE.exec(raw);
  if (!match) {
    throw new Error(
      `invalid semver "${raw}": expected vX.Y.Z[-prerelease][+build]`,
    );
  }
  const [, majorStr, minorStr, patchStr, prerelease, build] = match;
  const major = Number(majorStr);
  const minor = Number(minorStr);
  const patch = Number(patchStr);

  let channel = "stable";
  if (prerelease) {
    const head = prerelease.split(".")[0].toLowerCase();
    if (!KNOWN_CHANNELS.includes(head)) {
      throw new Error(
        `invalid prerelease "${raw}": first identifier "${head}" is not a ` +
          `supported channel (${KNOWN_CHANNELS.join("/")})`,
      );
    }
    channel = head;
  }

  const baseVersion = `${major}.${minor}.${patch}`;
  const version = prerelease ? `${baseVersion}-${prerelease}` : baseVersion;
  return {
    raw,
    tag: raw.startsWith("v") ? raw : `v${raw}`,
    version,
    major,
    minor,
    patch,
    prerelease: prerelease || "",
    build: build || "",
    channel,
    isPrerelease: Boolean(prerelease),
    baseVersion,
  };
}

/**
 * SemVer precedence comparison. Build metadata is ignored per the spec.
 * Returns -1, 0 or 1. Throws if either argument fails validation.
 *
 * @param {string} a
 * @param {string} b
 * @returns {-1|0|1}
 */
export function compareVersions(a, b) {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  for (const key of ["major", "minor", "patch"]) {
    if (pa[key] !== pb[key]) return pa[key] < pb[key] ? -1 : 1;
  }
  // No prerelease sorts higher than one with a prerelease.
  if (pa.isPrerelease !== pb.isPrerelease) return pa.isPrerelease ? -1 : 1;
  if (pa.isPrerelease) {
    const ra = pa.prerelease.split(".");
    const rb = pb.prerelease.split(".");
    const length = Math.max(ra.length, rb.length);
    for (let index = 0; index < length; index += 1) {
      if (index >= ra.length) return -1;
      if (index >= rb.length) return 1;
      const result = comparePrereleaseIdentifier(ra[index], rb[index]);
      if (result !== 0) return result;
    }
  }
  return 0;
}

function comparePrereleaseIdentifier(a, b) {
  const numericA = /^\d+$/.test(a);
  const numericB = /^\d+$/.test(b);
  if (numericA && numericB) {
    const valueA = Number(a);
    const valueB = Number(b);
    if (valueA !== valueB) return valueA < valueB ? -1 : 1;
    return 0;
  }
  if (numericA !== numericB) return numericA ? -1 : 1;
  if (a === b) return 0;
  return a < b ? -1 : 1;
}

/**
 * Sort an array of version strings by SemVer precedence (ascending).
 * Throws if any entry fails validation.
 *
 * @param {string[]} versions
 * @returns {string[]}
 */
export function sortVersions(versions) {
  return [...versions].sort((a, b) => compareVersions(a, b));
}

/**
 * Select the maximum version by SemVer precedence (stable-first aware).
 * Prefers the highest stable version when `stableOnly` is set; otherwise the
 * absolute maximum.
 *
 * @param {string[]} versions
 * @param {{stableOnly?: boolean}} [options]
 * @returns {string}
 */
export function maxVersion(versions, options = {}) {
  const { stableOnly = false } = options;
  const candidates = stableOnly
    ? versions.filter((value) => !parseVersion(value).isPrerelease)
    : versions;
  if (candidates.length === 0) {
    throw new Error(`no ${stableOnly ? "stable " : ""}versions to select from`);
  }
  return candidates.reduce((best, current) =>
    compareVersions(current, best) > 0 ? current : best,
  );
}
