# 安装说明

桌面壳**不内置运行时**，需要你本机已安装以下外部依赖。首次启动的检测页会逐项检查并给出安装帮助。

## 依赖一览

| 依赖 | 版本要求 | 作用 |
| --- | --- | --- |
| Node.js | `^22.19.0` 或 `>=24` | dsh 引擎的 JavaScript 运行时 |
| npm 或 pnpm | 任意可用版本 | 安装与管理 dsh 包（至少一个） |
| dsh | 官方 `@deepseek-ai/dsh` | 智能体框架 CLI |

## Windows

1. **Node.js**：前往 [nodejs.org](https://nodejs.org/en/download) 下载 LTS 22.x（或 24.x）安装包，按向导安装。国内用户可选用 [npmmirror 镜像](https://npmmirror.com/mirrors/node)。
   - 安装完成后打开新的命令提示符验证：`node --version` 应输出 `v22.19.0` 或更高。
2. **dsh**（随 Node 自带 npm）：
   ```bat
   npm install -g @deepseek-ai/dsh
   ```
   国内可加 `--registry=https://registry.npmmirror.com`。
3. 验证：`dsh --version`。

## macOS

1. **Node.js**：下载 [nodejs.org](https://nodejs.org/en/download) 的 macOS 安装包，或使用 Homebrew：
   ```sh
   brew install node@22
   ```
2. **dsh**：
   ```sh
   npm install -g @deepseek-ai/dsh
   ```
3. 验证：`node --version` 与 `dsh --version`。

## Linux

1. **Node.js**：使用发行版包管理器或 [NodeSource](https://nodejs.org/en/download/package-manager)：
   ```sh
   # Debian/Ubuntu
   curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
   sudo apt-get install -y nodejs
   ```
2. **dsh**：
   ```sh
   npm install -g @deepseek-ai/dsh
   ```
3. 验证：`node --version` 与 `dsh --version`。

## pnpm（可选）

npm 已满足门禁（「npm 或 pnpm 至少一个」）。如需 pnpm：

```sh
corepack enable          # Node 22 自带 corepack
# 或官方脚本：curl -fsSL https://get.pnpm.io/install.sh | sh -
```

## 常见版本要求

- **Node**：官方 dsh 要求 `^22.19.0 || >=24`。版本不满足时检测页会标记「版本不满足」，请升级或切换（nvm / fnm / nvs）。
- **dsh**：任何可用版本均可通过门禁；升级到最新版用 `npm install -g @deepseek-ai/dsh@latest`。

## 验证门禁

```sh
node --version        # v22.19.0+ 或 v24+
npm --version         # 或 pnpm --version
dsh --version
```

三条命令均成功输出后，启动桌面壳即可直接进入 Web UI。
