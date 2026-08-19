# 使用指南

本文面向普通用户，介绍如何安装、首次启动、日常使用，以及如何进行 dsh 核心热更新、应用自动更新，并说明数据与目录位置。

## 支持平台

| 平台 | 架构 | 安装包形式 |
| --- | --- | --- |
| Windows | x64 / ARM64 | NSIS 安装程序（`.exe`）/ MSI |
| macOS | Intel / Apple Silicon | DMG（`.dmg`）/ `.app.tar.gz` |
| Linux | x64 | AppImage / `.deb` / `.rpm` |

> 安装包由 CI 在每个发布标签下自动构建。前往 [Releases](https://github.com/KevinT-hub/dsh-tauri-gui/releases) 获取。

## 安装

1. 在 Releases 下载对应平台的安装包；
2. 按系统常规方式安装（Windows 运行安装程序、macOS 拖入 Applications、Linux 使用对应包管理器或 AppImage）；
3. 启动应用。

如需从源码构建，请参阅 [CONTRIBUTING.md](../CONTRIBUTING.md)。

## 首次启动与运行时选择

首次启动时，会先弹出「选择运行时」对话框：

- **内置运行时（推荐）**：使用应用自带的 Node + pnpm + dsh，版本固定、无需额外安装，最省心。
- **系统运行时（高级）**：使用你系统在 PATH 中已安装的 node / npm / dsh，适合已自行管理 Node 环境的用户。

选择后，应用会：

1. 检测内置/系统运行时是否可用；
2. 启动 dsh 引擎（`dsh web`）；
3. 引擎就绪后，在窗口中直接呈现官方 dsh Web UI。

> 该选择**只需在首次启动时确认一次**。之后想换运行时，可随时通过**系统托盘菜单 → 运行时**切换（切换会做可用性预检并重启应用）。

## 日常使用

### 主窗口与托盘

- **系统托盘**菜单提供：显示主窗口、打开 Web UI（系统默认浏览器）、重启引擎、检查更新、外观（亮色/暗色/跟随系统）、运行时（内置/系统）、退出。
- **双击托盘图标**即可唤起已最小化的主窗口。
- 默认「关闭窗口」是**最小化到托盘**（`minimizeToTray`），而非退出；如需真正退出，用托盘菜单的「退出」。

### 外观主题

在托盘菜单「外观」中切换 亮色 / 暗色 / 跟随系统。主题会与操作系统实时同步（系统主题变化时窗口外观自动跟随）。

### 重启引擎

当 dsh Web UI 无响应或需要重新加载时，可用托盘「重启引擎」，或前端错误/引导界面的「重启引擎」按钮。引擎默认在崩溃后自动重启（`restartOnCrash`）。

### 端口与工作目录

- 默认 dsh Web UI 监听 **3080**。若被占用，桌面壳会自动回退到系统分配端口（不会报错退出）。
- 引擎默认在你的**用户主目录**下运行；可在配置中设置 `defaultWorkspace` 指定固定工作目录。
- dsh 的数据目录默认是 `~/.dsh`（`engineHome`），可通过配置改用其他路径。

## dsh 核心热更新

在内置运行时模式下，你可以把 `@deepseek-ai/dsh` 升级到最新版本，而无需重装整个应用：

1. 托盘菜单「检查更新」会同时检查应用与 dsh 核心；
2. 当发现新的 dsh 版本时，引导/状态界面会提示「发现新版本 dsh x.y.z（当前 a.b.c）」；
3. 确认后，桌面壳会：
   - 停止当前引擎（释放 node-pty 等被锁定的原生模块）；
   - 在 `.app-staging-*` 目录暂存安装新版本；
   - 校验产物后，与旧 `app/` 交换并保留 `.app-old-*` 备份；
   - 更新 `runtime.json` 并重启引擎。

> 若中途失败，会自动回滚到旧版本，不会影响现有环境。
> **系统运行时模式下不提供热更新**——请用你的系统包管理器更新 dsh。

## 应用自动更新

- 桌面壳启动后会在后台探测 GitHub Releases 的 `latest.json`；
- 仅当**确有新版本**时，窗口右下角出现更新浮层按钮；
- 点击后下载安装包，并用 **SHA-256 + minisign 签名** 双重校验（下载地址限定为官方 GitHub Release）。
- 若官方 GitHub 访问缓慢，会自动尝试社区镜像源（仅作为无凭证的下载代理）。

## 数据与目录

| 路径 | 内容 |
| --- | --- |
| `~/.dsh-tauri-gui/config.json` | 桌面壳配置（主题、运行时、端口等） |
| `~/.dsh-tauri-gui/runtime/` | 内置运行时（仅 bundled 模式使用） |
| `~/.dsh-tauri-gui/logs/` | 脱敏后的运行日志 |
| `~/.dsh/`（默认 `engineHome`） | 官方 dsh 数据：会话、插件、`cordis.patch.yml`、`settings.yaml` 等 |

> 想让桌面壳与你在用的官方 `dsh` CLI / 其他 GUI 共享同一套会话与插件？把 `engineHome` 指向同一个 `~/.dsh` 目录即可。

## 设置项速查

完整字段见 [架构说明](ARCHITECTURE.md#配置)。常用项：

- **关闭即最小化到托盘**：`minimizeToTray`（默认开）
- **启动自动拉起引擎**：`autoStartEngine`（默认开）
- **崩溃自重启**：`restartOnCrash`（默认开）
- **关闭遥测**：`telemetryDisabled`（默认开）
- **npm 镜像**：`npmRegistry`（默认 npmmirror）
- **Web UI 端口**：`webuiPort`（默认 3080）
- **外观**：`uiTheme`（默认 system）
- **运行时来源**：`runtimeMode`（默认 bundled）
