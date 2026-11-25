# GitHub 贡献约定

先读目标仓库的 `AGENTS.md`，确认工作入口、检查和目标分支。选择 `collaboration.github-contribution` 的宿主采用本页的 Issue、工作分支和 PR 约定。宿主在自己的协作说明中登记实际分支、启用的标签、必需检查、合并方式、发布和跨仓协调入口。

## Issue

用一句话描述可观察的问题或想要的结果，例如「PR 进入冲突后立即返回」。创建前搜索同仓的开放和已关闭事项。需要多个 PR、下游采用或后续跟踪时开 Issue。一次 PR 即可验收的小改动可以直接提交。

普通 Issue 按主要诉求选择一个主标签。网页使用宿主的分类表单，CLI 用 `gh issue create --label <主标签>`，API 设置 `labels`。改变分类时只更换主标签，保留其他有效标签。

| 主标签 | 适用情形 | 正文应提供的信息 |
| --- | --- | --- |
| `bug` | 已有行为偏离预期 | 版本或 SHA、复现步骤、预期与实际结果、原始失败 |
| `enhancement` | 新能力、行为改进、工程任务或跨仓路线图 | 使用方、交付物、依赖和验收条件 |
| `documentation` | 指南、示例或参考文档 | 哪位读者在哪一步遇到困难，读完应能完成什么 |
| `question` | 用法或行为有待解释 | 目标、所用版本、已试方法、具体问题 |

采用额外标签的仓库可用 `accessibility` 标出无障碍影响，`help wanted` 表示可领取，`good first issue` 表示任务边界清楚且附有入门指引。标 `duplicate` 时链接原事项；标 `invalid` 或 `wontfix` 时说明判定或取舍。开放、关闭和关闭原因使用 GitHub 原生状态，排期使用 Milestone。需要说明先后关系时，在正文写明阻塞事项。自动化产生的专用故障单依照发布者的约定维护。

## 工作分支

从宿主登记的最新开发分支创建短期分支。`feature/` 用于单项功能、修复或文档，`integration/` 用于规则、门控、稳定性或关联变更的集成验证。用小写英文短语描述交付，词间使用一个连字符，词中可以有数字。有本仓 Issue 时，在短语前放正整数编号。同仓并行处理同一 Issue 时分别说明各自的交付。

| 场景 | 示例 |
| --- | --- |
| 为本仓 Issue #42 修复冲突返回 | `feature/42-report-pr-conflicts` |
| 为同一 Issue 验证合并流程 | `integration/42-check-merge-flow` |
| 一次可验收的独立文档改动 | `feature/document-release-tags` |

普通工作分支使用以下正则整串匹配。跨仓 Issue 的编号放在 PR 正文中，以完整仓库名引用。长期分支、发布分支和宿主登记的其他专用分支按负责仓的配置命名。

```regex
^(feature|integration)/([1-9][0-9]*-)?[a-z][a-z0-9]*(-[a-z0-9]+)*$
```

## PR 和交付

1. **标题。** 使用 `type(scope): 结果`，scope 可省略。type 选 `feat`、`fix`、`docs`、`refactor`、`perf`、`test`、`build`、`ci`、`chore` 或 `revert`。破坏接口时可在冒号前加 `!`，并在正文注明受影响的使用方和迁移方法。例如 `fix(pr): 冲突时立即结束等待` 或 `docs: 补充正式版安装步骤`。
2. **正文。** 写明改动、使用方、关联 Issue、实际检查及其结果、剩余验收。已有 CI 证据可以直接链接。宿主启用独立的 `collaboration.github-agent-identity` 规则时，执行操作的 agent 依其约定追加实际事件。[GitHub 工作事件](../README.md#github-工作事件)
3. **关联与关闭。** 跟踪进展用 `Refs #42`，跨仓用 `Refs owner/repo#42`。合入默认分支即可满足该 Issue 全部验收时，可使用 `Closes #42` 或 `Closes owner/repo#42`。先进入 integration 的阶段 PR 使用 `Refs`。尚待发布、落地 CI 或实际下游采用时，保留 Issue，取得对应证据后关闭。
4. **合入。** 按负责仓登记的目标和检查合入。采用分阶段集成时，先进入该 integration，验收后由 integration 向开发分支提 PR。独立 PR 可并行验收。head 更新后按新 SHA 核对。遇到冲突时解决冲突并验证受影响范围。
5. **交付。** 记录合并 SHA、实际落地检查和 Issue 要求的发布或下游证据。等待 PR、push 或 CI 时使用完成事件，终态后核对原运行。验证完成后按宿主生命周期清理已交付的短期分支和工作树。

## 宿主如何选择

`.chrono-harness/rules.json` 缺省时采用默认规则。若只屏蔽本文，使用 `{"mode":"all","exclude":["collaboration.github-contribution"]}`。若只选本文，使用 `{"mode":"only","include":["collaboration.github-contribution"]}`。身份事件有独立的规则 ID，贡献约定和身份协议可分别选择。设置 `{"mode":"off"}` 关闭继承，宿主自己的约定仍可写在根指南的生成区块之外。[规则选择与同步](../README.md)
