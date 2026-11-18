import { h } from "vue";
import type { DialogApi } from "naive-ui";
import type { AppConfig } from "../types";

/** 后端分段保存返回的结构化冲突；携带磁盘上的最新配置。 */
export interface ConfigConflictError {
  code: "CONFLICT";
  message: string;
  config: AppConfig;
  configRevision: number;
}

export function configConflictOf(err: unknown): ConfigConflictError | null {
  if (!err || typeof err !== "object") return null;
  const value = err as Partial<ConfigConflictError>;
  if (value.code !== "CONFLICT") return null;
  if (!value.config || typeof value.configRevision !== "number") return null;
  return {
    code: "CONFLICT",
    message: value.message ?? "配置已被其他页面修改。",
    config: value.config,
    configRevision: value.configRevision,
  };
}

export type ConflictChoice = "keep" | "disk" | "cancel";

/**
 * 冲突只允许二选一：保留我的 = 用最新版本重试；用磁盘的 = 丢弃当前草稿。
 * 关闭对话框视为取消，不丢编辑。
 */
export function askConfigConflict(
  dialog: DialogApi,
  conflict: ConfigConflictError,
  useDisk: () => void,
): Promise<ConflictChoice> {
  return new Promise((resolve) => {
    let settled = false;
    const finish = (choice: ConflictChoice) => {
      if (settled) return;
      settled = true;
      resolve(choice);
    };
    dialog.warning({
      title: "配置已被其他页面修改",
      content: () =>
        h("div", { style: "line-height: 1.8" }, [
          h("div", conflict.message),
          h("div", "「用磁盘的」会丢弃本页未保存的修改；「保留我的」会把本页修改合并到最新配置后重试。"),
        ]),
      positiveText: "用磁盘的",
      negativeText: "保留我的",
      onPositiveClick: () => {
        useDisk();
        finish("disk");
      },
      onNegativeClick: () => finish("keep"),
      onClose: () => finish("cancel"),
      onMaskClick: () => finish("cancel"),
    });
  });
}

/** 冲突时二选一，重试沿用同一份提交内容；返回 null 表示用户改用磁盘或取消。 */
export async function saveConfigWithConflict<T>(
  dialog: DialogApi,
  save: (baseRevision: number) => Promise<T>,
  baseRevision: number,
  useDisk: (conflict: ConfigConflictError) => void,
): Promise<T | null> {
  let base = baseRevision;
  for (let attempt = 0; attempt < 4; attempt += 1) {
    try {
      return await save(base);
    } catch (err) {
      const conflict = configConflictOf(err);
      if (!conflict) throw err;
      const choice = await askConfigConflict(dialog, conflict, () => useDisk(conflict));
      if (choice !== "keep") return null;
      base = conflict.configRevision;
    }
  }
  throw new Error("[CONFLICT] 配置持续被其他页面修改，请稍后重试");
}
