<h1 align="center">DeepSeek Harness Tauri Desktop</h1>

<p align="center">
  <a href="https://github.com/KevinT-hub/dsh-tauri-gui/actions/workflows/release.yml"><img alt="Release Build" src="https://github.com/KevinT-hub/dsh-tauri-gui/actions/workflows/release.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-263146?style=flat-square"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-4b6fff?style=flat-square">
  <img alt="Tauri v2" src="https://img.shields.io/badge/Tauri-v2-4b6fff?style=flat-square">
</p>

<p align="center">
  <a href="README.md">中文</a> · <strong>English</strong>
</p>

> A **Tauri v2** desktop client for DeepSeek Harness. Ships a self-contained runtime and runs out of the box — it hosts the official DeepSeek Harness local Web UI, Host service, and plugin system inside a native desktop window.
>
> A community-maintained open-source project, **not an official DeepSeek product**.

---

## What is it

[DeepSeek Harness (dsh)](https://github.com/deepseek-ai/deepseek-harness) is DeepSeek's official agent runtime: it ships a Web UI, a Host service, and a pluggable plugin system. Officially you run it in the browser via `npx @deepseek-ai/dsh web`.

This project is a **native desktop shell**: it runs the official dsh at a pinned version exactly as-is, and only owns the window, system tray, auto-update, runtime management, and desktop configuration. It launches the official `dsh web` command and hosts its Web UI inside a Tauri window.

- It does **not** modify or fork the upstream dsh source.
- It does **not** hack the dsh plugin mechanism.
- All agents, models, tools, sessions, and the Web UI come from the official `@deepseek-ai/dsh`.

> In one sentence: **this project = "native desktop shell" + "official dsh"**. The shell owns the experience; dsh owns the capability.

## Key features

- **Self-contained runtime**: bundles Node.js + pnpm + `@deepseek-ai/dsh`, so no separate Node / npm / dsh install is required (bundled runtime mode).
- **Two runtime modes**: `bundled` (recommended — pinned, stable) and `system` (reuses node / npm / dsh from PATH, for advanced users who manage their own environment). Chosen once on first launch, switchable anytime from the tray.
- **Official Web UI, native**: launches `dsh web` and renders it in the Tauri window — same experience as the browser. If a dsh instance is already serving the port, it connects instead of spawning a second one.
- **System tray & single-instance**: double-click the tray icon to raise the window; a second launch focuses the existing window instead of starting a new process.
- **App auto-update**: distributed via GitHub Releases + mirror sources, verified with **minisign signatures and SHA-256**. The update button only appears when a new version exists.
- **dsh core hot-update**: in bundled mode you can upgrade `@deepseek-ai/dsh` to the latest version in one click, using a "staged install + backup rollback" scheme so a failed update never corrupts the existing environment.
- **First-run guidance & checklist**: shown only on the first launch of a new version, so daily use is never interrupted.
- **Appearance themes**: light / dark / system, synced live with the OS.
- **Crash auto-restart**: the engine restarts automatically after an unexpected exit (can be disabled).
- **Privacy-first**: telemetry is off by default; logs are redacted for secrets; the Webview only loads the dsh address on local `127.0.0.1`.

## Download & install

CI builds installers for every release tag across **Windows (x64 / ARM64)**, **macOS (Intel / Apple Silicon)**, and **Linux (x64)**.

1. Go to **[Releases](https://github.com/KevinT-hub/dsh-tauri-gui/releases)** and download the installer for your platform;
2. Install and launch;
3. On first launch you choose a runtime, after which the dsh Web UI opens automatically.

> No release yet? Build from source following [CONTRIBUTING.md](CONTRIBUTING.md).

## Quick start

1. Launch the app; on first run choose **Bundled runtime (recommended)** in the "Select runtime" dialog;
2. The app detects the environment and starts the dsh engine;
3. Once the engine is ready, the Tauri window shows the official dsh Web UI — start chatting;
4. Use the **system tray** menu for: show main window, open Web UI, restart engine, check for updates, switch appearance / runtime, and quit.

## Documentation

| Topic | Description |
| --- | --- |
| [Architecture](docs/ARCHITECTURE.md) | Shell/dsh boundary, runtime, boot flow, security model, update channels |
| [User guide](docs/USER_GUIDE.md) | Install, first launch, daily use, core hot-update, data & directories |
| [FAQ](docs/FAQ.md) | Launch failures, port conflicts, runtime switching, updates & privacy |
| [Contributing](CONTRIBUTING.md) | Local dev environment, build, release flow, code conventions |

## Relationship to the official project

This project is built on [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) and reuses its official `dsh web` capability and plugin ecosystem. We make **no modifications** to upstream — the official dsh runs at a pinned version, and the desktop shell only composes with it via official commands.

- If you want to run dsh from the **command line**, or contribute to **core features**, prefer the official repository.
- If you want a **native desktop experience** (window, tray, update, zero-setup), this project fits better.

## Disclaimer

> This project is a community desktop build based on DeepSeek Harness. It is **not an official DeepSeek product**, has no affiliation with DeepSeek, and is not endorsed by them.
>
> This project is fully open-source and free. If anyone tries to sell you this software in any form, please refuse.
>
> DeepSeek is a trademark of DeepSeek AI. DSH Tauri Desktop is an independent community project.

## License

[MIT](LICENSE) © 2026 KevinT-hub
