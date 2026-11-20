import { test } from "node:test";
import assert from "node:assert/strict";

import { copyTaskName } from "../src/utils/taskName.ts";

test("复制任务名称追加副本后缀", () => {
  assert.equal(copyTaskName("每日备份"), "每日备份（副本）");
});

test("名称为空或空白时使用未命名任务", () => {
  assert.equal(copyTaskName(""), "未命名任务（副本）");
  assert.equal(copyTaskName("   "), "未命名任务（副本）");
});

test("名称首尾空白会被去掉再追加后缀", () => {
  assert.equal(copyTaskName("  打卡提醒  "), "打卡提醒（副本）");
});
