import assert from "node:assert/strict";
import test from "node:test";
import { createDbSeq } from "../src/utils/dbSeq.ts";

test("begin 按通道递增序号", () => {
  const seq = createDbSeq();
  assert.equal(seq.begin("query"), 1);
  assert.equal(seq.begin("query"), 2);
  assert.equal(seq.begin("open"), 1);
  assert.equal(seq.begin("selectDatabase"), 1);
  assert.equal(seq.begin("close"), 1);
});

test("同一通道后续请求会让先发起的响应过期", () => {
  const seq = createDbSeq();
  const firstRun = seq.begin("query");
  const secondRun = seq.begin("query");
  assert.equal(seq.isLatest("query", firstRun), false);
  assert.equal(seq.isLatest("query", secondRun), true);
});

test("不同通道的序号互不影响", () => {
  const seq = createDbSeq();
  const openToken = seq.begin("open");
  const queryToken = seq.begin("query");
  // 切库不会让打开的响应失效
  seq.begin("selectDatabase");
  assert.equal(seq.isLatest("open", openToken), true);
  assert.equal(seq.isLatest("query", queryToken), true);
});

test("invalidate 作废未返回的请求", () => {
  const seq = createDbSeq();
  const token = seq.begin("query");
  seq.invalidate("query");
  assert.equal(seq.isLatest("query", token), false);
  // 作废后新发起的请求重新是最新
  const next = seq.begin("query");
  assert.equal(seq.isLatest("query", next), true);
});

test("invalidateAll 作废所有通道，用于断连或页面卸载", () => {
  const seq = createDbSeq();
  const openToken = seq.begin("open");
  const queryToken = seq.begin("query");
  const switchToken = seq.begin("selectDatabase");
  seq.invalidateAll();
  assert.equal(seq.isLatest("open", openToken), false);
  assert.equal(seq.isLatest("query", queryToken), false);
  assert.equal(seq.isLatest("selectDatabase", switchToken), false);
});

test("模拟快速切表：只接受最后一次查询结果", () => {
  const seq = createDbSeq();
  const responses: string[] = [];
  const apply = (token: number, label: string): void => {
    if (seq.isLatest("query", token)) responses.push(label);
  };
  const slow = seq.begin("query");
  const fast = seq.begin("query");
  // 先返回慢请求的旧结果，再返回快请求的新结果
  apply(slow, "旧表");
  apply(fast, "新表");
  assert.deepEqual(responses, ["新表"]);
});
