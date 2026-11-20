# 待办与未完成事项

整理时间：2026-11-20（0.3.2 发版进行中）。0.3.2 新增「检查新版本」并支持跳转下载页；0.3.1 为界面按设计稿重构 + 定时任务复制；0.2.0 新增下载分享文本与站点字幕、原文/译文双朗读；0.1.0 的遗留待办见第 3 节。

## 0. 0.3.2 发版（进行中）

| 项目 | 状态 |
| --- | --- |
| 版本号 | 三处均为 `0.3.2`，`node scripts/check-version.mjs` 已通过 |
| 代码提交 | `e21699c`（检查新版本）；版本号、`Cargo.lock` 与本文档随后单独提交 |
| 功能说明 | 设置 → 关于 → 「新版本」行：启动时静默检查 GitHub Release latest，发现更新显示版本号与发布日期，可「打开下载页」；支持「跳过此版本 / 撤销跳过」，失败可重试 |
| 安全约束 | 接口固定为 `api.github.com/repos/zyTheGit/toolbox/releases/latest`；跳转前在 Rust 侧校验 URL 必须属于 `github.com/zyTheGit/toolbox/releases/` 前缀；请求遵循全局代理配置 |
| 验证 | 开发版实测：已是最新 / 线上更新 / 当前版本更高 / 跳过与撤销 / 外部 URL 被拒（`[DATA] 下载地址不属于本仓库的发布页`）/ 真实发布页跳转 |
| 构建 / 冒烟 / 推送 / tag | 待完成，见下方待办 |

### 待办

1. `npm run build` → `npm run tauri:build` → `npm run package:portable`，核对产物时间晚于 HEAD
2. 安装 NSIS 冒烟：版本徽标 v0.3.2、7 页渲染、设置 → 关于「新版本」行、关窗隐藏
3. 提交（版本号 + `Cargo.lock` + 文档）→ `git push origin main` → CI 四平台全绿
4. `v0.3.2` tag → 草稿 Release → 核对四平台产物与 `SHA256SUMS-*.txt` → 填写描述并发布

### Release 描述草稿（0.3.2）

**新增**

- 设置 → 关于新增「新版本」：启动时静默检查线上最新版本，发现更新显示版本号与发布日期，可一键打开下载页
- 支持「跳过此版本」与「撤销跳过」，跳过记录保存在本地

**优化**

- 下载页跳转在 Rust 侧校验仓库前缀，避免打开任意外部地址
- 检查请求遵循设置中的全局代理

**兼容性**

- 配置、备份与数据目录格式未变，可直接覆盖安装
- 新增对 GitHub API 的网络请求；离线或接口不可用时仅提示检查失败，不影响其它功能

## 1. 0.3.1 发版（已发布）

| 项目 | 状态 |
| --- | --- |
| 版本号 | 三处均为 `0.3.1`，`node scripts/check-version.mjs` 通过 |
| 代码提交 | `9bd26f8`（设计稿）、`b5d8d75`（界面重构）、`932af33`（任务复制）、`f4422e6`（版本号）、`621bf67`（文档与发版）；0.3.0 起的 `8b0be6f`、`ff8c89e`、`2482c49`、`39b57e4`、`a77bc07` 随本次一并推送 |
| 回归测试 | `npm run typecheck`、`npm run test`（82 个）全绿；`cargo test` 全绿 |
| 发版构建 | 本地重建后安装冒烟；正式产物以 CI 为准：NSIS、便携版 zip 等 12 个产物，Windows NSIS SHA256 `2beab963…` 与 `SHA256SUMS-windows-x64.txt` 一致 |
| 安装冒烟 | 本地 NSIS 复装：版本徽标 v0.3.1、7 页渲染、任务列表与复制预填正常、无控制台异常；另验证工具检测（yt-dlp 2026.10.26 / ffmpeg 9.0.2 就绪）与关闭窗口隐藏且进程存活 |
| 推送 / CI | 已推送至 `621bf67`；CI run `44690644528` 四平台全绿 |
| tag / Release | tag `v0.3.1`；Release run `44691927510` 成功，草稿 12 个产物核对后于 2026-11-20 07:23 UTC 正式发布（非预发布） |
| 备注 | 仓库原为私有，Actions 因账单额度被拒，经授权改为公开仓库后 CI 恢复 |

### Release 描述（0.3.1，已发布）

**新增**

- 定时任务支持复制：任务卡片新增「复制任务」按钮，打开预填源任务的新建表单（名称自动追加「（副本）」），保存后生成独立任务

**改动**

- 界面按 `design/` 设计稿整体重做：统一左侧导航与卡片布局，翻译 / 下载 / 定时任务 / 开发工具 / 数据库 / AI 工具 / 设置 七个页面全部重写
- 翻译历史由抽屉改为弹框，新增筛选栏与「最近翻译」
- 设置页拆分为下载 / 翻译 / 命令执行 / 通知 / 工具 / 关于等独立卡片，底部统一保存栏
- 任务表单拆分为触发方式与载荷子组件，cron 提供中文语义摘要与未来 3 次执行时间预览
- 凭据过期日期改用日期选择器

**优化**

- 去除全局装饰动画与弹窗淡入 / 缩放过渡，桌面端优先响应速度
- 视图改为静态导入并按页缓存，菜单切换不再出现白屏
- 修复翻译历史弹窗「删除」「清空历史」在浅色底看不清（错误按钮前景 / 背景色分离）
- 修复任务列表刷新导致卡片顺序跳动；系统通知点击直达对应任务；任务卡片操作按钮补充加载状态
- `TaskFormModal.vue`、`SettingsView.vue`、`TranslateView.vue` 拆分到 400 行以内

**兼容性**

- 配置、备份与数据目录格式未变（配置仍为 schemaVersion 1），可直接覆盖安装
- 任务复制为纯前端表单行为，不涉及后端命令与数据格式变更
- 已知未实机验证项：macOS / Linux 安装包与自启、LLM 实际调用、PostgreSQL / MySQL 真实实例连接、托盘菜单项交互、备份导入导出、快捷翻译快捷键

## 2. 0.2.0 发版（已发布）

| 项目 | 状态 |
| --- | --- |
| 版本号 | 三处均为 `0.2.0`，`node scripts/check-version.mjs` 通过 |
| 代码提交 | 截至 `0405f63`，按功能拆分为 `8bf5b54`、`5aa30b8`、`5d99ccd`、`2b699ae`、`9a9759f`、`0405f63` |
| CI 门禁 | 发版代码 run `44471022882`、文档提交 run `44475098866` 均四平台全绿 |
| tag | `v0.2.0`（annotated）已推送，指向 `0405f63` |
| 发版构建 | Release run `44473244426` 四平台全绿 |
| Release | `Toolbox v0.2.0` 已于 2026-11-19 06:52 UTC 发布（非预发布） |
| 产物 | NSIS、便携版 zip、两个 dmg、两个 app.tar.gz、deb、AppImage 与 4 份 `SHA256SUMS-*.txt` 共 12 个附件 |
| 产物复核 | 已下载 Windows 安装包与便携包，哈希与 `SHA256SUMS-windows-x64.txt` 一致（`a52b6b47…` / `0592a54a…`） |
| 安装冒烟 | 已完成：安装/启动、7 页渲染、关闭窗口隐藏、单实例唤起、开机自启开关（注册表项验证后已恢复）、便携版启动 |
| 描述 | 已按新增 / 改动 / 优化 / 兼容性分节填写 |
| 未自动化验证 | 托盘菜单项交互、备份导入导出、快捷翻译快捷键、下载目录切换 |

## 3. 0.1.0 发版（已发布，NSIS 安装冒烟待补做）

当前状态：

| 项目 | 状态 |
| --- | --- |
| 版本号 | `package.json` / `tauri.conf.json` / `Cargo.toml` 均为 `0.1.0`，`node scripts/check-version.mjs` 通过 |
| 推送状态 | `origin/main` 已更新至 `cc5614a`；T0-T7 与两处 CI 测试修复均已推送 |
| 本地构建验证 | 2026-11-18 11:43 已在 T0-T7 提交后的 HEAD 重建 NSIS 与便携版，并完成 7 页运行冒烟；产物校验值见交接文档附录 C.3 |
| tag | `v0.1.0`（annotated）已推送，指向 `cc5614a` |
| CI 门禁 | run `44234306466` 四平台全绿（frontend + Windows / macOS / Ubuntu Rust） |
| 发版构建 | Release run `44234824785` 四平台全绿 |
| Release | `Toolbox v0.1.0` 已于 2026-11-18 04:42 UTC 发布（非预发布）；描述分节完整，Windows 产物哈希已与 `SHA256SUMS-windows-x64.txt` 比对一致 |
| 发版基线（T0） | 已新增 `scripts/check-version.mjs`、`scripts/check-file-length.mjs`、`scripts/sha256sums.mjs`、`docs/RELEASE_CHECKLIST.md`；CI / Release workflow 与 CSP 已加固，Action 已固定到 commit SHA |

待办：

1. 已完成：按功能拆分提交 T0-T7 改动（`6d19b24` 至 `8261964`）
2. 已完成：在 HEAD 重新执行 `npm run typecheck`、`npm run test`、`npm run build`、`npm run tauri:build`、`npm run package:portable`，并完成便携版 7 页运行冒烟
3. 已完成：`node scripts/check-version.mjs` 与 `node scripts/check-file-length.mjs` 通过
4. **待授权**：按 `docs/RELEASE_CHECKLIST.md` 完成 NSIS 安装冒烟（托盘、自启、备份导入导出等），安装会写入本机应用状态
5. 已完成：`git push origin main`，CI run `44234306466` 四平台全绿
6. 已完成：`git tag -a v0.1.0 -m "Toolbox 0.1.0"` 并推送，tag 指向 `cc5614a`
7. 已完成：Release run `44234824785` 四平台构建全绿
8. 已完成：复核产物与 `SHA256SUMS-*.txt`，填写发布描述；Windows 安装包 / 便携包实测哈希一致
9. 已完成：已发布 Release `v0.1.0`（2026-11-18 04:42 UTC）；仓库为私有，需登录且有权限才能访问
10. **待补做**：NSIS 安装冒烟（托盘、自启、备份导入导出），安装会写入本机应用状态

### Release 描述草稿（0.1.0 为首个版本，兼容性无破坏性变更）

**新增**

- 翻译：多翻译服务商、历史记录、收藏与筛选、系统朗读、全局快捷翻译
- m3u8 / 视频下载：画质档位、MP4 优先、MP3 音频提取、Referer/Cookie/代理、批量链接与并发控制
- 定时任务：每天/每周/每月/间隔/自定义 cron，cron 预设与中文语义提示；一次性任务与提醒
- 开发工具：JSON 格式化与 TS 类型生成、编解码（时间戳/UUID/Base64/URL）、正则测试、curl ↔ fetch / axios / Python 转换、随机密码、大小写与命名转换
- AI 工具（实验）：多模型并行调试台、Prompt 模板与变量替换、Token 用量统计；API Key 集中管理（脱敏、复制确认、过期提醒）
- 数据库浏览器（实验）：SQLite 文件、PostgreSQL 与 MySQL 只读浏览、结构树与 SQL 查询、分页与列宽记忆、连接历史
- 备份与恢复：导出 / 导入配置、翻译历史、任务与下载记录，默认脱敏
- 多端自启：Windows 注册表 / macOS launchd / Linux XDG autostart
- 设置页：显示实际数据目录

**改动**

- 配置保存改为按 `settings` / `llm` / `security` 分节提交并携带配置版本号；版本冲突时只允许“保留我的 / 用磁盘的”，不再整包覆盖
- 备份导入按不可信输入处理：关闭命令执行开关、禁用命令任务、清除代理 / Cookie / 环境变量 / 运行输出等敏感或运行态字段，并保留恢复前快照
- 备份导出默认剔除命令、环境变量、输出与 URL 凭据；可显式勾选“包含命令与输出”
- 下载凭据只保存在进程内存，`jobs.json` 不再写入代理、Cookie、Referer 与额外 Header；重启后重试会提示重填
- 工具校验文件固定从官方源获取，镜像只用于二进制下载；镜像强制 https，二进制 / 校验文件分别限制大小
- 数据库会话上限 8 个、空闲 30 分钟回收；SQLite 长查询 15 秒中断；PostgreSQL 只读语句白名单；前端丢弃过期的查询响应
- 调度任务写盘改为持锁完成，避免旧快照覆盖新建、删除或完成状态
- 组件库切换为全局中文语言
- 命令失败信息标注所用 Shell，并提示 cmd 不支持 PowerShell 语法

**优化**

- 前端按视图分包，主包 12 KB，消除体积告警
- 工具安装 SHA256 校验为 fail-closed，流式计算哈希；校验失败保留旧版本
- Unix 下数据目录 `0700`、`secrets.bin` `0600`，启动时收紧已有权限；旧版内联密钥迁移后立即写回
- 开发工具长文本限制高度并支持滚动，卡片等高对齐
- 下载器、调度器、工具安装等模块按职责拆分

**兼容性**

- 首个正式版本，无历史版本升级的破坏性变更
- 旧 `jobs.json` 中的明文下载凭据会在启动时迁入内存并从磁盘清除；重启后需要重填才能继续重试
- 旧 `config.json` 中的内联密钥会迁移到 `secrets.bin`，迁移后配置文件不再包含密钥
- macOS / Linux 当前仅靠文件权限保护密钥，内容等同明文，未接入系统钥匙串；请勿共享或同步数据目录
- 数据目录 Windows 为 `%APPDATA%\Toolbox`，macOS / Linux 为系统 data_dir 下的 `Toolbox`
- 备份不包含 yt-dlp / ffmpeg，恢复后需按需重新安装
- 已知未实机验证项：macOS / Linux 安装包与自启、LLM 实际调用、PostgreSQL / MySQL 真实实例连接；Windows 安装包仅完成便携版运行冒烟


## 4. CI：Ubuntu 单测（已修复）

- run `44008967361`（提交 `7bdad85`）：frontend、rust (macOS)、rust (Windows) 通过；rust (ubuntu-22.04) 失败，`cargo test` 约 50 分钟后被整体杀掉，日志缺失
- 根因：`kill_tree` 通过 `kill -9 -<pid>` 子进程清理进程组，procps 的 `kill` 对负 PID 参数解析不一致，信号实际打到了调用方自己所在的进程组（runner 的组），导致测试进程与 `timeout` 一起被 SIGKILL，而真正的目标 `sh`/`sleep` 存活为孤儿进程
- 修复：改为直接调用 `kill(2)`（`src-tauri/src/core/proc.rs`），仅在目标确为进程组组长时按组发送；调度器与下载器共用该实现；命令执行在终止后增加兜底期限，回收失败也不会永久阻塞
- 验证：run `44029486162` 四平台全绿，Ubuntu 63 个单测 0.86s 通过；新增 `core::proc::tests::kills_child_in_its_own_process_group` 防回归

## 5. 超过 400 行的文件

项目要求单文件不超过 400 行。存量超限文件写入 `scripts/file-length-baseline.json`：基线只允许变小，新增超限文件直接失败。2026-11-20（0.3.1 界面重构后）实测仅剩 2 个：

| 文件 | 行数 | 建议拆分 |
| --- | --- | --- |
| `src-tauri/src/core/secrets.rs` | 500 | 编解码、条目管理、迁移逻辑 |
| `src/types.ts` | 428 | 按域拆分类型（AI / 任务 / 设置；下载域已拆到 `src/types/download.ts`） |

0.3.1 界面重构已把 `TaskFormModal.vue`（993 → 108）、`SettingsView.vue`（919 → 262）、`TranslateView.vue`（635 → 398）拆到 400 行以内并移出基线；`engine.rs` 已在 0.2.0 收窄到 397 行。

行数以 `node scripts/check-file-length.mjs` 输出为准；拆分收窄后运行 `--update` 更新基线。拆分属纯结构调整，需独立提交、逐次 `npm run typecheck` 与 `npm run test` 验证。

## 6. 前端体积（已解决，0.3.1 再调整）

- 0.2.0：所有视图用 `v-show` 常驻挂载，异步视图在启动时全部加载；修复为 `<KeepAlive>` + 动态组件，最大 chunk 766 KB → 166 KB。
- 0.3.1：为解决首切白屏，视图改为静态导入（桌面端资源在本地，切换零等待），视图代码并入 `index` chunk（约 696 KB，gzip 191 KB）；`vue`、`naive-ui-core`、`date-fns`、`lodash` 仍单独分桶，无构建告警。桌面本地加载不受网络影响，可接受。

## 7. 视频下载失败未定位

- 工具链正常（yt-dlp 2026.10.26、ffmpeg 就绪），具体失败场景缺日志
- 复现方式：应用内重跑失败任务，读取 `%APPDATA%\Toolbox\jobs.json` 中的完整日志
- 需重点区分：站点页面链接失败、裸 m3u8 分片 403（防盗链需 Referer）、AES 加密流处理失败

## 8. 多端实机验证未完成

本机仅有 Windows，以下行为尚未在真实设备验证：

- macOS：launchd 自启写入 `~/Library/LaunchAgents`、`ffmpeg-static` 下载安装、dmg 安装包
- Linux：XDG autostart（尊重 `XDG_CONFIG_HOME`）、`ffmpeg-static` 下载安装、deb/AppImage 安装包
- Windows：开机自启、下载与安装流程的回归确认

## 9. 规划功能（2026-11-17 已实现）

四项均已完成编码并通过类型检查与单测。Windows 实机点检（2026-11-17）已覆盖：

- AI 工具：LLM 调试台渲染正常（模板、变量、多模型勾选、用量面板、模型配置折叠区），API Key 管理器空态与新增凭据弹窗正常
- 数据库：SQLite 文件连接成功，结构树两表可切换，结果表分页 / NULL 样式 / 行数区间正常，多语句被拦截（`[PARAM] 一次只能执行一条 SQL 语句`）且数据未受影响
- 开发工具 → curl 转换：示例命令解析出 POST / URL / 请求体摘要，fetch 与 Python 生成结果带语法着色

尚未验证：LLM 实际调用（本机无凭据，仅走通页面）、PostgreSQL / MySQL 实际连接（无可用实例）。

### 7.1 LLM 调试台（P0）— 已完成

- 后端：`core/llm/` 并行请求 OpenAI 兼容接口（非流式，120 秒超时），`llm-usage.json` 按日期 / 模型聚合用量
- 前端：新增「AI 工具」页（`views/AiView.vue`），Prompt 模板与 `{{name}}` 变量校验、多模型并排结果卡（耗时 / Token）、用量面板
- 模型只保存凭据条目 ID，实际密钥由 API Key 管理器维护

### 7.2 API Key 管理器（P0）— 已完成

- `core/secrets.rs` 改为可扩展条目列表（名称 / 服务 / 密钥 / 备注 / 过期日），旧版四字段格式自动迁移并重写为 V2
- 内置翻译凭据继续通过 `SecretPatch` 写入，兼容原设置页
- 前端：默认脱敏、复制需二次确认、过期标签与启动时提醒

### 7.3 SQLite / PostgreSQL / MySQL 浏览器（P1）— 已完成

- 新增「数据库」页：SQLite 文件直读（`rusqlite` bundled，只读打开），PostgreSQL（`tokio-postgres` + rustls）与 MySQL（`mysql_async` + rustls）连接串，会话只读 + 15 秒超时
- 结构树、只读 SQL、分页、列宽记忆（localStorage）；仅允许单条语句，SELECT / WITH / VALUES 自动包裹分页
- 不监听端口；PostgreSQL / MySQL 为显式发起的出站连接

### 7.4 curl ↔ 代码转换器（P1）— 已完成

- `utils/curl.ts`：tokenizer 支持引号 / 转义 / 续行，解析 `-X / -H / -d / --json / -F / -u / -b / -A / -G` 等；生成 fetch / axios / Python requests，反向由请求生成 curl
- 「开发工具 → curl 转换」页：左右分栏、轻量语法着色、复制按钮
- 单测覆盖解析、生成与往返一致性（`tests/curl.test.ts`）

## 10. 规划中未启动

- **MCP Server 模式**：把翻译、下载能力暴露为 MCP 工具供 agent 调用；需先定工具粒度与鉴权方式
- **m3u8 原生下载引擎**：自解析 playlist、分片并发、AES-128 解密、ffmpeg 合并；需先确认 yt-dlp 无法覆盖的真实站点需求再立项
