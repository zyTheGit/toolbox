# Toolbox 发版检查清单

固化流程：**构建 → 安装冒烟 → push → CI → tag → draft → 填写描述 → 发布**。

> 原则：
> - 一个版本只对应一个 tag；tag 必须指向已验证的 HEAD。
> - Release 以草稿（`releaseDraft: true`）产出，必须人工复核后手动发布。
> - `push`、`tag`、发布 Release 属于共享状态操作，执行前必须得到用户明确确认。
> - 发版前必须用当前 HEAD 重新构建，禁止复用旧产物。

## 0. 发版前置检查

- [ ] 版本号已统一：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 三处一致。
- [ ] `node scripts/check-version.mjs` 通过。
- [ ] `node scripts/check-file-length.mjs` 通过；若有文件被拆分到 400 行以内，运行 `--update` 收窄基线并单独提交。
- [ ] `git status` 干净，HEAD 包含本版本全部修复；`git log` 与 Release 描述一致。
- [ ] R9 回归测试全绿：`npm run typecheck`、`npm run test`、`cargo test --manifest-path src-tauri/Cargo.toml`。
- [ ] `docs/remaining-work.md` 中的状态（推送、tag、产物时间）已更新为最新事实。

## 1. 构建

```bash
npm run typecheck
npm run test
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
node scripts/check-version.mjs
node scripts/check-file-length.mjs
npm run tauri:build
npm run package:portable      # Windows 便携版
```

- [ ] 记录 NSIS / dmg / deb / AppImage 与便携版 zip 的路径、大小、构建时间。
- [ ] 确认产物构建时间晚于当前 HEAD 提交时间；`portable/` 中的 zip 与本次构建的 exe 一致。
- [ ] 可选：`node scripts/sha256sums.mjs --out SHA256SUMS-local.txt` 记录本地哈希，供发布后比对。

## 2. 安装冒烟（Windows）

- [ ] 安装 NSIS 产物，逐页点检：翻译、下载、任务、设置、AI 工具、数据库、开发工具。
- [ ] 验证托盘菜单、关闭窗口隐藏、`--autostart` 自启开关。
- [ ] 验证工具状态检测、yt-dlp / ffmpeg 按需安装、下载目录切换。
- [ ] 验证备份导出 / 导入，导入后命令执行开关与命令任务状态符合预期。
- [ ] 验证快捷翻译：快捷键唤起窗口、剪贴板预览、不自动发起请求。
- [ ] 打开开发者工具，确认无 CSP 违规报错（本版本补充了 `object-src` / `base-uri` / `frame-ancestors` / `form-action`）。
- [ ] 直接运行便携版 zip 中的 exe，确认可启动且数据目录与安装版一致。

> macOS / Linux 无实机时，安装包与自启只能标记“未实机验证”，不得在描述中宣称已验证。

## 3. push

```bash
git push origin main
```

- [ ] push 前再次确认工作区干净、CI 配置与脚本已提交。

## 4. 等待 CI

- [ ] 新 run 的前端 job 与 Rust 三平台 job 全绿。
- [ ] 前端 job 实际执行了 `npm run build`、`node scripts/check-version.mjs`、`node scripts/check-file-length.mjs`。
- [ ] 若 Ubuntu 单测失败，先修复，禁止带红发布。

## 5. tag

```bash
git tag -a v0.1.0 -m "Toolbox 0.1.0"
git push origin v0.1.0
```

- [ ] tag 名与三处版本号一致（`v` + 版本号），`check-version.mjs` 会在 Release workflow 中再次校验。
- [ ] tag 指向已通过 CI 的 HEAD，禁止在未验证的提交上打 tag。

## 6. 等待草稿 Release

- [ ] Release workflow 四平台（Windows x64 / macOS arm64 / macOS x64 / Linux x64）全部成功。
- [ ] 草稿中包含：NSIS 安装包、dmg（两个架构）、deb、AppImage、Windows 便携版 zip。
- [ ] 草稿中包含每个平台的 `SHA256SUMS-<平台>.txt`。
- [ ] 核对产物时间与 tag 提交时间，确认不是旧产物。

## 7. 填写 Release 描述

按固定分节书写，参考模板：

```text
## 新增
- ...

## 改动
- ...

## 优化
- ...

## 兼容性
- 数据目录：Windows 为 %APPDATA%\Toolbox；macOS / Linux 以系统 data_dir 下的 Toolbox 为准
- 配置 / 备份格式版本与迁移说明
- 未实机验证项（macOS / Linux 安装与自启、LLM 实调、PostgreSQL / MySQL 实连等）
```

- [ ] 不夸大验证范围；未验证项必须写入“兼容性”或“已知问题”。
- [ ] 大版本更新必须说明配置迁移、数据格式、命令行参数是否兼容。

## 8. 发布

- [ ] 下载至少一个产物，用 `SHA256SUMS-<平台>.txt` 本地复核哈希。
- [ ] 确认草稿未被误设为正式发布、未被设为 prerelease（除非明确计划）。
- [ ] 人工点击 Publish release。
- [ ] 发布后再次确认 Release 页面产物可下载、描述分节完整、tag 链接正确。

## 附：常见失败处理

| 现象 | 处理 |
| --- | --- |
| 版本一致性检查失败 | 统一三处版本号后重新提交；已推 tag 时需删除远端 tag（须用户确认）并重新打 tag |
| 文件长度检查失败 | 新增超限文件必须拆分；存量超限文件只能变小，严禁 `--update` 放大基线 |
| 产物缺失 / 时间早于 HEAD | 重新构建，禁止发布旧产物 |
| CSP 导致页面空白或资源加载失败 | 回滚本次 CSP 新增指令并记录问题，禁止带病发布 |
| Release 缺少便携版 | 检查 Windows job 的 `npm run package:portable` 与上传步骤日志 |
| SHA256 校验文件缺失 | 检查 `scripts/sha256sums.mjs` 是否找到产物，以及 `gh release upload` 权限 |
