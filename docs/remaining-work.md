# 待办与未完成事项

整理时间：2026-11-17。范围：0.1.0 发版前未完成的工作、已知问题与规划中未启动的功能。

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
- 开发工具：JSON 格式化与 TS 类型生成、编解码（时间戳/UUID/Base64/URL）、正则测试、随机密码、大小写与命名转换
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

## 2. CI：Ubuntu 单测未通过

- run `44008967361`（提交 `7bdad85`）：frontend、rust (macOS)、rust (Windows) 通过；rust (ubuntu-22.04) 失败
- Ubuntu job 中 `cargo check` 通过，`cargo test` 运行约 50 分钟后失败，步骤无结论，日志接口暂不可取
- 初步判断：某个测试在 Linux 下挂起（涉及进程、文件锁或自启路径），而非编译错误
- 待办：重跑该 job 取日志；若确认挂起，为相关测试加超时，并在 Linux 环境（WSL/容器）复现

## 3. 超过 400 行的文件

项目要求单文件不超过 400 行，以下文件待拆分：

| 文件 | 行数 | 建议拆分 |
| --- | --- | --- |
| `src/components/tasks/TaskFormModal.vue` | 885 | cron 配置块、命令高级选项、表单逻辑 composable |
| `src/views/SettingsView.vue` | 880 | 工具安装卡片、服务商配置卡片、关于卡片 |
| `src/views/TranslateView.vue` | 606 | 输入区、结果区、语言选择栏 |

拆分属纯结构调整，需独立提交、逐次 `npm run typecheck` 与 `npm run test` 验证。

## 4. 前端体积告警回归

- 引入开发工具组件后，`naive-ui` vendor chunk 增至 764 KB，超过 750 KB 告警阈值
- 待办：改为按需引入 naive-ui 组件，或按组件分桶；确认后再决定是否调整阈值

## 5. 视频下载失败未定位

- 工具链正常（yt-dlp 2026.10.26、ffmpeg 就绪），具体失败场景缺日志
- 复现方式：应用内重跑失败任务，读取 `%APPDATA%\Toolbox\jobs.json` 中的完整日志
- 需重点区分：站点页面链接失败、裸 m3u8 分片 403（防盗链需 Referer）、AES 加密流处理失败

## 6. 多端实机验证未完成

本机仅有 Windows，以下行为尚未在真实设备验证：

- macOS：launchd 自启写入 `~/Library/LaunchAgents`、`ffmpeg-static` 下载安装、dmg 安装包
- Linux：XDG autostart（尊重 `XDG_CONFIG_HOME`）、`ffmpeg-static` 下载安装、deb/AppImage 安装包
- Windows：开机自启、下载与安装流程的回归确认

## 7. 规划中未启动

按优先级排列，均未开始。开发者向功能的共同点：只复用现有后端能力，不新增端口，不引入外部服务。

### 7.1 LLM 调试台（P0）

- 目标：同一 Prompt 并行请求多模型（GLM / OpenAI / DeepSeek），并排对比响应；Prompt 模板管理与变量替换；Token 用量统计
- 复用：翻译模块已有的 provider 抽象与服务商回退链（`core/translate/`）、`core/secrets.rs` 凭据加密
- 要点：模板变量用 `{{name}}` 占位并在发送前校验缺失项；用量按模型/日期聚合；流式响应需按 provider 分别适配，非流式先落地
- 归属：新增「AI 工具」页，与「开发工具」并列

### 7.2 API Key 管理器（P0）

- 目标：所有服务凭据集中存储、一键复制、过期时间提醒（提醒可挂现有定时任务）
- 复用：DPAPI 加密存储（`core/secrets.rs`）已就绪，定时任务调度已就绪
- 要点：现有 `SecretPatch` 为固定四个字段，需改为可扩展的条目列表（名称/服务/密钥/备注/过期日）并兼容旧格式迁移；列表默认脱敏，复制需二次确认

### 7.3 SQLite / PostgreSQL 浏览器（P1）

- 目标：本地 SQLite 文件直读，表结构与数据浏览、只读 SQL 查询；PostgreSQL 走连接串
- 约束：SQLite 直接读文件，不监听端口；PG 为显式发起的出站连接
- 要点：SQLite 用 `rusqlite`（bundled）避免依赖系统库；查询结果分页与列宽记忆；写操作默认禁用

### 7.4 curl ↔ 代码转换器（P1）

- 目标：粘贴 curl 转 fetch / axios / Python requests，反向也支持
- 复用：开发工具页已有的卡片式布局与结果区（限高 + 滚动）
- 要点：解析 `-X` / `-H` / `-d` / `--data-raw` / `-F` / `-u` / `--cookie`，生成代码时做语法高亮；纯前端实现，无后端依赖

### 7.5 其他

- **MCP Server 模式**：把翻译、下载能力暴露为 MCP 工具供 agent 调用；需先定工具粒度与鉴权方式
- **m3u8 原生下载引擎**：自解析 playlist、分片并发、AES-128 解密、ffmpeg 合并；需先确认 yt-dlp 无法覆盖的真实站点需求再立项
