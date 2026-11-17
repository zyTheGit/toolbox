#!/usr/bin/env node
// 打包便携版：复制 release 可执行文件 + 说明文件，压缩为 zip。
// 仅使用 Node 内置模块，不触发任何构建。

import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");
const isWindows = process.platform === "win32";

function fail(message) {
  console.error(`\n[失败] ${message}\n`);
  process.exit(1);
}

let version;
try {
  const pkg = JSON.parse(readFileSync(join(rootDir, "package.json"), "utf8"));
  version = pkg.version;
} catch (err) {
  fail(`无法读取 package.json：${err instanceof Error ? err.message : String(err)}`);
}
if (!version) fail("package.json 中缺少 version 字段");

const releaseDir = join(rootDir, "src-tauri", "target", "release");
const candidates = isWindows
  ? ["toolbox.exe", "Toolbox.exe"]
  : ["toolbox", "Toolbox"];
const sourcePath = candidates
  .map((name) => join(releaseDir, name))
  .find((path) => existsSync(path));

if (!sourcePath) {
  fail(
    [
      "未找到已构建的可执行文件。",
      `查找位置：${releaseDir}`,
      `查找文件名：${candidates.join(" / ")}`,
      "请先运行 npm run tauri:build，再执行 npm run package:portable。",
    ].join("\n"),
  );
}

const portableDir = join(rootDir, "portable");
mkdirSync(portableDir, { recursive: true });

const exeName = isWindows ? `Toolbox-${version}.exe` : `Toolbox-${version}`;
const exePath = join(portableDir, exeName);
copyFileSync(sourcePath, exePath);
if (!isWindows) chmodSync(exePath, 0o755);
console.log(`已复制可执行文件：${exePath}`);

const notePath = join(portableDir, "便携版说明.txt");
const note = [
  `Toolbox 便携版 ${version}`,
  "",
  `1. 双击 ${exeName} 即可运行，无需安装。`,
  "2. 用户数据保存在 %APPDATA%\\Toolbox，卸载或移动程序不会删除这些数据。",
  "3. yt-dlp 与 ffmpeg 不随包分发，需要时由程序按需下载到 %APPDATA%\\Toolbox\\bin。",
  "4. 关闭主窗口不会退出程序，而是隐藏到系统托盘；请从托盘菜单选择「退出」结束进程。",
  "5. 开机自启的启动参数为 --autostart，可在程序设置中开启或关闭。",
  "6. 需要 Microsoft Edge WebView2 运行时：Windows 11 自带，Windows 10 如缺失请先安装。",
  "",
].join("\r\n");
writeFileSync(notePath, note, "utf8");
console.log(`已写入说明文件：${notePath}`);

const zipPath = join(portableDir, `Toolbox-${version}.zip`);
rmSync(zipPath, { force: true });

if (isWindows) {
  const quote = (value) => `'${value.replace(/'/g, "''")}'`;
  const command = [
    "Compress-Archive",
    "-LiteralPath",
    `${quote(exePath)},${quote(notePath)}`,
    "-DestinationPath",
    quote(zipPath),
    "-Force",
  ].join(" ");

  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-Command",
      command,
    ],
    { encoding: "utf8", windowsHide: true },
  );

  if (result.error || result.status !== 0 || !existsSync(zipPath)) {
    const detail = [
      result.error ? result.error.message : "",
      result.stderr ? result.stderr.trim() : "",
      result.stdout ? result.stdout.trim() : "",
    ]
      .filter(Boolean)
      .join("\n");
    fail(`PowerShell 压缩失败。${detail ? `\n${detail}` : ""}`);
  }
} else {
  const result = spawnSync("zip", ["-j", "-q", zipPath, exePath, notePath], {
    encoding: "utf8",
  });

  if (result.error || result.status !== 0 || !existsSync(zipPath)) {
    const detail = [
      result.error ? result.error.message : "",
      result.stderr ? result.stderr.trim() : "",
    ]
      .filter(Boolean)
      .join("\n");
    console.warn(
      [
        "",
        "[提示] 未能创建 zip（通常是没有安装 zip 命令）。",
        `便携版文件已保留在：${portableDir}`,
        detail ? `原因：${detail}` : "",
        "",
      ]
        .filter(Boolean)
        .join("\n"),
    );
    process.exit(0);
  }
}

const sizeMb = (statSync(zipPath).size / (1024 * 1024)).toFixed(2);
console.log(`\n已生成便携版：${zipPath}`);
console.log(`大小：${sizeMb} MB`);
