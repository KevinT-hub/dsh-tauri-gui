#!/usr/bin/env node
/**
 * Single source of truth for project release version handling.
 *
 * Commands:
 *   parse <version-or-tag>            parse and validate a version
 *       [--output <file>]             also write key=value lines (GITHUB_OUTPUT format)
 *       [--json]                      print the structured result as JSON instead
 *   compare <a> <b>                   print -1 | 0 | 1 (SemVer precedence)
 *   check-files                       validate package.json / Cargo.toml /
 *                                     tauri.conf.json / Cargo.lock agree
 *       [--version <v>]               additionally require the files to equal v
 *       [--json]                      print JSON instead of key=value lines
 *
 * Examples:
 *   node scripts/release-version.mjs parse v1.2.3-rc.1 --output "$GITHUB_OUTPUT"
 *   node scripts/release-version.mjs parse v1.2.3 --json
 *   node scripts/release-version.mjs compare v1.10.0 v1.9.9
 *   node scripts/release-version.mjs check-files --version 1.2.3
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  compareVersions,
  KNOWN_CHANNELS,
  parseVersion,
} from "./lib/semver.mjs";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function parseArgs(argv) {
  const options = { output: null, json: false, version: null };
  const positional = [];
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--output") options.output = argv[++index];
    else if (arg === "--json") options.json = true;
    else if (arg === "--version") options.version = argv[++index];
    else if (arg.startsWith("--output=")) options.output = arg.slice("--output=".length);
    else if (arg.startsWith("--version=")) options.version = arg.slice("--version=".length);
    else if (arg.startsWith("-")) throw new Error(`unknown argument: ${arg}`);
    else positional.push(arg);
  }
  return { options, positional };
}

function printKeyValue(output, obj) {
  const lines = Object.entries(obj)
    .map(([key, value]) => `${key}=${value ?? ""}`)
    .join("\n");
  if (output) {
    writeFileSync(output, `${lines}\n`, { flag: "a" });
  }
  return lines;
}

function cmdParse(args, options) {
  if (args.length !== 1) {
    throw new Error("usage: release-version parse <version-or-tag> [--output <file>] [--json]");
  }
  const parsed = parseVersion(args[0]);
  const output = {
    tag: parsed.tag,
    version: parsed.version,
    channel: parsed.channel,
    is_prerelease: parsed.isPrerelease ? "true" : "false",
    base_version: parsed.baseVersion,
    prerelease: parsed.prerelease,
    build: parsed.build,
  };
  if (options.json) {
    process.stdout.write(`${JSON.stringify(parsed, null, 2)}\n`);
    return;
  }
  const lines = printKeyValue(options.output, output);
  process.stdout.write(`${lines}\n`);
}

function cmdCompare(args) {
  if (args.length !== 2) {
    throw new Error("usage: release-version compare <a> <b>");
  }
  process.stdout.write(`${compareVersions(args[0], args[1])}\n`);
}

function readCargoTomlVersion(file) {
  const text = readFileSync(file, "utf8");
  const lines = text.split(/\r?\n/);
  let inPackage = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      inPackage = trimmed === "[package]";
      continue;
    }
    if (inPackage) {
      const match = /^version\s*=\s*"([^"]+)"/.exec(trimmed);
      if (match) return match[1];
    }
  }
  throw new Error(`cannot find [package] version in ${file}`);
}

function readCargoLockVersion(file) {
  const text = readFileSync(file, "utf8");
  const match = new RegExp(
    String.raw`\[\[package\]\]\r?\nname = "dsh-tauri-gui"\r?\nversion = "([^"]+)"`,
  ).exec(text);
  if (!match) {
    throw new Error(`cannot find version for package "dsh-tauri-gui" in ${file}`);
  }
  return match[1];
}

export function readProjectVersions(root = ROOT) {
  const pkg = JSON.parse(
    readFileSync(join(root, "package.json"), "utf8"),
  );
  const tauri = JSON.parse(
    readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"),
  );
  return {
    package: pkg.version,
    cargo: readCargoTomlVersion(join(root, "src-tauri", "Cargo.toml")),
    tauri: tauri.version,
    lock: readCargoLockVersion(join(root, "src-tauri", "Cargo.lock")),
  };
}

function cmdCheckFiles(args, options, root = ROOT) {
  const versions = readProjectVersions(root);
  const expected = options.version ? parseVersion(options.version).version : null;

  const mismatches = [];
  for (const [file, version] of Object.entries(versions)) {
    if (expected && version !== expected) {
      mismatches.push(`${file} (${version}) != expected ${expected}`);
    }
  }
  const unique = new Set(Object.values(versions));
  if (unique.size > 1) {
    mismatches.push(
      `version files disagree: ${JSON.stringify(versions, null, 2)}`,
    );
  }
  if (mismatches.length > 0) {
    throw new Error(`version check failed:\n${mismatches.join("\n")}`);
  }

  const result = {
    version: versions.package,
    ...versions,
    consistent: "true",
  };
  if (options.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  const lines = printKeyValue(options.output, result);
  process.stdout.write(`${lines}\n`);
}

function main(argv = process.argv.slice(2)) {
  const { options, positional } = parseArgs(argv);
  if (positional.length === 0) {
    throw new Error(
      "usage: release-version <parse|compare|check-files> ...",
    );
  }
  const [command, ...rest] = positional;
  switch (command) {
    case "parse":
      cmdParse(rest, options);
      break;
    case "compare":
      cmdCompare(rest);
      break;
    case "check-files":
      cmdCheckFiles(rest, options);
      break;
    default:
      throw new Error(
        `unknown command "${command}" (expected parse|compare|check-files)`,
      );
  }
}

const isMain =
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href;

if (isMain) {
  try {
    main();
  } catch (error) {
    console.error(`[release-version] ${error.message}`);
    process.exit(1);
  }
}

export { cmdCheckFiles, KNOWN_CHANNELS, parseVersion };
