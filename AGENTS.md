# AGENTS.md

桌面工具箱（内部使用）。翻译 / m3u8 下载 / 定时任务，常驻托盘。

## 技术栈

- 前端：Vue 3 + TypeScript + Naive UI + Vite（`src/`）
- 后端：Rust + Tauri 2（`src-tauri/`），核心逻辑在 `src-tauri/src/core/` 与 `scheduler/`
- 目标多端适配（Windows / macOS / Linux）。平台专属代码必须用 `#[cfg(...)]` 隔离，禁止调用未隔离的平台 API；新增功能需考虑跨平台
- 数据目录：Windows 为 `%APPDATA%\Toolbox`；macOS / Linux 为系统 data_dir 下的 `Toolbox`。跨平台代码以 `AppPaths::resolve` 中的 `app.path().data_dir()?.join("Toolbox")` 为准，禁止硬编码平台路径

## 常用命令

```bash
npm run tauri:dev      # 开发调试
npm run typecheck      # vue-tsc 前端类型检查
npm run test           # 前端单测（node --test tests/）
cargo test --manifest-path src-tauri/Cargo.toml   # Rust 测试
npm run tauri:build    # release + NSIS；便携版再跑 npm run package:portable
node scripts/check-version.mjs       # 三处版本号与 tag 一致性
node scripts/check-file-length.mjs   # 单文件 400 行基线（--update 更新基线）
```

改动后至少运行 `npm run typecheck` 与相关测试。

## 硬性约束

- 优先使用中文：交流、代码注释、用户可见文案均使用中文。
- 未经用户明确要求，不要执行全局 lint / 格式化命令。
- 单文件不超过 400 行，超过必须拆分为多个模块。
- 工具安装（yt-dlp、ffmpeg）必须做 SHA256 校验，采用 fail-closed：校验文件不可达、格式异常或哈希不匹配时终止安装、删除临时文件、保留旧版本，禁止静默安装未通过校验的二进制。
- 哈希计算必须流式进行，避免大文件全量载入内存。
- 命令执行默认关闭，需在设置中显式开启；不提权、不监听端口。
- yt-dlp / ffmpeg 不随安装包分发，按需下载到数据目录 `bin/`。

## 提交规范

格式：`<类型>: <中文描述>`，严格执行，如：

```
fix: 修复登录验证逻辑
feature: 添加用户权限管理接口
optimize: 优化大数据查询性能
```

类型：`fix` / `feature` / `optimize` / `refactor` / `test` / `docs`。

重构按独立可回滚的粒度拆分提交；纯接口抽取与耐久性（文件格式、迁移）变更分离提交。

## 发版流程

- 发版步骤以 `docs/RELEASE_CHECKLIST.md` 为准（构建 → 安装冒烟 → push → CI → tag → draft → 填写描述 → 发布）。
- 一个版本对应一个 tag，通过 GitHub Actions 发版：推送新 tag 即触发。若 `.github/workflows` 中的发版 workflow 不存在，发版前需创建（tag 触发、多平台构建）。
- 推送 tag 前必须先在本地验证构建通过（`npm run tauri:build`），未通过不得推送发版 tag。
- Release 描述必须分节写清：**新增**什么、**改动**什么、**优化**什么；大版本更新必须说明是否有兼容性问题（配置迁移、数据格式、命令行参数等）。

## 代码约定

- 前端通过 `src/api.ts` 调用 Tauri command，不直接访问后端内部状态。
<!-- BEGIN AGENTHUB -->
# AgentHub Managed Rules (toolbox)

- Hub: `personal`
- Profile: `personal-rust-tauri`

## [profile] 10-personal-engineering.md (personal:personal-base)

# 10 — 个人工程化与协作原则

本规范仅约束个人自研项目（不自动套用于公司历史存量仓）。

## 1. 单文件体积极限与渐进式拆分
- **行数红线**：个人项目源码、规范文档与规则文件默认单文件严格控制在 **≤ 500 行**（若项目 Overlay 约定更严上限如 400/600 行，以 Overlay 为准）。
- **提前模块化解耦**：逻辑预计超过 400 行时先规划拆分再实现，通过组合模式提取子模块并保留统一导出面，不写完再补救。
- **渐进式披露与上下文经济性**：高层索引文档仅承载入口与核心契约，不向上下文倾倒全量实现细节。

## 2. 确定性逻辑交付脚本
- **机械归脚本，语义归 LLM**：格式校验、哈希比对、代码生成、行号/行数统计等确定性工作优先编写独立脚本（Python / Node.js）自动化执行，拒绝人工脑补计算。

## 3. 多 Agent 并行协作约束
- **互不重叠的文件所有权**：并行委派多个子代理时，必须为每个 Agent 划定显式、互不重叠的文件所有权清单，严禁两个 Agent 同时修改同一个文件。

## [profile] rust-tauri-desktop.md (personal:personal-rust-tauri)

# Rust / Tauri 2 桌面端工程规范

适用于基于 Tauri 2 + Rust + TypeScript 前端的跨平台桌面客户端项目。

## 1. 跨进程安全边界（Command Seam）
- **受限 Command 契约**：前后端通信仅通过显式声明的 Tauri Command（明确输入 DTO 与 `Result<Dto, UserVisibleError>`），严禁向前端暴露通用 Shell 执行或任意文件路径写入通道。
- **数据目录解析**：应用数据路径统一通过 Tauri 官方 `data_dir` API 解析，严禁硬编码宿主平台路径；平台差异代码须用 `#[cfg(...)]` 隔离。

## 2. 持久化写入与 Windows 文件锁防坑
- **备份 + 原子写 + 回滚**：写入正式配置或状态文件前须先创建备份，使用「临时文件 + 原子替换」原语；多文件关联更新任一失败须逐文件回滚，严禁裸写正式文件。
- **规避运行中 `.exe` 编译锁**：Windows 下运行中的桌面进程会锁定可执行文件；排查问题或跑单测时优先只编译库目标（library-only），或先终止驻留进程。

## 3. 体积控制与纯结构拆分
- **提前规划模块粒度**：新文件按 ≤ 400 行规划（硬上限依项目 Overlay 约定），Rust 按职责拆分 `mod`，前端按主组件 + 子组件 + hooks + types 拆分。
- **零行为变更拆分**：拆分超限文件时仅做纯结构迁移与 `re-export`，不顺手修改运行时逻辑与公共契约。

## 4. 版本一致性与 UI 发布验证
- **三处版本号一致**：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 中的版本号必须保持严格一致。
- **UI 与 CSP 实测验收**：UI/视觉改动须实际启动应用核验渲染结果，不以单测或类型检查代替视觉验证；涉及窗口生命周期、终端或 CSP 策略的改动须构建 release 并直接启动 GUI 实测。

## 5. 安全与容错底线
- **外置二进制校验**：按需下载的可执行依赖须执行流式 SHA-256 校验与 fail-closed 处理，校验失败立即中止并清理临时文件。
- **诊断与导出脱敏**：导出配置、备份包与运行日志必须自动剥离 Token、密码及 URL 查询凭据。

## [overlay] overlay.md (personal:overlays/toolbox)

# toolbox 项目事实覆盖

- 项目定位：个人桌面工具箱（Vue 3 + TypeScript + Naive UI + Vite + Tauri 2 + Rust），提供翻译、m3u8 下载、定时任务与托盘常驻能力。
- 核心约束与红线：
  - 单文件硬上限 **400 行**，超限须当场拆分。
  - 数据目录统一经 `AppPaths::resolve`（`app.path().data_dir()?.join("Toolbox")`）解析；前端仅通过 `src/api.ts` 调用 Tauri command。
  - `yt-dlp` / `ffmpeg` 按需下载至数据目录 `bin/`，必须执行流式 SHA-256 + fail-closed 校验；命令执行默认关闭，不提权、不监听端口。
- 提交与发版：提交格式 `<类型>: <中文描述>`（`fix/feature/optimize/refactor/test/docs`）；发版遵循 `docs/RELEASE_CHECKLIST.md`，推 tag 前本地跑通 `npm run tauri:build`。
- 常用验证：`npm run typecheck`、`npm run test`、`cargo test --manifest-path src-tauri/Cargo.toml`、`node scripts/check-version.mjs`、`node scripts/check-file-length.mjs`。
<!-- END AGENTHUB -->
