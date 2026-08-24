<h1 align="center">dsh-tauri-gui</h1>

<p align="center">
  <a href="https://github.com/KevinT-hub/dsh-tauri-gui/releases"><img alt="Latest Release" src="https://img.shields.io/github/v/release/KevinT-hub/dsh-tauri-gui?style=flat-square&label=release"></a>
  <a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-263146?style=flat-square"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-4b6fff?style=flat-square">
  <img alt="Tauri v2" src="https://img.shields.io/badge/Tauri-v2-4b6fff?style=flat-square">
</p>

<p align="center">
  <strong>中文</strong> · <a href="README_EN.md">English</a>
</p>

> 基于 **Tauri v2** 的 DeepSeek Harness 桌面客户端。安装包**不内置任何运行时**：桌面壳检测并使用你本机已安装的 **Node.js / npm / pnpm / dsh**，把官方 DeepSeek Harness 的本地 Web UI、Host 服务与插件系统装进原生桌面窗口。
>
> 社区维护的开源项目，**并非 DeepSeek 官方产品**。

---

## 它是什么

[DeepSeek Harness（dsh）](https://github.com/deepseek-ai/deepseek-harness) 是 DeepSeek 官方的智能体运行框架，自带 Web UI、Host 服务与可插拔的插件系统。官方默认通过 `npx @deepseek-ai/dsh web` 在浏览器中运行。

本项目是一个 **原生桌面壳**：它检测并复用你机器上已有的 Node.js 与官方 `@deepseek-ai/dsh` CLI，自己只负责环境检测、安装帮助、窗口生命周期、进程管理、自动更新与桌面体验，并通过官方 `dsh web` 命令把 Web UI 承载在 Tauri 窗口里。

- 不修改、不 fork 上游 dsh 源码；
- 不复制或替换 dsh 的插件机制；
- 核心的智能体、模型、工具、会话与 Web UI 全部来自官方 `@deepseek-ai/dsh`。

> 一句话总结：**本项目 = 「原生桌面壳」+「官方 dsh」**。壳负责体验，dsh 负责能力。

## 环境要求

桌面壳**不打包运行时**，首次启动会引导你安装缺失的外部依赖：

| 依赖 | 版本要求 | 说明 |
| --- | --- | --- |
| **Node.js** | `^22.19.0` 或 `>=24` | 官方 dsh 的引擎要求 |
| **npm** 或 **pnpm** | 任意可用版本 | 至少一个即可（UI 分别展示检测结果） |
| **dsh** | 任意可用版本 | 官方包 `@deepseek-ai/dsh`，通常 `npm install -g @deepseek-ai/dsh` |

安装完成后，首次启动的检测页会引导你完成剩余步骤；之后每次启动直接进入 Web UI。详见 [安装说明](docs/INSTALLATION.md) 与 [使用指南](docs/USER_GUIDE.md)。

## 特性

- **环境检测**：Node / npm / pnpm / dsh 四项并行探测，逐项展示路径、版本与错误原因；
- **安装帮助**：缺失依赖时给出官方安装动作（Node 打开官方下载页、dsh 执行官方包安装），所有安装动作均需用户确认；
- **镜像策略**：geo 检测国内自动使用 npmmirror 镜像，境外使用官方源，geo 失败时安全回退官方源并允许手动切换；
- **首次启动门禁**：每个应用版本只显示一次检测页，检测失败也不会反复打扰，托盘「重新检测环境」随时可手动触发；
- **官方引擎**：以 `dsh web --no-open --port <port>` 启动，保留 `DSH_HOME`、遥测开关、registry、工作目录与插件目录语义；
- **连接或拉起**：端口上已有官方 `dsh web` 实例时直接连接，避免配置/会话分散；
- **崩溃自重启**：引擎异常退出后自动重启，端口被占用时自动回退系统分配端口；
- **应用自动更新**：GitHub 优先 + 镜像回退，SHA-256 与 minisign 签名双重校验，仅官方 Release 来源；
- **系统托盘**：显示窗口、打开 Web UI、重新检测环境、重启引擎、检查更新、外观主题、退出。

## 快速开始

```sh
# 前置：本机已安装 Node.js 22.19+ / 24+，以及官方 @deepseek-ai/dsh
npm install -g @deepseek-ai/dsh

# 安装依赖并启动（开发模式）
pnpm install
pnpm tauri dev
```

发布版从 [Releases](https://github.com/KevinT-hub/dsh-tauri-gui/releases) 下载安装包，首次启动按检测页指引即可。

## 仓库结构

```text
dsh-tauri-gui/
├─ src/                        React 桌面壳层 UI
│  ├─ main.tsx / App.tsx       应用入口与根组件
│  ├─ features/                功能模块
│  │  ├─ setup/                环境检测 / 安装帮助页面
│  │  ├─ updater/              应用与 dsh 更新覆盖层
│  │  ├─ splash/               启动闪屏
│  │  └─ error/                异常兜底页
│  ├─ shared/                  前后端桥接、主题、类型定义
│  ├─ ui/                      样式（tokens / global / animations）
│  └─ types/                   类型声明
├─ src-tauri/                  Rust / Tauri 原生壳层
│  ├─ src/
│  │  ├─ app/                  配置、事件、生命周期、状态
│  │  ├─ commands/             Tauri 命令（setup / geo / updater / shell）
│  │  ├─ core/                 错误 / 文件 / HTTP / 路径 / 进程 / 平台
│  │  ├─ detection/            依赖探测 + 安装帮助（不内置运行时）
│  │  ├─ engine/               dsh web 生命周期与协议
│  │  ├─ geo/                  地区识别（镜像策略）
│  │  ├─ ui/                   托盘 / 窗口 / 主题
│  │  └─ update/               自动更新（校验 / 下载 / dsh）
│  ├─ capabilities/            权限声明
│  ├─ tests/                   Rust 集成测试
│  └─ build.rs / Cargo.toml / tauri.conf.json / rust-toolchain.toml
├─ scripts/                    版本 / Release / 产物处理脚本
├─ tests/                      Node 脚本与跨层契约测试
├─ docs/                       用户文档、架构、安装与发布说明
├─ .github/workflows/          CI / Release / 更新通道
├─ public/ · dist/ · index.html   静态资源与构建产物
└─ package.json · pnpm-lock.yaml · vite.config.ts · tsconfig*.json
```

## 文档

- [安装说明](docs/INSTALLATION.md)
- [使用指南](docs/USER_GUIDE.md)
- [架构说明](docs/ARCHITECTURE.md)
- [常见问题](docs/FAQ.md)
- [插件兼容性](docs/PLUGIN_COMPATIBILITY.md)
- [发布与更新通道](docs/RELEASE.md)
- [贡献与开发](CONTRIBUTING.md)

## 免责声明

这是**社区维护的开源项目，并非 DeepSeek 官方产品**。我们不对上游 `@deepseek-ai/dsh` 做任何修改，仅通过官方 `dsh web` 命令与之组合。请遵循 [LICENSE](LICENSE) 使用本项目。
