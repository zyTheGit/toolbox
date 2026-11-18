# Toolbox

面向 Windows / macOS / Linux 的桌面工具箱（Windows 为主验证平台），基于 Tauri 2 + Vue 3 + Naive UI。提供中英文翻译、m3u8 视频下载和通用定时任务，并附带开发工具、AI 调试台与数据库浏览器；常驻系统托盘并支持开机自启。

## 功能要点

### 翻译

- 默认服务在设置中选择（默认微软），翻译页直接使用；主服务失败时按设置的回退列表自动重试。
- 免 Key 可用：默认走微软翻译，失败时按配置回退到 Google 等免 Key 服务。
- 自有 Key：支持 OpenAI、DeepL、百度、有道，Key 在设置页填写。
- 显示实际使用的服务与耗时。
- 译文支持系统语音朗读并随时停止；朗读倍率（0.5x - 2x）在设置中选择，无需额外下载语音包。
- 翻译历史本地保存，默认最多 200 条，支持收藏、关键词搜索及按方向/服务筛选。

### m3u8 下载

- 批量粘贴链接，可同时下载 1 - 5 个任务（设置中配置），避免无限制抢占带宽。
- 分片并发线程统一在设置中配置（1 - 16，默认 4）；支持自定义视频文件名（扩展名自动匹配，多个链接自动追加 01、02 等序号）。
- 实时显示进度、速度、剩余时间与日志，可随时取消单个任务。
- yt-dlp 与 ffmpeg 不随包分发，在设置页检测状态并一键下载到数据目录 `bin\`。
- 支持自定义工具下载镜像与代理；下载目录可在设置中修改。

### 定时任务

- 两种触发方式：按每天 / 每周 / 每月 / 间隔 / 每小时等规则持续重复（选时间和星期即可，无需写 cron，高级用户可切换自定义 cron）；或指定年月日时分秒只执行一次，执行后自动停用。
- 任务类型只支持终端（执行命令）与提醒（系统通知），支持单个启停，以及托盘菜单「暂停/恢复全部」。
- 提醒使用系统通知，标题为任务名称；应用需保持运行（可最小化到托盘）。
- 程序未运行时错过的触发点不补跑；一次性任务错过后标记为跳过并停用。
- 同一任务不会并发执行；每个任务保留最近 20 条执行记录。
- 终端任务受安全开关约束：保存或运行时若未开启，会提示前往设置开启。

### 快捷翻译

- 在设置中开启全局快捷翻译（默认 `CommandOrControl+Shift+T`），按下后唤起主窗口，并读取剪贴板内容作为翻译预览。
- 只预填、不自动请求，避免旧剪贴板内容被误发；在翻译页确认后再发起翻译。
- 快捷键被其他程序占用时注册会失败并提示更换。

### 开发工具

全部在本地处理，不上传网络：

- JSON 工具：格式化、压缩与校验，并可生成 TypeScript 类型。
- 编解码：时间戳、UUID、Base64、URL 等常用转换。
- 正则测试：实时匹配与分组结果。
- curl 转换：curl ↔ fetch / axios / Python requests 双向生成。
- 辅助工具：随机密码、大小写与命名风格转换。

### AI 工具（实验）

- LLM 调试台：请求 OpenAI 兼容接口（非流式，默认超时 120 秒），同一 Prompt 可勾选多个模型并行对比，展示耗时与 Token 用量。
- Prompt 模板支持 `{{变量}}` 占位符，发送前校验必填变量，模板随配置保存。
- 用量按日期与模型聚合在数据目录 `llm-usage.json`，不记录 Prompt 正文。
- API Key 管理器：多条凭据（名称 / 服务 / 密钥 / 备注 / 过期日），默认脱敏显示，复制需二次确认，启动时提醒即将过期的凭据。
- Windows 下密钥使用 DPAPI 加密（绑定当前用户）；macOS / Linux 当前只靠文件权限保护（数据目录 `0700`、`secrets.bin` `0600`），内容等同明文，且未接入系统钥匙串，请勿共享或同步数据目录。
- 实验性质：本机仅验证页面与流程，真实 LLM 调用需自行配置凭据后验证。

### 数据库浏览器（实验）

- SQLite：选择 `.db` / `.sqlite` 文件，以只读方式打开，浏览表结构与数据。
- PostgreSQL / MySQL：填写连接串建立只读会话，支持结构树、库 / 表切换与 SQL 查询。
- 仅允许单条只读语句，结果分页展示；列宽记忆保存在本地。连接历史可命名、复用，凭据保存在本机。
- 不监听端口；PostgreSQL / MySQL 为显式发起的出站连接。
- 实验性质：PostgreSQL / MySQL 尚未在真实实例上验证；不提供写入、DDL 与数据编辑。

### 备份与恢复

- 设置页可一键导出备份、从备份恢复；备份包含配置、翻译历史、定时任务与下载记录。
- 默认导出会剔除密钥、代理、Cookie、请求头与命令输出等敏感信息。
- 导入按不可信输入处理：命令执行开关会被关闭，命令任务需重新确认；导入完成后按提示重启应用。
- 备份不包含 yt-dlp / ffmpeg 等外部工具，恢复后按需重新安装。

## 运行环境

| 场景 | 要求 |
| --- | --- |
| 运行（Windows） | Windows 10/11 x64，Microsoft Edge WebView2 运行时（Windows 11 自带，Windows 10 缺失时需安装） |
| 运行（macOS） | 使用系统 WKWebView；安装包与自启尚未实机验证 |
| 运行（Linux） | 需 webkit2gtk-4.1 等系统库；安装包与自启尚未实机验证 |
| 开发 | Node.js 20+、Rust stable；Windows 另需 Visual Studio 2022 Build Tools（C++ 桌面开发）与 Windows SDK，macOS 需 Xcode Command Line Tools，Linux 需 webkit2gtk-4.1 / libgtk-3 等开发包 |

## 三平台差异

| 项目 | Windows | macOS | Linux |
| --- | --- | --- | --- |
| 安装包 | NSIS 安装包 + 便携版 zip | dmg / .app | deb / AppImage |
| 数据目录 | `%APPDATA%\Toolbox` | 系统 data_dir 下的 `Toolbox` | 系统 data_dir 下的 `Toolbox` |
| 开机自启 | 注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | launchd 用户代理 | XDG autostart |
| 密钥保护 | DPAPI（绑定当前用户） | 仅文件权限 `0600`，等同明文，未接系统钥匙串 | 仅文件权限 `0600`，等同明文，未接系统钥匙串 |
| 渲染内核 | WebView2 | WKWebView | WebKitGTK |
| 实机验证 | 已覆盖 | 未实机验证 | 未实机验证 |

数据目录的实际路径以设置页「数据目录」展示为准；macOS 通常为 `~/Library/Application Support/Toolbox`，Linux 通常为 `~/.local/share/Toolbox`。

## 开发与构建

```bash
npm install
npm run typecheck     # vue-tsc 前端类型检查
npm run test          # 前端单测（node --test）
npm run build         # 类型检查 + 前端构建
node scripts/check-version.mjs       # 三处版本号（含 tag）一致性
node scripts/check-file-length.mjs   # 单文件 400 行基线检查
npm run tauri:dev     # 开发调试
npm run tauri:build   # 生成 release 与安装包
npm run package:portable            # Windows 便携版 zip
```

NSIS 安装包输出到 `src-tauri/target/release/bundle/nsis/`。构建产物依赖版本与目标架构，文件名以实际输出为准。发版步骤见 [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md)。

## 便携版

先完成 `npm run tauri:build`，再执行：

```bash
npm run package:portable
```

脚本会把 `src-tauri/target/release/toolbox.exe` 复制为 `portable/Toolbox-<version>.exe`，附带 `便携版说明.txt`，并压缩为 `portable/Toolbox-<version>.zip`。便携版免安装，但用户数据仍写入 `%APPDATA%\Toolbox`，不会保存在 exe 旁边。

## 数据与工具

默认数据目录在 Windows 为 `%APPDATA%\Toolbox`，macOS / Linux 为系统 data_dir 下的 `Toolbox`（以设置页展示的实际路径为准）：

| 路径 | 用途 |
| --- | --- |
| `config.json` | 应用设置（下载目录、并发数与分片线程、代理、镜像、翻译服务、安全开关、Prompt 模板等） |
| `history.json` | 翻译历史与收藏 |
| `jobs.json` | 下载任务列表与状态 |
| `tasks.json` | 定时任务定义 |
| `runs/` | 定时任务执行记录（每个任务最近 20 条） |
| `llm-usage.json` | LLM 用量统计（按日期 / 模型聚合） |
| `secrets.bin` | API Key 与数据库连接凭据（Windows 使用 DPAPI；macOS / Linux 仅文件权限 `0600`，内容等同明文） |
| `bin/` | 按需下载的 yt-dlp、ffmpeg 等工具 |
| `downloads/` | 默认下载目录，可在设置中更改 |

yt-dlp 与 ffmpeg 不随安装包或便携版分发，以避免体积与升级问题；设置页可检测状态并一键安装。工具下载镜像和网络代理均在设置中配置，适用于工具安装与视频下载。

工具安装的信任锚：镜像必须使用 `https://`（http 镜像会被直接拒绝），且只用于二进制下载；`SHA2-256SUMS` / `.sha256` 校验文件始终从官方源获取，不经过镜像。校验文件不可达、格式异常或哈希不匹配时终止安装、清理临时文件并保留旧版本，不会静默安装未通过校验的二进制；二进制与校验文件分别限制 200 MB / 2 MB。

Unix（macOS / Linux）下数据目录与子目录权限为 `0700`、`secrets.bin` 为 `0600`；已存在的目录与文件也会在启动时收紧。

## 命令执行安全

- 执行命令默认关闭，必须在设置中显式开启。
- 以当前用户权限运行，不提权，不监听任何网络端口。
- 命令、环境变量等参数以明文保存在本机 `tasks.json`，不要写入密钥等敏感信息。
- 命令任务串行执行；默认超时 10 分钟，超时会终止整个进程树。

## 开机自启

自启通过当前用户注册表项实现：

```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run
值名：Toolbox
数据："<程序绝对路径>" --autostart
```

关闭主窗口只会隐藏到托盘，程序继续驻留；需要退出时使用托盘菜单「退出」。`--autostart` 为自启参数，配合启动设置可实现最小化启动或延迟显示窗口。

macOS 使用 `~/Library/LaunchAgents` 下的 launchd 用户代理，Linux 使用 `$XDG_CONFIG_HOME/autostart`（缺省为 `~/.config/autostart`）下的 XDG 自启项，启动参数同为 `--autostart`。两者的实机行为尚未验证。

## 常见问题

- **提示缺少 yt-dlp**：在「设置 → 下载与工具」中一键安装，或手动指定已有的 `yt-dlp.exe` 路径。m3u8 下载依赖它。
- **合并视频失败**：通常是缺少 ffmpeg 或路径失效。重新安装 ffmpeg，并确认设置中的工具路径存在。
- **无法写入下载目录**：检查目录是否存在、当前用户是否有写权限；必要时改到用户目录下的可写路径。
- **下载或工具安装很慢**：在设置中填写代理地址或切换工具下载镜像后重试。
- **开机自启失效**：移动或重命名程序后，注册表中的旧路径会失效。重新开关一次自启，或删除上述 Run 值后重新开启。
- **窗口打不开或白屏**：确认 WebView2 运行时已安装；Windows 10 可安装 Evergreen Runtime。

## 许可证

内部使用（Internal Use）。仓库未附带开源许可证，未经授权请勿对外分发。
