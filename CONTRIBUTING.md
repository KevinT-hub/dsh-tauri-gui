# 贡献与开发

感谢你关注 DeepSeek Harness Tauri Desktop！本文件介绍如何从源码构建、本地开发、参与发布，以及代码约定。

## 技术栈

- **桌面壳**：Tauri v2（Rust）
- **前端**：React 19 + TypeScript + Vite 7 + Tailwind CSS v4 + Material Web（`@material/web`）
- **运行时**：内置 Node.js + pnpm + `@deepseek-ai/dsh`（见 `scripts/runtime-versions.json`）
- **CI 工具链**：pnpm 11.7.0、Rust 1.93.0（由 `src-tauri/rust-toolchain.toml` 固定）

## 本地开发环境

### 前置条件

- **Node.js** ≥ 22（包声明 `engines.node >= 22`；推荐与 `runtime-versions.json` 中的 22.19.0 一致）
- **pnpm** 11.7.0（`corepack enable` 后 `corepack prepare pnpm@11.7.0 --activate`）
- **Rust** 1.93.0（进入仓库后 `rustup toolchain install 1.93.0` 或让 `rust-toolchain.toml` 自动接管）
- 平台依赖：
  - **Linux**：`libwebkit2gtk-4.1-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev`、`libfuse2`、`patchelf`、`libxdo-dev`、`libssl-dev`
  - **Windows / macOS**：通常无需额外系统库

### 本地运行

```sh
# 1. 安装前端依赖
pnpm install

# 2. 准备自包含运行时（Node + pnpm + dsh）到 ~/.dsh-tauri-gui/runtime
pnpm runtime:prepare --dev

# 3. 以开发模式启动（Tauri + Vite 热更新）
pnpm tauri dev
```

`prepare-runtime.mjs` 会从 `scripts/runtime-versions.json` 读取固定版本，下载并校验 Node 归档（SHASUMS256），再安装 `@deepseek-ai/dsh` 与 pnpm（带 npmmirror → npmjs 镜像回退），最后做体积裁剪。`--dev` 把运行时放到用户目录，使 `tauri dev` 与打包构建行为一致。

### 常用脚本

| 命令 | 说明 |
| --- | --- |
| `pnpm dev` | 仅启动前端 Vite（不含 Rust） |
| `pnpm build` | `tsc` 类型检查 + Vite 生产构建 |
| `pnpm tauri dev` | 开发模式（前端 + Rust 壳） |
| `pnpm runtime:prepare` | 准备本地运行时（`--dev` / `--package` / `--prune`） |
| `pnpm runtime:package` | 打包运行时为 `src-tauri/resources/runtime.tar.gz` + `runtime.json` |
| `pnpm runtime:smoke` | 冒烟测试：校验解压后的运行时可用 |
| `pnpm test:scripts` | 运行 `scripts/*.test.mjs` |
| `pnpm version:check` / `pnpm version:set` | 版本一致性检查 / 设置 |

## 运行时打包

发布安装包前，必须先用目标平台原生的 Node 打包运行时（node-pty 等原生模块按平台分发）：

```sh
pnpm run runtime:package
```

该命令会生成 `src-tauri/resources/runtime.tar.gz` 与 `runtime.json`。CI 的 `release.yml` 在构建安装包前会执行此步骤，并**断言资源真实**（拒绝 `testMode` 占位文件与过小归档），防止仅用于编译的占位资源泄漏到发布包。

## 发布流程

发布由 `.github/workflows/release.yml` 驱动，关键约束：

1. **触发**：推送 `v*.*.*` 标签，或 `workflow_dispatch`（指定 tag 与 `source_ref`）。
2. **版本一致性**：`scripts/release-version.mjs check-files` 校验 `package.json` / `Cargo.toml` / `tauri.conf.json` 中的版本一致。
3. **平台矩阵**：Windows、macOS（Intel + Apple Silicon）、Linux 各构建同一 tag commit，使用固定的 Rust 工具链，产出 `bundle-manifest.json`。
4. **更新通道**：
   - 稳定版写入 `update` 发布下的 `latest.json`；
   - 预发布版上传 `latest-prerelease.json`，**不污染**稳定更新通道；
   - `reconcile-latest` 与 `publish-updater-channel` 两个 job 全局串行，确保 `GitHub Latest` 与稳定 `update` 通道指向最高稳定版本。
5. **签名**：Tauri updater 签名私钥来自仓库密钥 `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，公钥同时写在 `tauri.conf.json` 与 `src-tauri/src/update/netprobe.rs`。

> 需要为某个发布版本生成对应的 `RELEASE_NOTES_<tag>.md` 时，可在仓库根放置该文件，`release.yml` 会自动作为发布说明。

CI 安全加固的细节（最小权限、产物校验、密钥边界等）见 [`docs/GITHUB_ACTIONS_HARDENING_PLAN.md`](docs/GITHUB_ACTIONS_HARDENING_PLAN.md)。

## 代码约定

- **Rust**：提交前 `cargo fmt` 与 `cargo clippy` 应无警告；`src-tauri` 下不要引入不必要的第三方依赖。
- **TypeScript / React**：类型完整；`pnpm build`（含 `tsc`）须通过；UI 文案使用中文。
- **配置写入**：`app/config.rs` 的 `save()` 采用「临时文件 + 备份」的原子写入，并在主配置缺失时从 `.bak` 恢复——新增持久化字段时沿用该模式。
- **日志脱敏**：任何写入日志 / 转发给前端的字符串都必须先经过 `core/redact.rs` 的 `redact()`，避免泄露密钥、token 等。
- **无黑屏**：主窗口初始 `visible: false`，仅在 shell 页面已绘制（首次引导）或 Web UI 已就绪（`__DSH_BOOT__` 标记 + 白名单来源）后才显示——修改启动 / 导航逻辑时务必保持该不变量。
- **安全边界**：Webview 仅允许加载 `is_allowed_web_url()` 白名单内的 `http://127.0.0.1:<port>` 来源，以及固定的 shell 来源；禁止放宽 CSP 或允许任意导航。
- **运行时切换**：从托盘切换运行时时，`ui/tray.rs` 的 `switch_runtime` 必须先做预检（目标运行时可用），再持久化、停旧引擎、整体重启——新增运行时相关逻辑时保持一致。

## 测试

- 前端 / 脚本：`pnpm test:scripts`（Node 内置测试，覆盖 `scripts/*.test.mjs`）
- Rust 单元：`cargo test`（版本解析、Node 版本支持窗口、更新比较等）
- 运行时冒烟：`pnpm runtime:smoke`

## 提交与反馈

- Issue / PR 请使用仓库的 GitHub 功能；
- 安全相关问题请私下联系维护者，勿公开披露。

## License

[MIT](LICENSE) © 2026 KevinT-hub
