#!/usr/bin/env node
// 版本一致性检查：
// 1. package.json、src-tauri/tauri.conf.json、src-tauri/Cargo.toml 的 version 必须一致；
// 2. 当环境变量 GITHUB_REF 为 refs/tags/v* 时，去掉前缀 v 后必须与三处版本一致。
// 只读脚本，不做任何写操作。

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");

const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;

function fail(message) {
  console.error(`\n[失败] ${message}\n`);
  process.exit(1);
}

function readText(relativePath) {
  const absolutePath = join(rootDir, relativePath);
  try {
    return readFileSync(absolutePath, "utf8");
  } catch (err) {
    fail(
      `无法读取 ${relativePath}：${err instanceof Error ? err.message : String(err)}`,
    );
  }
}

function readJsonVersion(relativePath) {
  const text = readText(relativePath);
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch (err) {
    fail(
      `${relativePath} 不是合法的 JSON：${err instanceof Error ? err.message : String(err)}`,
    );
  }
  const version = parsed?.version;
  if (typeof version !== "string" || version.trim() === "") {
    fail(`${relativePath} 中缺少 version 字段`);
  }
  return version.trim();
}

/// 只解析 Cargo.toml 的 [package] 段，避免误取依赖版本。
function readCargoVersion() {
  const relativePath = "src-tauri/Cargo.toml";
  const text = readText(relativePath);
  let section = "";
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    const sectionMatch = line.match(/^\[([^\]]+)\]$/);
    if (sectionMatch) {
      section = sectionMatch[1].trim();
      continue;
    }
    if (section !== "package") continue;
    const versionMatch = line.match(/^version\s*=\s*"([^"]+)"/);
    if (versionMatch) return versionMatch[1].trim();
  }
  fail(`${relativePath} 的 [package] 段中缺少 version 字段`);
}

const sources = [
  { path: "package.json", version: readJsonVersion("package.json") },
  {
    path: "src-tauri/tauri.conf.json",
    version: readJsonVersion("src-tauri/tauri.conf.json"),
  },
  { path: "src-tauri/Cargo.toml", version: readCargoVersion() },
];

for (const item of sources) {
  if (!VERSION_PATTERN.test(item.version)) {
    fail(`${item.path} 的版本号 "${item.version}" 不是合法的语义化版本`);
  }
}

const reference = sources[0];
const mismatched = sources.filter((item) => item.version !== reference.version);
if (mismatched.length > 0) {
  const detail = sources
    .map((item) => `  - ${item.path}: ${item.version}`)
    .join("\n");
  fail(`三处版本号不一致，请统一后再提交：\n${detail}`);
}

const version = reference.version;
console.log(`[版本检查] 三处版本号一致：${version}`);

const githubRef = (process.env.GITHUB_REF ?? "").trim();
if (githubRef.startsWith("refs/tags/v")) {
  const tagVersion = githubRef.slice("refs/tags/v".length);
  if (!tagVersion) {
    fail(`GITHUB_REF 为 "${githubRef}"，但无法解析出 tag 版本号`);
  }
  if (tagVersion !== version) {
    fail(
      `tag 版本与项目版本不一致：tag 为 v${tagVersion}，` +
        `package.json / tauri.conf.json / Cargo.toml 为 ${version}`,
    );
  }
  console.log(`[版本检查] tag 一致：v${tagVersion}`);
} else {
  console.log("[版本检查] 未检测到 refs/tags/v* 形式的 GITHUB_REF，跳过 tag 比对");
}

console.log("[版本检查] 通过");
