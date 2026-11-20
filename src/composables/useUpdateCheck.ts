/** 检查新版本：状态共享给设置页关于卡片与启动时自动检查。 */

import { computed, reactive } from "vue";
import { api } from "../api";
import type { UpdateCheck } from "../types/update";
import { describeError } from "../utils/errors";

// 「跳过此版本」只记在本地：一次后台网络检查不该产生写配置文件的副作用。
const SKIPPED_KEY = "toolbox.update.skippedVersion";

function readSkipped(): string | null {
  try {
    return localStorage.getItem(SKIPPED_KEY);
  } catch {
    return null;
  }
}

const state = reactive({
  checking: false,
  result: null as UpdateCheck | null,
  error: "",
  skippedVersion: readSkipped(),
});

/** 有待更新且用户没有跳过该版本时才提示。 */
export const pendingUpdate = computed<UpdateCheck | null>(() => {
  const result = state.result;
  if (!result?.available) return null;
  return result.latestVersion === state.skippedVersion ? null : result;
});

/** 更新仍在线但被用户跳过了：设置页提供撤销入口。 */
export const showUndoSkip = computed(
  () => Boolean(state.skippedVersion) && Boolean(state.result?.available) && pendingUpdate.value === null,
);

/** 启动时自动检查一次；失败时保留上一次成功结果，已发现的更新不凭空消失。 */
export async function runUpdateCheck(): Promise<void> {
  if (state.checking) return;
  state.checking = true;
  state.error = "";
  try {
    state.result = await api.checkForUpdate();
  } catch (err) {
    state.error = describeError(err);
  } finally {
    state.checking = false;
  }
}

/** 打开最近一次检查拿到的 Release 页，由用户在浏览器里选择下载。 */
export async function openDownloadPage(): Promise<void> {
  const result = state.result;
  if (!result) return;
  state.error = "";
  try {
    await api.openReleasePage(result.pageUrl);
  } catch (err) {
    state.error = describeError(err);
  }
}

export function skipUpdateVersion(): void {
  const latest = state.result?.latestVersion;
  if (!latest || !state.result?.available) return;
  state.skippedVersion = latest;
  try {
    localStorage.setItem(SKIPPED_KEY, latest);
  } catch {
    // 隐私模式等场景下写不进去：本次会话内仍然生效。
  }
}

export function clearSkippedVersion(): void {
  state.skippedVersion = null;
  try {
    localStorage.removeItem(SKIPPED_KEY);
  } catch {
    // 同上，忽略存储不可用。
  }
}

export function useUpdateCheck() {
  return {
    state,
    pendingUpdate,
    showUndoSkip,
    runUpdateCheck,
    openDownloadPage,
    skipUpdateVersion,
    clearSkippedVersion,
  };
}
