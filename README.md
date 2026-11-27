# chrono-harness-instructions

按宿主规则生成项目指南。

使用 `cargo build --locked --manifest-path crates/instructions/Cargo.toml` 构建。运行 `mise run harness-setup` 安装本仓固定的工具，再运行 `mise exec -- ./check.sh` 检查格式、构建、测试和证明。检查读取 Verus 返回的编译依赖，将生产判法和独立预期的完整字节逐项与本次输入核对。

源码来自 [固定版本的原实现](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/instructions)。

`init --host-root DIR` 在宿主内登记输入并创建指南；`generate --host-root DIR` 根据已登记的输入更新指南。运行期间独占该宿主，失败时仅回滚仍与本次发布匹配的对象。成功或完整回滚后自动删除临时目录。回滚遇到独立改动时，错误中给出精确路径和仍保存原件的工作树临时目录。中断后从 Git 核对工作树并重新执行。长期保留的源文件和生成指南归宿主 Git 管理。

如需文件日志，安装 `chrono-diagnostics` 并在宿主的 `.chrono-harness/logging.json` 选择目录、大小、份数和可选保留时间。指令工具通过已发布的 `cli-events-v1` 协议交给该工具写入。查看所选目录下的 `runtime.jsonl`，按 `producer=chrono-instructions` 选取记录。旧版写在该目录的 `chrono-instructions/` 子目录，升级前仍需留存的日志可转入宿主的持久存储。未配置文件日志的安装可以单独运行。

从[本工具的发行页](https://github.com/chrono-harness/chrono-harness-instructions/releases)选定标签，下载对应平台的二进制、`methodology-attribution.md`、`trureturing-Apache-2.0.txt` 和 `SHA256SUMS`，验算 SHA-256 后安装为工作树外的 `chrono-instructions`。升级和回退按所需标签独立替换该二进制。

Issue 分类、分支命名、PR 的关联与关闭及宿主选择方式见[贡献约定](docs/contributing.md)。

本仓贡献者可以单独安装 [chrono-pr v0.1.0-alpha.2](https://github.com/chrono-harness/chrono-harness-pr/releases/tag/v0.1.0-alpha.2)，从仓库根目录使用 `pr-open`、`pr-status` 和 `pr-reminders` 跟进变更。仓库、分支和检查名登记在 `.chrono-harness/pr.json`。

## 本仓检查

从本仓根目录运行 `mise exec -- ./check.sh`。它安装本仓固定的发行工具，按 Git 基线和候选的 FILEMAP 选择操作。事务修复的变更同时运行产品证明、独立测试证明及原生回滚用例。GitHub 的 `Instructions / recovery` 单元上传真实报告和编译回执，`check` 汇总作业使用 `chrono-harness check --collect` 读回这些原件。CLI 会报告原操作失败原因。

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

默认规则 `communication.plain-language` 用一条可核实的测试结果说明怎样写给新读者。`recovery.bounded` 说明读写超时、编译失败、断言失败、磁盘不足和进程被终止时怎样查原错，何时可以重试。两条可以分别开关。外部写入超时的例子给出安全重放的条件和盲目重发的反例。[异常处置模型和运行方法](docs/recovery-model.md)

上游维护者修改产品的 catalog 或默认 manifest 后，dev 的检查通过即自动发布规则。下游无需更新二进制、模板或提交。维护自己完整登记的项目也可继续使用 init 和 generate。

事务生成的正式检查同时收取产品和专属测试的 Verus 原件，并执行中途发布失败的回滚用例。该用例通过独立测试构建观察当前线程实际进入 `publication_decision::repair`，同时验证宿主文件恢复。移除这次生产调用会让用例报告 `E_VERUS_COVERAGE`。本项覆盖被执行的回滚场景。优化后的发行二进制及其他函数的实际调用覆盖仍须分别验收。


## GitHub 工作事件

每个会话使用固定的公开专用 session UUIDv4。同一 task 可包含多个会话和模型，同一会话可以在事件之间切换模型。每个新动作或状态变化使用新的 event。用 GitHub API 读取原始 body。下列命令输出转换失败时退出 1，成功时退出 0。

~~~sh
chrono-instructions event-format < event.json
gh api repos/OWNER/REPO/issues/NUMBER | jq -j .body | chrono-instructions event-read
chrono-instructions event-regex
~~~

event-format 接受 agent、session、model、parent、task、event、action、state、target 九个字符串。model 写运行时提供的当前模型 ID 或别名，符合 `[A-Za-z0-9][A-Za-z0-9._/@:+-]*`，如 `gpt-5.4` 或 `claude-opus-4-6`。运行时未提供型号时写 `unknown`。省略 model 也生成 `unknown`。它生成以下三行，页面会显示中间的摘要。用户可以先写任意说明，空一行后附上完整记录。首尾 event 相同，session 与 task 可跨事件关联。

~~~markdown
<!-- chrono-agent:v4 event=e1000000-0000-4000-b000-000000000000 parent=- task=a1000000-0000-4000-8000-000000000000 -->
**Agent** `codex/root` · **session** `058ca5a5-1923-4ec4-8cf6-74a1694a510d` · **model** `unknown` · **action** `review` · **state** `succeeded` · **target** `org/repo#12`
<!-- /chrono-agent:v4 event=e1000000-0000-4000-b000-000000000000 -->
~~~

用 `event-read` 读取 GitHub API 的原始 body，正文可用 LF 或 CRLF，末尾可有一个换行。读取器识别 Markdown 顶层结构，核对记录唯一、完整、位于文末且 UUID 各自独立。引述、围栏和嵌套 HTML 示例中的内容按正文处理。已发布的 v2、v3 记录可读取为九字段 JSON，其中 model 为 unknown。保存事件时连同 GitHub 作者、原始对象 URL 和创建时间一起保存。

`event-regex` 提供Python、PCRE2 和 JavaScript 均可使用的跨行表达式，匹配完整的三行内容，捕获组 1 到 9 依次为 event、parent、task、agent、session、model、action、state、target。`\1` 要求末行 UUID 与首行逐字一致。各环境关闭多行、忽略大小写和 dotall 模式。比如在 Python 中可使用 `re.fullmatch(pattern.rstrip("\n"), frame, flags=re.ASCII).groups()`。应从 Markdown 中识别出顶层候选后再匹配。正文原样复制仍能通过语法校验。[可复制正则](assets/instructions/github-event-v4.regex) · [完整规则](assets/instructions/catalog.json)

工作事件读取器按原始帧从第一个字节起的完整 `<!-- chrono-agent:v3 ` 前缀选择旧语法，其余帧由 v4 语法校验。该分派的 Verus 证明与实际读取器共享实现，独立预期覆盖任意后缀、任意截断和任意前缀字节差异。捕获字节的身份关系也有 Verus 证明：v3、v4 的结束标记须重复开头的 event，session、parent、task 和 event 两两不同。历史 v2 传入四个原始 ID。登记检查同时收取生产根、独立测试根和实际读取器的反例结果。GFM 结构由 pulldown-cmark 提供，字段词法和捕获由 regex 提供，其余边界见上述协议。

上游规则的选择由正式检查收取生产及独立预期证明。规则 ID 和上游重复行按完整字节比较，选中条目保持上游顺序。关闭或空单选可离线使用。同步前检查输出是否包含受宿主管理的界标，失败时保留原指南。JSON 解码、Unicode 去首尾空白、树集合按键查询、网络和文件事务按各自边界工作。

## 本地检查产物

原生编译在 `target/native-checks/` 下按次分配目录。正常完成会立即回收。强制中断后，已配置的同一小时维护任务负责清理超过 24 小时且没有进程持锁的目录。失败原件需要长期保留时，请在窗口内转存。

本仓的单元和汇总检查在各自的业务目录发布原件。判官读回本次 Verus 编译库及依赖文件后，成功的操作会释放这两份文件。检查报告的保留窗口为 24 小时，需要长期保存的报告及其引用原件须在窗口内按相对路径转存，并在检出外读回。

运行 `mise run harness-maintain-checks` 可执行一轮本仓的检查产物维护。已到期的成功报告在读取和交付义务结束后释放，后续维护轮次回收失去引用的原件。没有业务事件或显式维护时，到期的文件会继续占用磁盘。
