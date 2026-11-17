import assert from "node:assert/strict";
import test from "node:test";
import { describeError, errorText, parseError } from "../src/utils/errors.ts";

test("parseError extracts the code and action", () => {
  const parsed = parseError("[TOOL] 未找到 yt-dlp");
  assert.equal(parsed.code, "TOOL");
  assert.equal(parsed.message, "未找到 yt-dlp");
  assert.match(parsed.action, /安装或更新工具/);
});

test("parseError leaves uncoded messages untouched", () => {
  const parsed = parseError("网络抖动");
  assert.equal(parsed.code, "");
  assert.equal(parsed.message, "网络抖动");
  assert.equal(parsed.action, "");
});

test("describeError appends the next step without leaking the tag", () => {
  const text = describeError("[CONFLICT] 任务正在运行");
  assert.match(text, /^任务正在运行/);
  assert.doesNotMatch(text, /CONFLICT/);
  assert.match(text, /刷新后重试/);
});

test("errorText accepts Error and unknown values", () => {
  assert.equal(errorText(new Error("boom")), "boom");
  assert.equal(errorText(undefined), "操作失败，请稍后重试");
});