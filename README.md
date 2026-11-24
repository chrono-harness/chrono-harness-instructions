# chrono-harness-instructions

按宿主规则生成项目指南。

使用 `cargo build --locked --manifest-path crates/instructions/Cargo.toml` 构建。运行 `mise run harness-setup` 安装本仓固定的工具，再运行 `mise exec -- ./check.sh` 检查格式、构建、测试和证明。检查读取 Verus 返回的编译依赖，将生产判法和独立预期的完整字节逐项与本次输入核对。

源码来自 [固定版本的原实现](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/instructions)。

`init --host-root DIR` 在宿主内登记输入并创建指南；`generate --host-root DIR` 根据已登记的输入更新指南。运行期间独占该宿主，失败时仅回滚仍与本次发布匹配的对象。成功或完整回滚后自动删除临时目录。回滚遇到独立改动时，错误中给出精确路径和仍保存原件的工作树临时目录。中断后从 Git 核对工作树并重新执行。长期保留的源文件和生成指南归宿主 Git 管理。

宿主可通过 `.chrono-harness/logging.json` 选择有限大小和份数的日志。指令工具只写所选日志目录中的 `chrono-instructions/` 子目录。临时目录随任务工作树回收。

从[本工具的发行页](https://github.com/chrono-harness/chrono-harness-instructions/releases)选定标签，下载对应平台的二进制、`methodology-attribution.md`、`trureturing-Apache-2.0.txt` 和 `SHA256SUMS`，验算 SHA-256 后安装为工作树外的 `chrono-instructions`。升级和回退按所需标签独立替换该二进制。

本仓贡献者可以单独安装 [chrono-pr v0.1.0-alpha.2](https://github.com/chrono-harness/chrono-harness-pr/releases/tag/v0.1.0-alpha.2)，从仓库根目录使用 `pr-open`、`pr-status` 和 `pr-reminders` 跟进变更。仓库、分支和检查名登记在 `.chrono-harness/pr.json`。

## 继承上游规则

在仓库根目录运行 `chrono-instructions rules --host-root .`。命令每次读取上游最近一次通过检查后发布的数据，将本仓接纳的规则和 ID 写到标准输出。运行 `chrono-instructions sync-rules --host-root .` 会把相同的选择写入根 `CLAUDE.md` 的生成块，并将 `AGENTS.md` 链接到它。本仓追加说明保存在块外。克隆仓库后可以直接阅读已提交的全部规则。重复同步相同规则时文件保持原样，获取失败时保留已有规则。读取规则需要 HTTPS 和 curl。上游暂不可用时命令返回错误；本地说明仍在仓库内。

缺省时接纳全部上游基础规则，包括今后新增的 ID。需要定制时，在本仓 `.chrono-harness/rules.json` 选择一种写法。

| 选择 | 文件内容 |
| --- | --- |
| 接纳全部 | `{"mode":"all"}` |
| 接纳全部，屏蔽指定规则 | `{"mode":"all","exclude":["job.wait"]}` |
| 只接纳指定规则 | `{"mode":"only","include":["job.wait","artifact.cleanup"]}` |
| 关闭上游规则 | `{"mode":"off"}` |

选择按完整 ID 匹配。上游修改同一 ID 的文字会随下次读取生效。已移除的 ID 自动失效。关闭和空白名单均可离线使用。可在[公共规则数据](https://raw.githubusercontent.com/chrono-harness/chrono-harness-instructions/published-rules/rules.json)查看当前可选 ID。

上游维护者修改产品的 catalog 或默认 manifest 后，dev 的检查通过即自动发布规则。下游无需更新二进制、模板或提交。维护自己完整登记的项目也可继续使用 init 和 generate。

事务生成的正式检查同时收取产品和专属测试的 Verus 原件，并执行中途发布失败的回滚用例。该用例通过独立测试构建观察当前线程实际进入 `publication_decision::repair`，同时验证宿主文件恢复。移除这次生产调用会让用例报告 `E_VERUS_COVERAGE`。本项覆盖被执行的回滚场景。优化后的发行二进制及其他函数的实际调用覆盖仍须分别验收。


## GitHub 工作事件

每个会话用固定的公开 session，每项工作用同一个 task，每次状态变化得到新的 event。用 GitHub API 读取原始 body。下列命令输出转换失败时退出 1，成功时退出 0。

~~~sh
chrono-instructions event-format < event.json
gh api repos/OWNER/REPO/issues/NUMBER | jq -j .body | chrono-instructions event-read
chrono-instructions event-regex
~~~

event-format 接受 agent、session、parent、task、event、action、state、target 八个字符串组成的 JSON 对象。原始 API body 可能已自带末尾换行；对机器输入请保留原始字节，例如用 jq -j '.body' 提取。[完整规则](assets/instructions/catalog.json) · [可复制正则](assets/instructions/github-event-v2.regex)

正则读取单行，按顺序捕获八个字段。开启 ASCII，大小写敏感，关闭多行，以 fullmatch 或等价的整串比较应用。正文先将 CRLF 转为 LF，拒绝孤立 CR 和两个及以上末尾换行；最终一条必须是顶层普通段落，前方若有正文须有空行。代码、引述、HTML 注释与嵌套段落中的示例不作签名。读取器同样检查 UUID 独立性。每条记录保留 GitHub 作者、原始对象 URL 和创建时间。精确复制一条合法事件会通过语法检查。
