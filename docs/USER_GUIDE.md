# 使用指南

本文面向普通用户，介绍如何安装外部依赖、首次启动、日常使用，以及应用自动更新与数据目录位置。

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

> 安装包**不包含** Node.js / dsh 运行时。首次启动前请先确认本机依赖，详见 [安装说明](INSTALLATION.md)。

## 首次启动与环境检测

首次启动（或升级到新版本后）会显示**环境检测页**，逐项检查：

1. **Node.js**（必须，`^22.19.0 || >=24`）；
2. **npm / pnpm**（至少一个可用，UI 分别展示）；
3. **dsh**（官方 `@deepseek-ai/dsh` CLI）。

每一项展示状态（检查中 / 已通过 / 未找到 / 版本不满足）、可执行文件路径、版本与错误原因。全部通过后出现 **「进入 Harness →」** 按钮，点击后启动官方 dsh Web UI。

- 依赖缺失时，检测页给出**安装帮助**：Node 打开官方下载页、dsh 提供一键安装官方包等；所有安装动作都需你确认。
- 安装完成后点击「重新检测」，通过后即可进入。
- 检测页**每个应用版本只显示一次**；即使检测失败，下次启动也不会自动重复弹出，可随时通过**系统托盘 → 重新检测环境**手动重试。

## 日常使用

### 主窗口与托盘

- **系统托盘**菜单提供：显示主窗口、打开 Web UI（系统默认浏览器）、重启引擎、重新检测环境、检查更新、外观（亮色/暗色/跟随系统）、退出。
- **双击托盘图标**即可唤起已最小化的主窗口。
- 默认「关闭窗口」是**最小化到托盘**（`minimizeToTray`），而非退出；如需真正退出，用托盘菜单的「退出」。

### 外观主题

在托盘菜单「外观」中切换 亮色 / 暗色 / 跟随系统。主题会与操作系统实时同步（系统主题变化时窗口外观自动跟随），并同步写入官方 `$DSH_HOME/settings.yaml` 的 `ui-theme.preference`。

### 重启引擎

当 dsh Web UI 无响应或需要重新加载时，可用托盘「重启引擎」，或前端错误页的「重启引擎」按钮。引擎默认在崩溃后自动重启（`restartOnCrash`）。

### 端口与工作目录

- 默认 dsh Web UI 监听 **3080**。若被占用，桌面壳会自动回退到系统分配端口（不会报错退出）。
- 引擎默认在你的**用户主目录**下运行；可在配置中设置 `defaultWorkspace` 指定固定工作目录。
- dsh 的数据目录默认是 `~/.dsh`（`engineHome`），可通过配置改用其他路径。

## dsh 核心的更新

桌面壳**不管理 dsh 的更新**——dsh 由你的包管理器负责。当检测页提示 dsh 版本不满足或你想升级时：

```sh
npm install -g @deepseek-ai/dsh@latest
# 或使用 pnpm：
pnpm add -g @deepseek-ai/dsh@latest
```

安装后通过托盘「重新检测环境」或重启应用即可生效。镜像用户可使用 `--registry` 指定镜像源。

## 应用自动更新

- 桌面壳启动后会在后台探测 GitHub Releases 的 `latest.json`；
- 仅当**确有新版本**时，窗口右下角出现更新浮层按钮；
- 点击后下载安装包，并用 **SHA-256 + minisign 签名** 双重校验（下载地址限定为官方 GitHub Release）。
- 若官方 GitHub 访问缓慢，会自动尝试社区镜像源（仅作为无凭证的下载代理）。

## 数据与目录

| 路径 | 内容 |
| --- | --- |
| `~/.dsh-tauri-gui/config.json` | 桌面壳配置（主题、端口、registry 等） |
| `~/.dsh-tauri-gui/logs/` | 脱敏后的运行日志 |
| `~/.dsh/`（默认 `engineHome`） | 官方 dsh 数据：会话、插件、`cordis.patch.yml`、`settings.yaml` 等 |

> 想让桌面壳与你在用的官方 `dsh` CLI / 其他 GUI 共享同一套会话与插件？把 `engineHome` 指向同一个 `~/.dsh` 目录即可。

## 设置项速查

完整字段见 [架构说明](ARCHITECTURE.md#配置)。常用项（位于 `~/.dsh-tauri-gui/config.json`）：

- **关闭即最小化到托盘**：`minimizeToTray`（默认开）
- **启动自动拉起引擎**：`autoStartEngine`（默认开）
- **崩溃自重启**：`restartOnCrash`（默认开）
- **关闭遥测**：`telemetryDisabled`（默认开）
- **npm registry**：`npmRegistry`（默认官方源，geo 国内自动镜像）
- **Web UI 端口**：`webuiPort`（默认 3080）
- **外观**：`uiTheme`（默认 system）
