# DeepSeek Harness Tauri Desktop

把官方 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`@deepseek-ai/dsh`）装进一个真正开箱即用的 Tauri 桌面应用：不需要预装 Node.js / npm / pnpm，双击安装包即可使用，与官方 WebUI / CLI 共用同一份数据。

[![Latest release](https://img.shields.io/github/v/release/KevinT-hub/dsh-tauri-gui)](https://github.com/KevinT-hub/dsh-tauri-gui/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/KevinT-hub/dsh-tauri-gui/total)](https://github.com/KevinT-hub/dsh-tauri-gui/releases)
[![Windows](https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white)](#下载)
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white)](#下载)
[![Linux](https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black)](#下载)
[![Community](https://img.shields.io/badge/community-non--official-orange)](#免责声明)

**[⬇️ 立即下载](https://github.com/KevinT-hub/dsh-tauri-gui/releases/latest)** · [功能一览](#功能一览) · [为什么是它](#为什么是它) · [常见问题](#常见问题)

---

## 这是什么

DeepSeek Harness 官方形态是命令行 + 网页版。本项目的定位：

> 一个 Tauri v2 桌面壳，把官方 `dsh` 引擎、Node.js 运行时和 pnpm 一起打进安装包；首次启动自动解压，界面直接用系统 WebView 承载官方 WebUI，不对前端做任何重写。

## 功能一览

- **开箱即用**：内置 Node.js 22.19 + `@deepseek-ai/dsh` + pnpm，首次启动自动解压到用户目录，不需要命令行、不需要懂端口。
- **与官方数据完全通用**：引擎默认使用 `~/.dsh`（与官方 `dsh web` / CLI 相同），Profile、会话、设置、凭据、插件全部共享；官方 WebUI 已在 3080 运行时直接连接它，不会启动第二个引擎。
- **插件 100% 兼容**：加载、热重载、卸载全部由官方 CLI 完成，`dsh plugin` 与 WebUI 插件页开箱即用。
- **原生桌面体验**：Tauri 2 + 系统 WebView，托盘驻留、关窗不退出、单实例保护、后台运行。
- **崩溃自己爬起来**：引擎子进程异常退出后自动重启，失败也会给出原因。
- **主题跟随**：亮色 / 暗色 / 跟随系统，并读取官方 WebUI 持久化的 `ui-theme.preference` 同步。
- **检测页面按版本出现**：首次启动与软件更新后的第一次启动显示检测引导页，之后直接进入 WebUI，无黑屏。
- **双更新通道**：应用本体走 Tauri updater（GitHub + 镜像 + SHA-256 + minisign），dsh 核心用内置 npm 热更新（staging + swap）。
- **供应链加固**：Node 下载校验官方 SHASUMS256，npm 安装镜像回退，运行时打包前自动裁剪（约 367MB → 239MB，压缩包约 72MB，较原始 npm 树减少约 35%）。
- **安全默认值**：CSP、导航白名单、日志脱敏、配置备份恢复。

## 为什么是它

- **零依赖安装**：同类项目常见“先装 Node 再 npx”或首次联网下载运行时；本项目把完整运行时打进安装包，离线也能完成首启。
- **官方 UI 一比一**：只做引擎监督与生命周期管理，不重写前端，官方插件生态不受影响。
- **Tauri 而非 Electron**：安装包更小，内存占用更低。
- **三平台自动更新**：Windows / macOS / Linux 均产出规范安装包并维护 `latest.json`；macOS 额外提供 `.app.tar.gz` 更新通道，DMG 安装后也能自动更新。

## 下载

前往 [Releases 页面](https://github.com/KevinT-hub/dsh-tauri-gui/releases/latest)，按系统选择安装包：

| 系统 | 安装包 |
| --- | --- |
| Windows | `dsh-tauri-gui_<version>_Windows_x64-setup.exe`、`dsh-tauri-gui_<version>_Windows_x64.msi` |
| macOS | `dsh-tauri-gui_<version>_macOS_x64.dmg`、`dsh-tauri-gui_<version>_macOS_aarch64.dmg`（另附同名 `.app.tar.gz` 供自动更新通道使用，安装请选 `.dmg`） |
| Linux | `dsh-tauri-gui-<version>-Linux-x86_64.AppImage`、`dsh-tauri-gui_<version>_Linux_amd64.deb`（另生成 rpm） |

所有安装包均附带 `.sig` 签名文件。


## 快速开始（开发）

前置要求：Node.js 22.19+（或 24+）、Rust stable、pnpm。

```sh
pnpm install
pnpm run runtime:prepare   # 下载 Node，并按版本清单安装 @deepseek-ai/dsh + pnpm 到 ~/.dsh-tauri-gui/runtime
pnpm tauri dev
```

首次启动会看到检测引导页；引擎就绪后自动进入官方 WebUI。

## 打包

```sh
pnpm run runtime:package   # 生成 src-tauri/resources/runtime.tar.gz（每个目标平台执行一次）
pnpm tauri build
```

发布流水线由 GitHub Actions 完成，产物按上述命名规范重命名并校验。详见 [docs/PACKAGING.md](docs/PACKAGING.md)。

## 用户数据目录

| 路径 | 用途 |
| --- | --- |
| `~/.dsh/` | 引擎数据，与官方 WebUI / CLI 共享：Profile、会话、`settings.yaml`、凭据 |
| `~/.dsh-tauri-gui/runtime/` | Node + `@deepseek-ai/dsh` + pnpm，支持热更新 |
| `~/.dsh-tauri-gui/logs/` | 桌面壳与引擎日志 |
| `~/.dsh-tauri-gui/config.json` | 桌面壳自身配置 |

引擎从 `DSH_HOME=~/.dsh` 启动（可用 config.json 的 `engineHome` 覆盖），因此配置路径、格式与热重载行为与官方 `dsh` 完全一致。

## 更新机制

- **应用自更新**：Tauri updater 从 GitHub `update` release 读取 `latest.json`，GitHub 优先、镜像兜底，校验 SHA-256 与 minisign 签名后安装并 relaunch。
- **dsh 核心热更新**：用内置 npm 将 `runtime/app` 更新到 registry 最新 `@deepseek-ai/dsh`，staging + swap 原子替换后重启引擎；默认镜像 `registry.npmmirror.com`，可用 `npmRegistry` 配置修改。
- **更新按钮**：检测到新版本时右下角出现独立悬浮按钮，不注入官方 WebUI。

## 常见问题

**需要安装 Node.js 吗？**

不需要。安装包内已包含 Node.js 22.19 + pnpm。

**会和官方 WebUI 冲突吗？**

不会。两者共用 `~/.dsh` 数据；如果 3080 已有官方 `dsh web` 在运行，桌面应用会直接连接它；被其他程序占用时自动回退到随机端口。

**插件兼容吗？**

兼容。插件加载 / 热重载 / 卸载全部由官方 `dsh` 完成，桌面壳不重写任何加载逻辑。

**退出托盘后终端出现 `Failed to unregister class Chrome_WidgetWin_0 ... 1412`？**

这是 WebView2 / Chromium 退出时的无害日志，不影响使用与退出。

**GitHub Actions 会产出哪些安装包？**

Windows：setup.exe + msi；macOS：x64 / aarch64 dmg + 同名 app.tar.gz；Linux：AppImage + deb（另生成 rpm），均含 `.sig`。

## 免责声明

本项目是社区项目，与 DeepSeek / DeepSeek Harness 官方无隶属关系，不提供任何官方支持与担保。核心引擎来自官方 npm 包 `@deepseek-ai/dsh`，本项目仅作为桌面壳封装。相关商标与权利归各自所有者。
