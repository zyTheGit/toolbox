import assert from "node:assert/strict";
import test from "node:test";
import { extractUrls } from "../src/utils/download-links.ts";

test("识别裸链接与常见分享文本", () => {
  assert.deepEqual(extractUrls("https://youtu.be/abc?si=1"), ["https://youtu.be/abc?si=1"]);
  assert.deepEqual(extractUrls("【测试视频】 https://b23.tv/AbCdE，快来围观"), [
    "https://b23.tv/AbCdE",
  ]);
  assert.deepEqual(extractUrls("分享给你 https://x.com/foo/status/123?s=20 看看"), [
    "https://x.com/foo/status/123?s=20",
  ]);
});

test("一行包含多个链接时全部提取", () => {
  assert.deepEqual(
    extractUrls("https://youtu.be/a 和 https://www.bilibili.com/video/BV1xx"),
    ["https://youtu.be/a", "https://www.bilibili.com/video/BV1xx"],
  );
});

test("忽略无链接与非 http(s) 文本", () => {
  assert.deepEqual(extractUrls("随便一段文字"), []);
  assert.deepEqual(extractUrls("ftp://example.com/a"), []);
});

test("去掉尾随中英文标点", () => {
  assert.deepEqual(extractUrls("https://youtu.be/a。"), ["https://youtu.be/a"]);
  assert.deepEqual(extractUrls("（https://youtu.be/a）"), ["https://youtu.be/a"]);
});
