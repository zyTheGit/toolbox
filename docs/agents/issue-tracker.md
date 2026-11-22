# Issue tracker: GitHub

本仓库的 issue 与 spec 都存放在 GitHub Issues，所有操作统一走 `gh` CLI。

## 约定

- **创建**：`gh issue create --title "..." --body "..."`；多行正文用 heredoc。
- **读取**：`gh issue view <number> --json number,title,body,labels,comments`。
- **列表**：`gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'`，按需加 `--label` / `--state`。
- **设为父 issue 的子 issue**：`gh issue create --parent <parent> ...`，或事后 `gh issue edit <parent> --add-sub-issue <child>`（`gh` 2.94+）。旧版：`gh api --method POST repos/<owner>/<repo>/issues/<parent>/sub_issues -F sub_issue_id=<child-db-id>`。不支持子 issue 时，在子 issue 正文顶部写 `Part of #<parent>`。
- **评论**：`gh issue comment <number> --body "..."`。
- **打 / 移除标签**：`gh issue edit <number> --add-label "..."` / `--remove-label "..."`。
- **关闭**：`gh issue close <number> --comment "..."`。

仓库由 `gh` 从当前 clone 自动推断（当前为 `zyTheGit/toolbox`）。

## Pull requests as a triage surface

**PRs as a request surface: no.**（若本仓库把外部 PR 当作需求来源，改为 `yes`；`/triage` 会读这个开关。）

置为 `yes` 时，PR 与 issue 走同一套标签与状态，命令换成 `gh pr` 系列：

- **读取**：`gh pr view <number> --comments`、`gh pr diff <number>`。
- **列出待 triage 的外部 PR**：`gh api --paginate 'repos/{owner}/{repo}/pulls?state=open' --jq '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") | not) | {number, title, author: .user.login, author_association, labels: [.labels[].name]}'`。
- **评论 / 打标签 / 关闭**：`gh pr comment`、`gh pr edit --add-label`/`--remove-label`、`gh pr close`。

GitHub 的 issue 与 PR 共用一个编号空间，裸写 `#42` 可能是任一者：先 `gh pr view 42`，失败再 `gh issue view 42`。

## 技能说"发布到 issue tracker"时

创建 GitHub issue。

## 技能说"取对应工单"时

按上面 **读取** 操作。

## Wayfinding 操作

供 `/wayfinder` 使用。**地图**是一个 issue，**子 issue** 是工单。

- **地图**：带 `wayfinder:map` 标签的单个 issue，正文承载 Notes / Decisions-so-far / Fog。`gh issue create --label wayfinder:map`。
- **子工单**：以 GitHub sub-issue 关联到地图（见上文）；不支持时在地图正文的任务列表中加入，并在子工单正文顶部写 `Part of #<map>`。标签 `wayfinder:<type>`（`research`/`prototype`/`grilling`/`task`）。领取后指派给推进者。
- **阻塞**：用 GitHub 原生 issue 依赖。添加边：`gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`，其中 `<blocker-db-id>` 是阻塞者的数值 **database id**（`gh api repos/<owner>/<repo>/issues/<n> --jq .id`），不是 `#number` 或 `node_id`。GitHub 以 `issue_dependencies_summary.blocked_by` 报告未关闭的阻塞者。不可用时退化为子工单正文顶部 `Blocked by: #<n>, #<n>`。所有阻塞者关闭才算解锁。
- **前沿查询**：列出地图下未关闭的子 issue，剔除存在未关闭阻塞者或已有 assignee 的，按地图顺序取第一个。
- **领取**：`gh issue edit <n> --add-assignee @me`，这是本会话的第一次写操作。
- **结清**：`gh issue comment <n> --body "<答案>"` → `gh issue close <n>` → 把上下文指针追加到地图的 Decisions-so-far。
