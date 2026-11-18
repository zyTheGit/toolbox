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
