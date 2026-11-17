# 待办与未完成事项

整理时间：2026-11-17。范围：0.1.0 发版前未完成的工作、已知问题与规划中未启动的功能。第 7 节四项规划功能已于同日完成实现。

## 1. 阻塞项：0.1.0 发版（进行中，已中断）

当前状态：

| 项目 | 状态 |
| --- | --- |
| 版本号 | `package.json` / `tauri.conf.json` / `Cargo.toml` 均为 `0.1.0` |
| 本地提交 | 17 个提交未推送（`origin/main` 停在 `7bdad85`） |
| 本地构建验证 | `npm run tauri:build` 执行到 Rust 编译阶段被中断，**未完成** |
| tag | `v0.1.0` 未创建 |
| 远端 CI | 最近一次 run `44008967361` 前端/Windows/macOS 通过，Ubuntu 失败（见第 2 节） |

待办（顺序不可颠倒，AGENTS.md 要求本地构建通过后才能推 tag）：

1. 重新执行 `npm run tauri:build`，确认 NSIS 产物生成成功
2. `git push origin main`，确认新触发的 CI 三平台结果
3. `git tag -a v0.1.0 -m "Toolbox 0.1.0"` 并 `git push origin v0.1.0`
4. 等待 GitHub Actions 四平台构建，产出草稿 Release
5. 人工填写 Release 描述（分节，见下方草稿），再发布

### Release 描述草稿（0.1.0 为首个版本，兼容性无破坏性变更）

**新增**

- 翻译：多翻译服务商、历史记录
- m3u8 / 视频下载：画质档位、MP4 优先、MP3 音频提取、Referer/Cookie/代理
- 定时任务：每天/每周/每月/间隔/自定义 cron，cron 预设与中文语义提示
- 开发工具：JSON 格式化与 TS 类型生成、编解码（时间戳/UUID/Base64/URL）、正则测试、curl ↔ fetch / axios / Python 转换、随机密码、大小写与命名转换
- AI 工具：多模型并行调试台、Prompt 模板与变量替换、Token 用量统计；API Key 集中管理（脱敏、复制确认、过期提醒）
- 数据库浏览器：SQLite 文件、PostgreSQL 与 MySQL 只读浏览、结构树与 SQL 查询、分页与列宽记忆
- 多端自启：Windows 注册表 / macOS launchd / Linux XDG autostart
- 设置页：显示实际数据目录

**改动**

- 组件库切换为全局中文语言
- 命令失败信息标注所用 Shell，并提示 cmd 不支持 PowerShell 语法
- 下载器、调度器、工具安装等模块按职责拆分

**优化**

- 前端按视图分包，主包 12 KB，消除体积告警（后续回归见第 4 节）
- 工具安装 SHA256 校验为 fail-closed，流式计算哈希
- 开发工具长文本限制高度并支持滚动，卡片等高对齐

**兼容性**

- 首个版本，无历史数据迁移问题；数据目录 `%APPDATA%\Toolbox`

## 2. CI：Ubuntu 单测（已修复）

- run `44008967361`（提交 `7bdad85`）：frontend、rust (macOS)、rust (Windows) 通过；rust (ubuntu-22.04) 失败，`cargo test` 约 50 分钟后被整体杀掉，日志缺失
- 根因：`kill_tree` 通过 `kill -9 -<pid>` 子进程清理进程组，procps 的 `kill` 对负 PID 参数解析不一致，信号实际打到了调用方自己所在的进程组（runner 的组），导致测试进程与 `timeout` 一起被 SIGKILL，而真正的目标 `sh`/`sleep` 存活为孤儿进程
- 修复：改为直接调用 `kill(2)`（`src-tauri/src/core/proc.rs`），仅在目标确为进程组组长时按组发送；调度器与下载器共用该实现；命令执行在终止后增加兜底期限，回收失败也不会永久阻塞
- 验证：run `44029486162` 四平台全绿，Ubuntu 63 个单测 0.86s 通过；新增 `core::proc::tests::kills_child_in_its_own_process_group` 防回归

## 3. 超过 400 行的文件

项目要求单文件不超过 400 行，以下文件待拆分：

| 文件 | 行数 | 建议拆分 |
| --- | --- | --- |
| `src/components/tasks/TaskFormModal.vue` | 885 | cron 配置块、命令高级选项、表单逻辑 composable |
| `src/views/SettingsView.vue` | 880 | 工具安装卡片、服务商配置卡片、关于卡片 |
| `src/views/TranslateView.vue` | 606 | 输入区、结果区、语言选择栏 |

拆分属纯结构调整，需独立提交、逐次 `npm run typecheck` 与 `npm run test` 验证。

## 4. 前端体积告警（已解决）

- 原因：所有视图用 `v-show` 常驻挂载，异步视图在启动时全部加载；`manualChunks` 又把 naive-ui 全量打进单个 vendor chunk
- 修复：视图改为 `<KeepAlive>` + 动态组件，首次进入才加载；naive-ui 组件跟随所在视图分包；date-fns / lodash / vue / naive-ui-core 独立分桶
- 结果：最大 chunk 由 766 KB 降至 166 KB（TasksView），首屏 JS 约 934 KB → 约 526 KB，构建告警消失
- 说明：naive-ui 的具名导入本身已按组件 tree-shaking，无需引入 `unplugin-vue-components`

## 5. 视频下载失败未定位

- 工具链正常（yt-dlp 2026.10.26、ffmpeg 就绪），具体失败场景缺日志
- 复现方式：应用内重跑失败任务，读取 `%APPDATA%\Toolbox\jobs.json` 中的完整日志
- 需重点区分：站点页面链接失败、裸 m3u8 分片 403（防盗链需 Referer）、AES 加密流处理失败

## 6. 多端实机验证未完成

本机仅有 Windows，以下行为尚未在真实设备验证：

- macOS：launchd 自启写入 `~/Library/LaunchAgents`、`ffmpeg-static` 下载安装、dmg 安装包
- Linux：XDG autostart（尊重 `XDG_CONFIG_HOME`）、`ffmpeg-static` 下载安装、deb/AppImage 安装包
- Windows：开机自启、下载与安装流程的回归确认

## 7. 规划功能（2026-11-17 已实现）

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

## 8. 规划中未启动

- **MCP Server 模式**：把翻译、下载能力暴露为 MCP 工具供 agent 调用；需先定工具粒度与鉴权方式
- **m3u8 原生下载引擎**：自解析 playlist、分片并发、AES-128 解密、ffmpeg 合并；需先确认 yt-dlp 无法覆盖的真实站点需求再立项
