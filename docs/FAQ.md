# 常见问题（FAQ）

> 排查前建议先打开 **托盘菜单 → 打开日志目录**，查看 `~/.dsh-tauri-gui/logs/` 下的脱敏日志——绝大多数启动/连接问题都能在日志里看到原因。

## 启动后一直黑屏 / 卡在加载界面？

1. 确认是否处于「首次启动」或「版本升级后首次启动」：此时会显示**环境检测页**，属正常流程，等待检测完成即可。
2. 若长时间无进展，打开日志目录，搜索关键字：
   - `环境检测未全部通过` / `未在 PATH 中找到`：缺少 Node.js / npm / pnpm / dsh，按检测页的安装帮助操作，完成后「重新检测」。
   - `EADDRINUSE` / `address already in use` / `EACCES`：端口被占用，桌面壳会自动回退到系统分配端口，一般无需处理；若想固定端口，在配置中改 `webuiPort`。
3. 强制重启：托盘「退出」后重新打开。

## 提示找不到 Node.js / npm / dsh？

桌面壳使用你**本机 PATH 中**的依赖，不会自动安装。请：

- **Node.js**：前往 [nodejs.org](https://nodejs.org) 安装 22.19+ 或 24+（国内可用 npmmirror 镜像）；
- **dsh**：`npm install -g @deepseek-ai/dsh`；
- 安装完成后，通过托盘「重新检测环境」或重启应用。

详见 [安装说明](INSTALLATION.md)。

## Node 版本不满足要求？

官方 dsh 要求 Node `^22.19.0 || >=24`。请升级 Node（或用 nvm/fnm 等版本管理工具切换），然后重新检测。

## 端口 3080 被占用怎么办？

桌面壳检测到 3080 被占用（且不是自己的 dsh 实例）时，会自动改用系统分配的端口，通常不会影响使用。如需固定端口：

- 关闭占用 3080 的其他程序；或
- 修改 `~/.dsh-tauri-gui/config.json` 中的 `webuiPort` 为其他值（如 `4080`）或 `0`（始终系统分配），重启应用生效。

> 注意：若 3080 上已有**另一个 dsh 实例**在运行，桌面壳会直接连接它，而不是再拉起一个——这是预期行为。

## 为什么国内下载慢 / 用的什么镜像？

桌面壳启动时会做 **geo 国家码检测**：国内自动使用 npmmirror 镜像源（`https://registry.npmmirror.com`），境外使用官方源。geo 检测失败时默认官方源，你可以在设置/检测页手动切换镜像。geo 只影响源选择，**不会**阻塞应用启动。

## 应用更新检查 / 下载失败？

- 桌面壳只在校验通过后才更新：**SHA-256 + minisign 签名** 双重校验，且下载地址限定为官方 GitHub Release。任何校验失败都会放弃更新并保留旧版本。
- 若 GitHub 访问慢，会自动尝试社区镜像源（ghfast.top 等），镜像仅作无凭证的下载代理。
- 仍失败：可前往 [Releases](https://github.com/KevinT-hub/dsh-tauri-gui/releases) 手动下载安装包覆盖安装。

## dsh 核心怎么更新？

桌面壳**不提供 dsh 热更新**，dsh 由你的包管理器管理：

```sh
npm install -g @deepseek-ai/dsh@latest
```

更新后通过托盘「重新检测环境」或重启应用生效。

## 遥测和隐私如何？

- **遥测默认关闭**（`telemetryDisabled = true`，对应环境变量 `DSH_TELEMETRY_DISABLED=1`）。
- 所有日志在写入前都经过**密钥脱敏**（`core/redact.rs`），API Key、token 等不会明文记录。
- 应用更新仅下载官方安装包并做完整性/签名校验；镜像源不接收任何凭证。
- geo 检测只获取国家码，不持久化 IP 或详细网络信息。
- Webview 只允许加载本机 `127.0.0.1` 上的 dsh 地址，不会被外部页面劫持。

## 能和官方的 `dsh` CLI 或其他 dsh GUI 共存吗？

可以。桌面壳默认把 dsh 数据放在 `~/.dsh`（`engineHome`）。只要把不同工具的 `engineHome` / `DSH_HOME` 指向同一目录，就能共享会话与插件。

- 若你已用官方 `npx @deepseek-ai/dsh web` 起了一个实例在 3080，桌面壳启动时会**直接连接**它，不会冲突。

## 日志在哪里？

- 桌面壳日志：`~/.dsh-tauri-gui/logs/`（托盘 → 打开日志目录）。
- dsh 自身数据/会话：`~/.dsh/`。

## 如何彻底清理 / 重装？

1. 托盘「退出」应用；
2. 删除 `~/.dsh-tauri-gui/`（配置与日志）；
3. 如需同时清空 dsh 数据，再删除 `~/.dsh/`；
4. 重新安装应用（或 `pnpm install && pnpm tauri dev` 本地开发）。

> 删除 `~/.dsh` 会丢失会话与已安装的插件，请先确认。

## 这跟 DeepSeek 官方是什么关系？

这是**社区维护的开源项目，并非 DeepSeek 官方产品**。我们不对上游 `@deepseek-ai/dsh` 做任何修改，仅通过官方 `dsh web` 命令与之组合。详见 [README 的免责声明](../README.md#免责声明)。
