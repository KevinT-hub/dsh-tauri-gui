# 参与贡献

欢迎向 DeepSeek Harness Tauri Desktop 提交 issue、PR 或改进建议。

## 开发环境

- Node.js 22.19+（或 24+）
- Rust stable
- pnpm 11.7.0（项目通过 `packageManager` 固定）

```sh
pnpm install
pnpm run runtime:prepare   # 准备本地开发运行时
pnpm tauri dev             # 启动桌面壳
```

## 提交前请自测

```sh
pnpm install --frozen-lockfile
pnpm run build             # 前端构建
cd src-tauri
cargo check
cargo test --lib           # Rust 单元测试
cd ..
pnpm run runtime:smoke -- --runtime <runtime 目录>  # 端到端 smoke
```

CI 会执行同样的检查；PR 必须保持所有检查通过。

## 提交规范

- 使用 Conventional Commits 风格：`feat:` / `fix:` / `chore:` / `docs:` / `ci:` / `build:` / `test:`。
- 保持细粒度提交，一个逻辑变更一个 commit，不要把所有改动塞进一个大 commit。
- 提交信息用英文，描述具体变更内容。

## 注意事项

- 不要提交 `docs/`、`node_modules/`、`dist/`、`src-tauri/resources/` 等生成物；它们已被 `.gitignore` 排除。
- 永远不要提交 Tauri 签名私钥（`TAURI_SIGNING_PRIVATE_KEY`）或本地密钥文件。
- 修改运行时裁剪规则后，必须跑 `runtime:smoke` 验证裁剪后的运行时仍可启动。
- README 中引用的本地文件链接需要保持有效。
