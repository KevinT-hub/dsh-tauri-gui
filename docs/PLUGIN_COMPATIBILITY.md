# 插件兼容性

本文说明桌面壳与官方 `@deepseek-ai/dsh` 插件/会话/模型数据的边界，以及维护者应遵守的兼容性约束。

## 原则

桌面壳**不复制、不替换、不迁移** dsh 的插件机制。插件、会话、模型配置与工具全部属于官方 dsh，由官方 `dsh web` 进程在 `$DSH_HOME` 下管理。壳层只做两件事：

1. 以官方命令启动 dsh：`dsh web --no-open --port <port>`；
2. 把 `DSH_HOME` 与工作目录原样传给它。

## 兼容性契约

以下语义**不得改变**（修改即破坏既有用户数据）：

| 项目 | 约定 | 位置 |
| --- | --- | --- |
| `DSH_HOME` | 默认 `~/.dsh`，与官方 CLI 共享 | 环境变量 / `engineHome` 配置 |
| 插件目录 | 官方 dsh 管理的插件目录，壳层不读写 | `$DSH_HOME` 下 |
| 会话目录 | 会话数据由官方 dsh 持久化，壳层不另存 | `$DSH_HOME` 下 |
| 模型配置 | `settings.yaml` / 官方配置文档 | `$DSH_HOME` 下 |
| 启动参数 | `dsh web --no-open --port <port>` | `engine/command.rs` |
| registry 环境 | `npm_config_registry` 传递给引擎子进程 | `engine/environment.rs` |
| 主题联动 | 壳层可写 `ui-theme.preference`，保留其余文档 | `$DSH_HOME/settings.yaml` |

## 连接或拉起（connect-or-spawn）

若端口上已有**另一个官方 dsh 实例**在运行（无论是桌面壳启动的还是 CLI 手动启动的），桌面壳会直接连接它而不重复拉起——这样多工具共享同一份配置、会话与插件，不会出现数据分裂。识别标记为 `__DSH_BOOT__`。

## 约束检查

- 引擎模块（`engine/`）只接收检测阶段验证过的外部 `CommandSpec`，不含 bundled runtime / 归档 / 解压逻辑；
- 壳层任何代码都**不得**写入 `$DSH_HOME` 下的插件目录、会话目录或模型配置（主题 `settings.yaml` 的 `ui-theme.preference` 是唯一例外）；
- 检测（`detection/`）不触碰插件内容，只探测 PATH 中的工具版本；
- 新增功能时，若涉及 `DSH_HOME`、插件目录、会话目录或官方启动参数语义，必须回归验证与官方 CLI 共存场景。

## 回归验证

改动涉及上述契约时，至少验证：

1. 桌面壳启动的 Web UI 与 `npx @deepseek-ai/dsh web` 手动启动的实例可互相连接；
2. 在桌面壳创建的会话，官方 CLI / Web UI 可见且可用；
3. 已有插件在升级桌面壳后保持可用。
