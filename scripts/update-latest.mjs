#!/usr/bin/env node
/**
 * Generates the Tauri updater `latest.json` for the GitHub channel,
 * mirroring the clash-verge-rev updater-release practice.
 *
 * Usage:
 *   node scripts/update-latest.mjs --from-dir <dir> --version <v> --tag <t> [--notes <notes>] [--base-url <url>] [--output <file>] [--pub-date <iso>]
 *   node scripts/update-latest.mjs --from-release [--tag <t>] [--output <file>]
 *
 * Outputs:
 *   latest.json by default, or the path passed via --output.
 *   Download URLs default to GitHub; pass --base-url to use a custom base URL.
 *
 * Required env (from-release mode):
 *   GH_TOKEN, GITHUB_OWNER, GITHUB_REPO
 */

import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const args = process.argv.slice(2);
const argValue = (name) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};

const GITHUB_OWNER = process.env.GITHUB_OWNER || 'KevinT-hub';
const GITHUB_REPO = process.env.GITHUB_REPO || 'dsh-tauri-gui';
const BASE_URL = argValue('--base-url');
const OUTPUT = argValue('--output');
const PUB_DATE = argValue('--pub-date');

const PLATFORM_RULES = [
  { re: /_Windows_x64-setup\.exe$/, key: 'windows-x86_64' },
  { re: /_macOS_x64\.app\.tar\.gz$/, key: 'darwin-x86_64' },
  { re: /_macOS_aarch64\.app\.tar\.gz$/, key: 'darwin-aarch64' },
  { re: /-Linux-x86_64\.AppImage$/, key: 'linux-x86_64' },
  { re: /_Linux_amd64\.deb$/, key: 'linux-x86_64-deb' },
];

function detectPlatform(fileName) {
  for (const rule of PLATFORM_RULES) {
    if (rule.re.test(fileName)) return rule.key;
  }
  return null;
}

function releaseDownloadUrl(tag, fileName, baseUrl) {
  const base = (baseUrl || `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases/download`).replace(/\/+$/, '');
  return `${base}/${tag}/${encodeURIComponent(fileName)}`;
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

function writeOutputs(version, notes, platforms, outputPath = 'latest.json', pubDate = new Date().toISOString()) {
  const payload = {
    version,
    notes: notes || `Release ${version}`,
    pub_date: pubDate,
    platforms,
  };

  const resolvedOutput = path.resolve(outputPath);
  fs.mkdirSync(path.dirname(resolvedOutput), { recursive: true });
  fs.writeFileSync(resolvedOutput, `${JSON.stringify(payload, null, 2)}\n`);

  console.log(`Generated ${resolvedOutput}:`);
  console.log(JSON.stringify(payload, null, 2));
}

function buildFromDir(dir, version, tag, notes, baseUrl, outputPath, pubDate) {
  if (!fs.existsSync(dir)) {
    throw new Error(`Artifacts directory not found: ${dir}`);
  }

  const platforms = {};
  const files = walkFiles(dir).sort();

  for (const fullPath of files) {
    const file = path.basename(fullPath);
    if (file.endsWith('.sig') || file === 'latest.json') continue;
    const platform = detectPlatform(file);
    if (!platform) {
      console.warn(`[skip] ${file} does not match any updater platform`);
      continue;
    }

    const sigPath = `${fullPath}.sig`;
    if (!fs.existsSync(sigPath)) {
      throw new Error(`Missing signature file for ${file}: ${sigPath}`);
    }
    const signature = fs.readFileSync(sigPath, 'utf8').trim();
    if (!signature) {
      throw new Error(`Signature file is empty for ${file}: ${sigPath}`);
    }

    platforms[platform] = {
      signature,
      sha256: createHash('sha256').update(fs.readFileSync(fullPath)).digest('hex'),
      url: releaseDownloadUrl(tag, file, baseUrl),
    };
    console.log(`[ok] ${platform} <- ${file}`);
  }

  if (Object.keys(platforms).length === 0) {
    throw new Error('No updater artifacts found in directory');
  }

  writeOutputs(version, notes, platforms, outputPath, pubDate);
}

function gh(argsList) {
  return execFileSync('gh', argsList, {
    encoding: 'utf8',
    env: { ...process.env, GH_TOKEN: process.env.GH_TOKEN },
  }).trim();
}

async function fetchSignature(url) {
  const response = await fetch(url, {
    headers: {
      Authorization: `Bearer ${process.env.GH_TOKEN}`,
      Accept: 'application/octet-stream',
    },
  });
  if (!response.ok) {
    throw new Error(`Failed to download signature ${url}: HTTP ${response.status}`);
  }
  return (await response.text()).trim();
}

async function buildFromRelease(tag) {
  if (!process.env.GH_TOKEN) {
    throw new Error('GH_TOKEN is required in --from-release mode');
  }

  const resolvedTag =
    tag ||
    gh([
      'api',
      `repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/latest`,
      '--jq',
      '.tag_name',
    ]);
  const version = resolvedTag.replace(/^v/, '');

  const releaseJson = gh([
    'api',
    `repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/tags/${resolvedTag}`,
    '--jq',
    '{tag_name, body, published_at, assets: [.assets[] | {name, browser_download_url, digest}]}',
  ]);
  const release = JSON.parse(releaseJson);
  const notes = release.body || `Release ${version}`;
  const pubDate = PUB_DATE || release.published_at || new Date().toISOString();

  const platforms = {};
  const assets = release.assets;

  for (const asset of assets) {
    if (asset.name.endsWith('.sig')) continue;
    const platform = detectPlatform(asset.name);
    if (!platform) continue;

    const sigAsset = assets.find((a) => a.name === `${asset.name}.sig`);
    if (!sigAsset) {
      throw new Error(`Missing signature asset for ${asset.name}`);
    }
    const signature = await fetchSignature(sigAsset.browser_download_url);
    const digest = String(asset.digest || '').replace(/^sha256:/i, '');
    if (!digest) {
      throw new Error(`Missing sha256 digest for ${asset.name}`);
    }

    platforms[platform] = {
      signature,
      sha256: digest,
      url: releaseDownloadUrl(resolvedTag, asset.name, BASE_URL),
    };
    console.log(`[ok] ${platform} <- ${asset.name}`);
  }

  if (Object.keys(platforms).length === 0) {
    throw new Error('No updater assets found on the release');
  }

  writeOutputs(version, notes, platforms, OUTPUT, pubDate);
}

async function main() {
  if (args.includes('--from-dir')) {
    const dir = argValue('--from-dir');
    const version = argValue('--version');
    const tag = argValue('--tag');
    const notes = argValue('--notes');
    if (!dir || !version || !tag) {
      throw new Error('--from-dir requires --from-dir/--version/--tag');
    }
    buildFromDir(dir, version, tag, notes, BASE_URL, OUTPUT, PUB_DATE);
  } else if (args.includes('--from-release')) {
    await buildFromRelease(argValue('--tag'));
  } else {
    throw new Error('Usage: --from-dir <dir> --version <v> --tag <t> [--base-url <url>] [--output <file>] | --from-release [--tag <t>] [--output <file>]');
  }
}

main().catch((error) => {
  console.error(error.message || error);
  process.exit(1);
});
