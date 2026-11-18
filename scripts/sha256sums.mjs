#!/usr/bin/env node
// 生成发版产物的 SHA256 校验文件（sha256sum 格式：<哈希>  <相对路径>）。
//
// 默认扫描：
// - src-tauri/target/**/release/bundle/ 下的安装包（exe / msi / dmg / deb / rpm / AppImage 等）
// - portable/ 下的便携版 zip
// 哈希按流式计算，不把整个文件读入内存。

import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");

const ARTIFACT_EXTENSIONS = [
  ".exe",
  ".msi",
  ".dmg",
  ".deb",
  ".rpm",
  ".appimage",
  ".tar.gz",
  ".zip",
  ".sig",
];

function toPosix(path) {
  return path.split("\\").join("/");
}

function fail(message) {
  console.error(`\n[失败] ${message}\n`);
  process.exit(1);
}

function isArtifact(path) {
  const lower = path.toLowerCase();
  return ARTIFACT_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

function walk(absoluteDir, collected) {
  let entries;
  try {
    entries = readdirSync(absoluteDir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const absolutePath = join(absoluteDir, entry.name);
    if (entry.isDirectory()) {
      walk(absolutePath, collected);
      continue;
    }
    if (entry.isFile()) collected.push(absolutePath);
  }
}

/// 收集 target 目录下 bundle 产物；macOS 的 .app 是目录，展开后只保留可校验文件会过于琐碎，
/// 因此统一只看 bundle 目录内的文件（如 dmg）。只进入 release/bundle 子目录，避免遍历整个 target。
function collectBundleArtifacts(collected) {
  const targetDir = join(rootDir, "src-tauri", "target");
  if (!statSyncSafe(targetDir)) return;

  const bundleDirs = [];
  const direct = join(targetDir, "release", "bundle");
  if (statSyncSafe(direct)?.isDirectory()) bundleDirs.push(direct);
  for (const entry of readdirSync(targetDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const candidate = join(targetDir, entry.name, "release", "bundle");
    if (statSyncSafe(candidate)?.isDirectory()) bundleDirs.push(candidate);
  }

  for (const bundleDir of bundleDirs) {
    const candidates = [];
    walk(bundleDir, candidates);
    for (const path of candidates) {
      if (isArtifact(path)) collected.push(path);
    }
  }
}

function statSyncSafe(path) {
  try {
    return statSync(path);
  } catch {
    return null;
  }
}

function collectPortableArtifacts(collected) {
  const portableDir = join(rootDir, "portable");
  if (!statSyncSafe(portableDir)) return;
  for (const entry of readdirSync(portableDir, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    if (!entry.name.toLowerCase().endsWith(".zip")) continue;
    collected.push(join(portableDir, entry.name));
  }
}

function sha256File(absolutePath) {
  return new Promise((resolvePromise, rejectPromise) => {
    const hash = createHash("sha256");
    const stream = createReadStream(absolutePath);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("error", rejectPromise);
    stream.on("end", () => resolvePromise(hash.digest("hex")));
  });
}

async function main() {
  const args = process.argv.slice(2);
  let outPath = null;
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--out") {
      outPath = args[index + 1];
      index += 1;
      if (!outPath) fail("--out 缺少文件路径");
      continue;
    }
    if (arg === "-h" || arg === "--help") {
      console.log("用法：node scripts/sha256sums.mjs [--out <输出文件>]");
      return;
    }
    fail(`未知参数：${arg}\n用法：node scripts/sha256sums.mjs [--out <输出文件>]`);
  }

  const artifacts = [];
  collectBundleArtifacts(artifacts);
  collectPortableArtifacts(artifacts);

  const unique = [...new Set(artifacts)]
    .map((path) => ({ absolute: path, relative: toPosix(relative(rootDir, path)) }))
    .sort((a, b) => a.relative.localeCompare(b.relative));

  if (unique.length === 0) {
    fail(
      "未找到任何可校验产物。请先执行 npm run tauri:build" +
        "（Windows 便携版还需 npm run package:portable），再生成校验文件。",
    );
  }

  const lines = [];
  for (const item of unique) {
    const hash = await sha256File(item.absolute);
    const sizeMb = (statSync(item.absolute).size / (1024 * 1024)).toFixed(2);
    lines.push(`${hash}  ${item.relative}`);
    console.log(`  ${hash}  ${item.relative}（${sizeMb} MB）`);
  }

  const content = `${lines.join("\n")}\n`;
  if (outPath) {
    writeFileSync(resolve(process.cwd(), outPath), content, "utf8");
    console.log(`[SHA256] 已写入：${outPath}（${unique.length} 个产物）`);
  } else {
    process.stdout.write(content);
  }
}

main().catch((err) => {
  fail(`生成 SHA256 校验文件失败：${err instanceof Error ? err.message : String(err)}`);
});
