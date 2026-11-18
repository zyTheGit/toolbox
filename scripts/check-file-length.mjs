#!/usr/bin/env node
// 单文件长度基线检查（AGENTS.md：单文件不超过 400 行）。
//
// 规则：
// - 扫描 src/ 与 src-tauri/src/ 下的 .vue / .ts / .rs 文件；
// - 超过 400 行的存量文件进入基线（scripts/file-length-baseline.json），只允许变小；
// - 新增超过 400 行的文件直接失败；
// - 默认只检查；`--update` 按当前状态重写基线（人工确认后再使用）。

import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const LIMIT = 400;
const EXTENSIONS = new Set([".vue", ".ts", ".rs"]);
const SCAN_DIRS = ["src", "src-tauri/src"];

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");
const baselinePath = join(scriptDir, "file-length-baseline.json");

function toPosix(path) {
  return path.split("\\").join("/");
}

function fail(message) {
  console.error(`\n[失败] ${message}\n`);
  process.exit(1);
}

function walk(absoluteDir, collected) {
  for (const entry of readdirSync(absoluteDir, { withFileTypes: true })) {
    const absolutePath = join(absoluteDir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "target") continue;
      walk(absolutePath, collected);
      continue;
    }
    if (!entry.isFile()) continue;
    const ext = entry.name.slice(entry.name.lastIndexOf("."));
    if (!EXTENSIONS.has(ext)) continue;
    collected.push(absolutePath);
  }
}

/// 与 wc -l 口径保持一致：按换行计数，文件末尾无换行时也计入最后一行。
function countLines(absolutePath) {
  const text = readFileSync(absolutePath, "utf8").replace(/\r\n/g, "\n");
  if (text === "") return 0;
  const lines = text.split("\n");
  if (text.endsWith("\n")) lines.pop();
  return lines.length;
}

function readBaseline() {
  if (!existsSync(baselinePath)) {
    return { files: {} };
  }
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(baselinePath, "utf8"));
  } catch (err) {
    fail(
      `无法解析基线文件 ${toPosix(relative(rootDir, baselinePath))}：` +
        `${err instanceof Error ? err.message : String(err)}`,
    );
  }
  const files = parsed?.files;
  if (!files || typeof files !== "object" || Array.isArray(files)) {
    fail("基线文件缺少合法的 files 对象");
  }
  const normalized = {};
  for (const [path, lines] of Object.entries(files)) {
    if (typeof lines !== "number" || !Number.isFinite(lines) || lines <= LIMIT) {
      fail(`基线文件中 ${path} 的行数无效（必须为大于 ${LIMIT} 的数字）`);
    }
    normalized[toPosix(path)] = lines;
  }
  return { files: normalized };
}

function writeBaseline(overLimit) {
  const files = {};
  for (const item of overLimit.slice().sort((a, b) => a.path.localeCompare(b.path))) {
    files[item.path] = item.lines;
  }
  const payload = {
    limit: LIMIT,
    updatedAt: new Date().toISOString().slice(0, 10),
    note: "存量超限文件基线，只允许变小；新增超限文件直接失败。由 scripts/check-file-length.mjs --update 生成。",
    files,
  };
  writeFileSync(baselinePath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
  console.log(
    `[文件长度] 已更新基线：${toPosix(relative(rootDir, baselinePath))}（${overLimit.length} 个文件）`,
  );
}

function main() {
  const args = process.argv.slice(2);
  const unknown = args.filter((arg) => arg !== "--update" && arg !== "-h" && arg !== "--help");
  if (args.includes("-h") || args.includes("--help")) {
    console.log(
      [
        "用法：node scripts/check-file-length.mjs [--update]",
        `  默认：检查 src/ 与 src-tauri/src/ 下 .vue/.ts/.rs 文件是否超过 ${LIMIT} 行`,
        "  --update：按当前实际行数重写 scripts/file-length-baseline.json",
      ].join("\n"),
    );
    return;
  }
  if (unknown.length > 0) {
    fail(`未知参数：${unknown.join(" ")}\n用法：node scripts/check-file-length.mjs [--update]`);
  }

  const absoluteFiles = [];
  for (const dir of SCAN_DIRS) {
    const absoluteDir = join(rootDir, dir);
    if (!existsSync(absoluteDir) || !statSync(absoluteDir).isDirectory()) {
      fail(`扫描目录不存在：${dir}`);
    }
    walk(absoluteDir, absoluteFiles);
  }

  const scanned = absoluteFiles
    .map((absolutePath) => ({
      path: toPosix(relative(rootDir, absolutePath)),
      lines: countLines(absolutePath),
    }))
    .sort((a, b) => a.path.localeCompare(b.path));

  const overLimit = scanned.filter((item) => item.lines > LIMIT);

  if (args.includes("--update")) {
    writeBaseline(overLimit);
    console.log(
      `[文件长度] 当前 ${overLimit.length} 个文件超过 ${LIMIT} 行，已写入基线；` +
        "基线只应随拆分而收窄。",
    );
    return;
  }

  const baseline = readBaseline();
  const baselineFiles = baseline.files;
  const problems = [];
  const notices = [];

  for (const item of overLimit) {
    const recorded = baselineFiles[item.path];
    if (recorded === undefined) {
      problems.push(
        `新增超限文件：${item.path}（${item.lines} 行，上限 ${LIMIT} 行）——必须拆分后再提交`,
      );
    } else if (item.lines > recorded) {
      problems.push(
        `基线文件变大：${item.path}（${recorded} → ${item.lines} 行）——` +
          "存量超限文件只允许变小，改动需抽段而不是继续追加",
      );
    } else if (item.lines < recorded) {
      notices.push(`${item.path} 已收窄 ${recorded} → ${item.lines} 行，可用 --update 更新基线`);
    }
  }

  const scannedPaths = new Set(scanned.map((item) => item.path));
  for (const path of Object.keys(baselineFiles)) {
    if (!scannedPaths.has(path)) {
      notices.push(`基线中的文件已不存在：${path}，可用 --update 移除`);
      continue;
    }
    const item = scanned.find((entry) => entry.path === path);
    if (item && item.lines <= LIMIT) {
      notices.push(`${path} 已回到 ${LIMIT} 行以内（${item.lines} 行），可用 --update 移出基线`);
    }
  }

  console.log(
    `[文件长度] 扫描 ${scanned.length} 个文件，上限 ${LIMIT} 行；` +
      `超限 ${overLimit.length} 个，基线 ${Object.keys(baselineFiles).length} 个`,
  );
  for (const notice of notices) {
    console.log(`[提示] ${notice}`);
  }
  if (problems.length > 0) {
    for (const problem of problems) {
      console.error(`- ${problem}`);
    }
    fail(`文件长度检查未通过（${problems.length} 项）`);
  }
  console.log("[文件长度] 通过");
}

main();
