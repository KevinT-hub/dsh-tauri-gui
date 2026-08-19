<h1 align="center">DeepSeek Harness Tauri Desktop</h1>

<p align="center">
  <a href="https://github.com/KevinT-hub/dsh-tauri-gui/actions/workflows/release.yml"><img alt="Release Build" src="https://github.com/KevinT-hub/dsh-tauri-gui/actions/workflows/release.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-263146?style=flat-square"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-4b6fff?style=flat-square">
  <img alt="Tauri v2" src="https://img.shields.io/badge/Tauri-v2-4b6fff?style=flat-square">
</p>

<p align="center">
  <strong>中文</strong> · <a href="README_EN.md">English</a>
</p>

> 基于 **Tauri v2** 的 DeepSeek Harness 桌面客户端。内置自包含运行时，开箱即用；把官方 DeepSeek Harness 的本地 Web UI、Host 服务与插件系统装进原生桌面窗口。
>
> 社区维护的开源项目，**并非 DeepSeek 官方产品**。

---

## 它是什么

[DeepSeek Harness（dsh）](https://github.com/deepseek-ai/deepseek-harness) 是 DeepSeek 官方的智能体运行框架，自带 Web UI、Host 服务与可插拔的插件系统。官方默认通过 `npx @deepseek-ai/dsh web` 在浏览器中运行。

本项目是一个 **原生桌面壳**：它把官方 dsh 以固定版本原样运行，自己只负责窗口、系统托盘、自动更新、运行时管理与桌面工作配置，并通过官方 `dsh web` 命令把 Web UI 承载在 Tauri 窗口里。

- 不修改、不 fork 上游 dsh 源码；
- 不魔改 dsh 的插件机制；
- 核心的智能体、模型、工具、会话与 Web UI 全部来自官方 `@deepseek-ai/dsh`。

> 一句话总结：**本项目 = 「原生桌面壳」+「官方 dsh」**。壳负责体验，dsh 负责能力。

## 主要功能

- **自包含运行时**：内置 Node.js + pnpm + `@deepseek-ai/dsh`，用户无需自行安装 Node / npm / dsh 即可使用（内置运行时模式）。
- **双运行时模式**：`内置运行时`（推荐，版本固定、稳定）与 `系统运行时`（复用 PATH 中的 node / npm / dsh，适合自行管理环境的进阶用户）。首次启动选择一次，之后可随时从托盘切换。
- **官方 Web UI 原生承载**：启动 `dsh web` 并渲染在 Tauri 窗口中，体验与浏览器一致；若端口上已有运行的 dsh 实例则直接连接，不会重复拉起。
- **系统托盘与单例**：双击托盘图标唤起窗口；重复打开时聚焦已有窗口而非再起一个进程。
- **应用自动更新**：通过 GitHub Releases + 镜像源分发，使用 **minisign 签名 + SHA-256 双重校验**；仅在有新版本时显示更新按钮。
- **dsh 核心热更新**：在内置运行时下可一键把 `@deepseek-ai/dsh` 升级到最新版本，采用「暂存安装 + 备份回滚」确保中途失败不破坏现有环境。
- **首次引导与检查清单**：仅在新版本首次启动时展示检测 / 许可清单，避免每次打开都打扰。
- **外观主题**：亮色 / 暗色 / 跟随系统，与操作系统实时同步。
- **崩溃自重启**：引擎异常退出后自动重启（可关闭）。
- **隐私优先**：遥测默认关闭；所有日志对密钥做脱敏；Webview 仅允许加载本机 `127.0.0.1` 上的 dsh 地址。

## 下载与安装

当前由 CI 为每个发布标签构建 **Windows（x64 / ARM64）**、**macOS（Intel / Apple Silicon）** 与 **Linux（x64）** 安装包。

1. 前往 **[Releases](https://github.com/KevinT-hub/dsh-tauri-gui/releases)** 下载对应平台的安装包；
2. 安装并启动；
3. 首次启动会让你选择运行时，之后自动打开 dsh Web UI。

> 还没有发布版本？请参考 [CONTRIBUTING.md](CONTRIBUTING.md) 从源码构建。

## 快速开始

1. 启动应用，首次在「选择运行时」中选择 **内置运行时（推荐）**；
2. 应用自动检测环境并启动 dsh 引擎；
3. 引擎就绪后，Tauri 窗口会直接呈现官方 dsh Web UI，开始对话即可；
4. 日常可通过 **系统托盘** 菜单进行：显示主窗口、打开 Web UI、重启引擎、检查更新、切换外观与运行时、退出。

## 文档

| 主题 | 说明 |
| --- | --- |
| [架构说明](docs/ARCHITECTURE.md) | 桌面壳与 dsh 的边界、运行时、启动流程、安全模型、更新通道 |
| [使用指南](docs/USER_GUIDE.md) | 安装、首次启动、日常使用、核心热更新、数据与目录 |
| [常见问题](docs/FAQ.md) | 启动失败、端口占用、切换运行时、更新与隐私等排查 |
| [贡献与开发](CONTRIBUTING.md) | 本地开发环境、构建、发布流程与代码约定 |

## 与官方项目的关系

本项目基于 [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) 构建，并复用其官方 `dsh web` 能力与插件生态。我们对上游 **不做任何修改** —— 官方 dsh 以固定版本原样运行，桌面壳仅通过官方命令与之组合。

- 若你的目标是 **命令行** 运行 dsh，或参与 **核心功能** 开发，请优先查看官方仓库。
- 若你的目标是 **原生桌面体验**（窗口、托盘、更新、免装环境），本项目更合适。

## 免责声明

> 本项目是基于 DeepSeek Harness 构建的社区桌面版本，**并非 DeepSeek 官方产品**，与 DeepSeek 官方没有隶属关系，也未获得其背书。
>
> 本项目完全开源免费。如有人以任何形式向你出售本软件，请拒绝交易。
>
> DeepSeek 是 DeepSeek AI 的商标。DSH Tauri Desktop 是独立的社区项目。

## License

[MIT](LICENSE) © 2026 KevinT-hub
