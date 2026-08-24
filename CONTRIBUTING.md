# 贡献与开发

感谢你关注 dsh-tauri-gui！本文件介绍如何从源码构建、本地开发、参与发布，以及代码约定。

## 技术栈

- **桌面壳**：Tauri v2（Rust）
- **前端**：React 19 + TypeScript + Vite 7 + Tailwind CSS v4 + Material Web（`@material/web`）
- **外部依赖**：Node.js（`^22.19.0 || >=24`）+ 官方 `@deepseek-ai/dsh`（**不打包运行时**）
- **CI 工具链**：pnpm 11.7.0、Rust 1.93.0（由 `src-tauri/rust-toolchain.toml` 固定）

## 本地开发环境

### 前置条件

- **Node.js** ≥ 22.19（包声明 `engines.node >= 22`）
- **pnpm** 11.7.0（`corepack enable` 后 `corepack prepare pnpm@11.7.0 --activate`）
- **Rust** 1.93.0（`rust-toolchain.toml` 自动接管）
- **官方 dsh**：`npm install -g @deepseek-ai/dsh`（开发与运行都依赖本机 dsh）
- 平台依赖：
  - **Linux**：`libwebkit2gtk-4.1-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev`、`libfuse2`、`patchelf`、`libxdo-dev`、`libssl-dev`
  - **Windows / macOS**：通常无需额外系统库

### 本地运行

```sh
# 1. 安装前端依赖
pnpm install

# 2. 以开发模式启动（Tauri + Vite 热更新）
pnpm tauri dev
```

桌面壳会检测本机 PATH 中的 node/npm/pnpm/dsh；缺失时检测页给出安装帮助。没有 `runtime:prepare` 之类的步骤——运行时不在仓库内。

### 常用脚本

| 命令 | 说明 |
| --- | --- |
| `pnpm dev` | 仅启动前端 Vite（不含 Rust） |
| `pnpm build` | `tsc` 类型检查 + Vite 生产构建 |
| `pnpm tauri dev` | 开发模式（前端 + Rust 壳） |
| `pnpm test:scripts` | 运行 `tests/*.test.mjs` |
| `pnpm version:check` / `pnpm version:set` | 版本一致性检查 / 设置 |

## 目录结构

```text
src-tauri/src/
├─ app/          应用层：状态、配置、生命周期、状态机、事件
├─ commands/     Tauri command 适配层（校验/转发，不含业务实现）
├─ core/         基础设施：进程、HTTP、路径、日志、脱敏、版本、平台
├─ detection/    环境检测：Node/npm/pnpm/dsh 探测、聚合、安装帮助、源策略
├─ engine/       引擎：命令构造、进程、生命周期、Web、协议、环境、工作目录
├─ geo/          国家码检测：endpoints、client、consensus、cache
├─ ui/           托盘、窗口、主题、菜单
└─ update/       应用更新：checker、downloader、netprobe
```

依赖方向：`commands -> app/detection/engine/geo/update -> core`。`core` 不得依赖 `commands`/`ui`/`app`；`geo` 不得依赖 installer；`engine` 不得依赖具体 React 页面；`commands` 不得复制检测/安装/更新实现。

## 发布流程

发布由 `.github/workflows/release.yml` 驱动。**安装包不打包运行时**：CI 与 Release 均不准备/缓存 runtime 资源，并带反向断言确认产物无 `runtime.tar.gz` / `runtime.json` / bundled Node。详见 [RELEASE.md](docs/RELEASE.md)。

## 代码约定

- **Rust**：提交前 `cargo fmt` 与 `cargo clippy --all-targets --all-features -- -D warnings` 应无警告；`src-tauri` 下不要引入不必要的第三方依赖。
- **TypeScript / React**：类型完整；`pnpm build`（含 `tsc`）须通过；UI 文案使用中文。
- **配置写入**：`app/config.rs` 的 `save()` 采用「临时文件 + 备份」的原子写入，并在主配置缺失时从 `.bak` 恢复——新增持久化字段时沿用该模式；**旧配置中的 `runtimeMode` 等遗留字段必须保持 serde 静默忽略**。
- **日志脱敏**：任何写入日志 / 转发给前端的字符串都必须先经过 `core/redact.rs` 的 `redact()`，避免泄露密钥、token 等。
- **外部进程**：所有探测/安装/引擎命令必须通过 `core/process.rs` 的参数数组调用并隐藏控制台，禁止拼接任意 shell 字符串。
- **无黑屏**：主窗口初始 `visible: false`，仅在 shell 页面已绘制（首次引导）或 Web UI 已就绪（`__DSH_BOOT__` 标记 + 白名单来源）后才显示——修改启动 / 导航逻辑时务必保持该不变量。
- **安全边界**：Webview 仅允许加载 `is_allowed_web_url()` 白名单内的 `http://127.0.0.1:<port>` 来源，以及固定的 shell 来源；禁止放宽 CSP 或允许任意导航。
- **安装确认**：`commands/setup.rs` 的安装动作只接受用户明确触发，禁止静默修改用户环境。
- **插件兼容**：不得改变 `DSH_HOME`、插件目录、会话目录和官方 dsh 启动参数语义（见 [PLUGIN_COMPATIBILITY.md](docs/PLUGIN_COMPATIBILITY.md)）。
- **禁止提交 runtime 资源**：仓库内不得出现 `runtime.tar.gz`、`runtime.json`、`runtime/` 目录或 bundled Node/pnpm/dsh 文件；`src-tauri/resources/` 只放必要的静态资源。

## 测试

- 前端 / 脚本：`pnpm test:scripts`（Node 内置测试，覆盖 `tests/*.test.mjs`）
- Rust 单元：`cargo test`（环境检测门禁、版本解析、Windows 命令扩展名、geo 规范化与共识、源选择、配置迁移等）
- 契约测试：`tests/detection-contract.test.mjs`（前后端 dependency payload 契约）、`tests/release-no-runtime.test.mjs`（发行物禁止携带 runtime 资源）
- 测试不得依赖真实用户 PATH、真实 dsh home 或真实 geo 服务，必须通过 fixture、临时目录与可注入的 command runner。

## 提交与反馈

- Issue / PR 请使用仓库的 GitHub 功能；
- 安全相关问题请私下联系维护者，勿公开披露。

## License

[MIT](LICENSE) © 2026 KevinT-hub
