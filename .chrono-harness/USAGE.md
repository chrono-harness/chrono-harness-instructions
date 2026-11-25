# chrono-harness-instructions 使用说明

组件的用途、开发仓库和示例见 [Chrono Harness 用户与开发者入口](https://github.com/chrono-harness/chrono-harness-main)。

本仓维护基础 agent 指令、按规则 ID 选择的协议和 chrono-instructions 生成工具。

开发分支为 `dev`。日常 PR 合入开发分支，正式版从 `main` 发布。

首次检出运行 `mise trust` 和 `mise run harness-setup`。本仓默认安装策略为 `pinned`。固定配方读取 [.config/mise/conf.d/chrono-harness.toml](../.config/mise/conf.d/chrono-harness.toml)。模板答案 `tool_versions` 可以为每个工具选择 stable、preview 或精确标签。临时运行 `CHRONO_TOOL_CHANNEL=preview mise run harness-setup` 会为已选组织工具安装各自最新测试版，也可以把 `preview` 换成 `stable` 或精确的 `v` 标签。再次运行普通安装即可恢复本仓选择。本仓的专用 CI 通过 .chrono-harness/ci/install.sh 安装登记的检查工具。各分支、标签和 detached HEAD 使用相同规则。

根指南的刷新命令为 `mise run harness-setup`。完整规则和本仓开发步骤见根 [CLAUDE.md](../CLAUDE.md)。

用 `mise exec -- chrono-pr pr-reminders` 查看本地提醒，再运行 `mise exec -- chrono-pr pr-status --pr NUMBER --head-sha SHA` 查询 GitHub。

## 本仓验证

产品位于 [`crates/instructions`](../crates/instructions)。默认指令登记在 [`assets/instructions`](../assets/instructions)。在仓库根目录运行 `mise exec -- ./check.sh` 执行本仓的检查。
