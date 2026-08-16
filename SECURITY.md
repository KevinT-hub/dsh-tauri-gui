# 安全策略

## 支持的版本

安全修复仅针对最新 Release 版本。请尽量保持软件处于最新版本。

## 报告漏洞

请通过 GitHub 私有安全通告（Security Advisories）报告漏洞：

仓库页面 → **Security** → **Report a vulnerability**

请勿在公开 issue、讨论区或 PR 中透露漏洞细节。

## 关注范围

- 更新通道伪造（`latest.json` / 签名 / SHA-256 校验绕过）
- 内置运行时解压与路径穿越
- 日志与诊断信息中的凭据泄露
- CSP、导航白名单与 WebView 安全配置
- 供应链依赖投毒或校验绕过

## 第三方组件

核心引擎 `@deepseek-ai/dsh` 由 DeepSeek 官方维护。涉及引擎本身的安全问题请同时上报到 <https://github.com/deepseek-ai/deepseek-harness>。
