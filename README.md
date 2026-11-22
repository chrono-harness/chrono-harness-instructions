# chrono-harness-instructions

按宿主规则生成项目指南。

使用 `cargo build --locked --manifest-path crates/instructions/Cargo.toml` 构建。安装 Verus 0.2026.04.24.f8e1704，运行 `VERUS=/path/to/verus ./check.sh` 检查格式、构建、测试及证明。

源码来自 [固定版本的原实现](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/instructions)。

`init --host-root DIR` 在宿主内登记输入并创建指南；`generate --host-root DIR` 根据已登记的输入更新指南。运行期间独占该宿主，失败时仅回滚仍与本次发布匹配的对象。成功或完整回滚后自动删除临时目录。回滚遇到独立改动时，错误中给出精确路径和仍保存原件的工作树临时目录。中断后从 Git 核对工作树并重新执行。长期保留的源文件和生成指南归宿主 Git 管理。

宿主可通过 `.chrono-harness/logging.json` 选择有限大小和份数的日志。指令工具只写所选日志目录中的 `chrono-instructions/` 子目录。临时目录随任务工作树回收。

从[本工具的发行页](https://github.com/chrono-harness/chrono-harness-instructions/releases)选定标签，下载对应平台的二进制和 `SHA256SUMS`，验算 SHA-256 后安装为工作树外的 `chrono-instructions`。升级和回退按所需标签独立替换该二进制。

本仓贡献者可以单独安装 [chrono-pr v0.1.0-alpha.2](https://github.com/chrono-harness/chrono-harness-pr/releases/tag/v0.1.0-alpha.2)，从仓库根目录使用 `pr-open`、`pr-status` 和 `pr-reminders` 跟进变更。仓库、分支和检查名登记在 `.chrono-harness/pr.json`。
