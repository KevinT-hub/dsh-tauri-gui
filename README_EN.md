<h1 align="center">dsh-tauri-gui</h1>

<p align="center">
  <a href="https://github.com/KevinT-hub/dsh-tauri-gui/actions/workflows/release.yml"><img alt="Release Build" src="https://github.com/KevinT-hub/dsh-tauri-gui/actions/workflows/release.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-263146?style=flat-square"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-4b6fff?style=flat-square">
  <img alt="Tauri v2" src="https://img.shields.io/badge/Tauri-v2-4b6fff?style=flat-square">
</p>

<p align="center">
  <a href="README.md">中文</a> · <strong>English</strong>
</p>

> A **Tauri v2** desktop client for DeepSeek Harness. The installer **does not bundle any runtime**: the shell detects and reuses your locally installed **Node.js / npm / pnpm / dsh**, then hosts the official DeepSeek Harness Web UI, Host service and plugin system in a native desktop window.
>
> Community-maintained open source — **not an official DeepSeek product**.

---

## What it is

[DeepSeek Harness (dsh)](https://github.com/deepseek-ai/deepseek-harness) is DeepSeek's official agent framework with a built-in Web UI, Host service and a pluggable plugin system. By default it runs in the browser via `npx @deepseek-ai/dsh web`.

This project is a **native desktop shell**: it detects and reuses the Node.js and the official `@deepseek-ai/dsh` CLI already installed on your machine. The shell owns environment detection, install help, window lifecycle, process management, app updates and the desktop experience; the official `dsh web` command powers the Web UI inside a Tauri window.

- No forks or modifications of the upstream dsh source;
- No copying or re-implementing of the dsh plugin mechanism;
- Agents, models, tools, sessions and the Web UI all come from the official `@deepseek-ai/dsh`.

> In one line: **this project = a native desktop shell + the official dsh**. The shell handles the experience; dsh handles the capabilities.

## Requirements

The shell does **not** bundle a runtime. On first launch it guides you through installing any missing external dependency:

| Dependency | Version | Notes |
| --- | --- | --- |
| **Node.js** | `^22.19.0` or `>=24` | Official dsh engine requirement |
| **npm** or **pnpm** | any | At least one is required (the UI shows both rows) |
| **dsh** | any | Official package `@deepseek-ai/dsh`, typically `npm install -g @deepseek-ai/dsh` |

See [Installation](docs/INSTALLATION.md) and [User Guide](docs/USER_GUIDE.md) for details.

## Features

- **Environment detection**: Node / npm / pnpm / dsh probed in parallel, with path, version and error per row;
- **Install help**: official install actions for missing dependencies (Node opens the official download page, dsh installs the official package) — every action requires explicit user confirmation;
- **Mirror policy**: geo detection picks npmmirror inside mainland China and the official registry abroad; geo failure safely falls back to official sources with a manual mirror switch;
- **One-time setup gate**: the detection screen shows once per app version; a failed detection never re-opens it, and the tray "re-detect environment" entry is always available;
- **Official engine**: launched as `dsh web --no-open --port <port>`, preserving `DSH_HOME`, the telemetry switch, registry, working directory and plugin-directory semantics;
- **Connect-or-spawn**: an existing official `dsh web` instance on the port is reused instead of spawning a second engine;
- **Crash recovery**: the engine restarts automatically on abnormal exit; an occupied port falls back to an OS-assigned one;
- **App auto-update**: GitHub-first with mirror fallback, verified by SHA-256 + minisign signatures, official GitHub Release sources only;
- **System tray**: show window, open Web UI, re-detect environment, restart engine, check updates, appearance theme, quit.

## Quick start

```sh
# Prerequisite: Node.js 22.19+ / 24+ and the official @deepseek-ai/dsh on your machine
npm install -g @deepseek-ai/dsh

# Install dependencies and run (development)
pnpm install
pnpm tauri dev
```

Release builds are downloaded from [Releases](https://github.com/KevinT-hub/dsh-tauri-gui/releases); follow the setup screen on first launch.

## Repository layout

```text
src/            React shell UI (app / features / shared / ui)
src-tauri/      Rust/Tauri native shell (app / commands / core / detection / engine / geo / ui / update)
scripts/        version, release and artifact tooling
tests/          Node scripts, release tooling and cross-layer contract tests
docs/           user docs, architecture, installation and release notes
.github/        CI / Release / updater-channel workflows
```

## Docs

- [Installation](docs/INSTALLATION.md)
- [User Guide](docs/USER_GUIDE.md)
- [Architecture](docs/ARCHITECTURE.md)
- [FAQ](docs/FAQ.md)
- [Plugin Compatibility](docs/PLUGIN_COMPATIBILITY.md)
- [Release & Update Channel](docs/RELEASE.md)
- [Contributing](CONTRIBUTING.md)

## Disclaimer

This is a **community-maintained open-source project, not an official DeepSeek product**. We do not modify the upstream `@deepseek-ai/dsh`; we only combine it through the official `dsh web` command. Use under [LICENSE](LICENSE).
