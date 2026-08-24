<h1 align="center">dsh-tauri-gui</h1>

<p align="center">
  <a href="https://github.com/KevinT-hub/dsh-tauri-gui/releases"><img alt="Latest Release" src="https://img.shields.io/github/v/release/KevinT-hub/dsh-tauri-gui?style=flat-square&label=release"></a>
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
dsh-tauri-gui/
├─ src/                        React desktop shell UI
│  ├─ main.tsx / App.tsx       app entry and root component
│  ├─ features/                feature modules
│  │  ├─ setup/                environment detection / install-help screen
│  │  ├─ updater/              app & dsh update overlay
│  │  ├─ splash/               launch splash
│  │  └─ error/                error fallback screen
│  ├─ shared/                  bridge, theme, type definitions
│  ├─ ui/                      styles (tokens / global / animations)
│  └─ types/                   type declarations
├─ src-tauri/                  Rust / Tauri native shell
│  ├─ src/
│  │  ├─ app/                  config, events, lifecycle, state
│  │  ├─ commands/             Tauri commands (setup / geo / updater / shell)
│  │  ├─ core/                 error / fs / http / path / process / platform
│  │  ├─ detection/            dependency probing + install help (no bundled runtime)
│  │  ├─ engine/               dsh web lifecycle and protocol
│  │  ├─ geo/                  region detection (mirror policy)
│  │  ├─ ui/                   tray / windows / theme
│  │  └─ update/               auto-update (verify / download / dsh)
│  ├─ capabilities/            permission declarations
│  ├─ tests/                   Rust integration tests
│  └─ build.rs / Cargo.toml / tauri.conf.json / rust-toolchain.toml
├─ scripts/                    version / release / artifact tooling
├─ tests/                      Node scripts and cross-layer contract tests
├─ docs/                       user docs, architecture, install and release notes
├─ .github/workflows/          CI / Release / updater-channel
├─ public/ · dist/ · index.html   static assets and build output
└─ package.json · pnpm-lock.yaml · vite.config.ts · tsconfig*.json
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
