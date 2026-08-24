# 发布与更新通道

本文说明桌面壳的 Release 流程与稳定更新通道（`update`）维护方式。**发布不打包任何运行时**：安装包只含桌面壳本身，Node.js / dsh 由用户本机提供。

## 发布流程

发布由 `.github/workflows/release.yml` 驱动，关键约束：

1. **触发**：推送 `v*.*.*` 标签，或 `workflow_dispatch`（指定 tag 与 `source_ref`）。
2. **版本一致性**：`scripts/release-version.mjs check-files` 校验 `package.json` / `Cargo.toml` / `tauri.conf.json` 中的版本一致。
3. **平台矩阵**：Windows、macOS（Intel + Apple Silicon）、Linux 各构建同一 tag commit，使用固定 Rust 工具链，产出 `bundle-manifest.json`。
4. **无 runtime 断言**：构建前检查 `src-tauri/resources/` 与 `tauri.conf.json` 不声明任何 runtime 资源；构建后检查 bundle 产物中不存在 `runtime.tar.gz`、`runtime.json`、`runtime/` 目录或 bundled Node 文件。
5. **签名**：Tauri updater 签名私钥来自仓库密钥 `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，公钥同时写在 `tauri.conf.json` 与 `src-tauri/src/update/netprobe.rs`。
6. **更新通道**：
   - 稳定版写入 `update` 发布下的 `latest.json`；
   - 预发布版上传 `latest-prerelease.json`，**不污染**稳定更新通道；
   - `reconcile-latest` 与 `publish-updater-channel` 两个 job 全局串行，确保 `GitHub Latest` 与稳定 `update` 通道指向最高稳定版本。

> 需要为某个发布版本生成对应的 `RELEASE_NOTES_<tag>.md` 时，可在仓库根放置该文件，`release.yml` 会自动作为发布说明。

## 手动维护稳定通道

`.github/workflows/update-latest.yml` 可在任意时刻重算稳定 `update` 发布：

- `reconcile`（默认）：按 SemVer 选取最高稳定版本；
- `from-tag`：指定 `stable_tag`（如 `v1.2.3`）并校验后写入。

`sync-updater` 会创建/复用固定的 `update` 发布（`prerelease=true`、`make_latest=false`），上传 `latest.json` 并做字节级回读校验，确保稳定通道始终指向正确的最高稳定版本。

## 关键脚本

| 脚本 | 作用 |
| --- | --- |
| `scripts/release-version.mjs` | 解析/校验版本、检查版本文件一致性 |
| `scripts/reconcile-release-state.mjs` | 创建草稿、上传资产、发布、sync-updater、set-latest |
| `scripts/update-latest.mjs` | 生成 `latest.json` |
| `scripts/rename-bundles.mjs` | 重命名安装包为发布命名约定 |
| `scripts/version-set.mjs` | 统一设置三处版本号 |

## 更新校验

桌面壳只在校验通过后安装更新：

1. 下载地址必须属于官方 `github.com/KevinT-hub/dsh-tauri-gui/releases/download`；
2. SHA-256 与 `latest.json` 中发布值一致；
3. minisign 签名由公钥验证通过。

镜像（`ghfast.top` 等）仅作为无凭证下载代理，不会绕过校验。
