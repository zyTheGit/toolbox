import assert from "node:assert/strict";
import test from "node:test";
import { CRON_PRESETS, cronSummary, parseWeekdays } from "../src/utils/cron.ts";

test("parseWeekdays 支持列举与范围", () => {
  assert.deepEqual(parseWeekdays("1,3,5"), [1, 3, 5]);
  assert.deepEqual(parseWeekdays("1-5"), [1, 2, 3, 4, 5]);
  assert.deepEqual(parseWeekdays("7"), [0]);
  assert.deepEqual(parseWeekdays("5-7"), [0, 5, 6]);
});

test("parseWeekdays 拒绝非法写法", () => {
  assert.equal(parseWeekdays("9"), null);
  assert.equal(parseWeekdays("3-1"), null);
  assert.equal(parseWeekdays("1-9"), null);
  assert.equal(parseWeekdays("mon"), null);
  assert.equal(parseWeekdays(""), null);
});

test("cronSummary 覆盖常用表达式", () => {
  assert.equal(cronSummary("* * * * *"), "每分钟");
  assert.equal(cronSummary("*/15 * * * *"), "每 15 分钟");
  assert.equal(cronSummary("0 * * * *"), "每小时整点");
  assert.equal(cronSummary("30 8 * * *"), "每天 08:30");
  assert.equal(cronSummary("0 9 * * 1"), "每周一 09:00");
  assert.equal(cronSummary("0 9 * * 1-5"), "工作日 09:00");
  assert.equal(cronSummary("0 9 * * 6,0"), "每周日、周六 09:00");
  assert.equal(cronSummary("0 3 1 * *"), "每月 1 日 03:00");
});

test("cronSummary 对无法识别的表达式返回 null", () => {
  assert.equal(cronSummary("0 9 * * mon"), null);
  assert.equal(cronSummary("0 9 * *"), null);
  assert.equal(cronSummary(""), null);
});

test("预设表达式都能被 cronSummary 解释", () => {
  for (const item of CRON_PRESETS) {
    assert.notEqual(cronSummary(item.value), null, `预设「${item.label}」缺少解释`);
  }
});
