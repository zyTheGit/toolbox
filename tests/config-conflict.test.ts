import { test } from "node:test";
import assert from "node:assert/strict";

import {
  configConflictOf,
  saveConfigWithConflict,
  type ConfigConflictError,
} from "../src/utils/configConflict.ts";

const conflict: ConfigConflictError = {
  code: "CONFLICT",
  message: "配置已被其他页面修改",
  config: { schemaVersion: 1 } as ConfigConflictError["config"],
  configRevision: 7,
};

function fakeDialog(choose: "keep" | "disk" | "cancel") {
  let options: Record<string, unknown> = {};
  return {
    api: {
      warning(next: Record<string, unknown>) {
        options = next;
        queueMicrotask(() => {
          if (choose === "keep") (options.onNegativeClick as () => void)();
          else if (choose === "disk") (options.onPositiveClick as () => void)();
          else (options.onClose as () => void)();
        });
      },
    },
    get options() {
      return options;
    },
  };
}

test("configConflictOf 只识别带最新配置的 CONFLICT", () => {
  assert.equal(configConflictOf(conflict)?.configRevision, 7);
  assert.equal(configConflictOf(new Error("[CONFLICT] 配置冲突")), null);
  assert.equal(configConflictOf({ code: "CONFLICT" }), null);
});

test("保留我的会用最新 revision 重试", async () => {
  const dialog = fakeDialog("keep");
  const seen: number[] = [];
  const result = await saveConfigWithConflict(
    dialog.api as never,
    async (base) => {
      seen.push(base);
      if (seen.length === 1) throw conflict;
      return "saved";
    },
    3,
    () => assert.fail("不应使用磁盘配置"),
  );
  assert.equal(result, "saved");
  assert.deepEqual(seen, [3, 7]);
});

test("用磁盘的会丢弃草稿并返回 null", async () => {
  const dialog = fakeDialog("disk");
  let applied = 0;
  const result = await saveConfigWithConflict(
    dialog.api as never,
    async () => {
      throw conflict;
    },
    3,
    () => {
      applied += 1;
    },
  );
  assert.equal(result, null);
  assert.equal(applied, 1);
});

test("关闭冲突对话框按取消处理，不覆盖也不丢草稿", async () => {
  const dialog = fakeDialog("cancel");
  const result = await saveConfigWithConflict(
    dialog.api as never,
    async () => {
      throw conflict;
    },
    3,
    () => assert.fail("取消时不应使用磁盘配置"),
  );
  assert.equal(result, null);
});

test("连续冲突超过上限会明确报错而不是静默覆盖", async () => {
  const dialog = fakeDialog("keep");
  await assert.rejects(
    saveConfigWithConflict(
      dialog.api as never,
      async () => {
        throw conflict;
      },
      0,
      () => undefined,
    ),
    /配置持续被其他页面修改/,
  );
});
