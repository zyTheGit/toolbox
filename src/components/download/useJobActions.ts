import { computed, ref } from "vue";
import { useDialog, useMessage } from "naive-ui";
import { api } from "../../api";
import type { Job } from "../../types";
import { describeError } from "../../utils/errors";

type Action = "" | "cancel" | "open" | "remove";

/**
 * 任务行上的取消 / 打开目录 / 移除 / 重新下载确认。
 * 重新下载的实际请求交给 useJobRetry，以便共用凭据补填流程。
 */
export function useJobActions(
  job: () => Job,
  isRetrying: () => boolean,
  redownload: () => Promise<void>,
) {
  const message = useMessage();
  const dialog = useDialog();
  /** 当前进行中的本地操作，用于按钮 loading 与防重复。 */
  const action = ref<Action>("");
  const busy = computed(() => Boolean(action.value) || isRetrying());

  async function cancel(): Promise<void> {
    if (busy.value) return;
    action.value = "cancel";
    try {
      await api.jobCancel(job().id);
      message.success("已请求取消");
    } catch (err) {
      message.error(describeError(err));
    } finally {
      action.value = "";
    }
  }

  async function openDir(): Promise<void> {
    if (busy.value) return;
    action.value = "open";
    try {
      const path = await api.jobOpen(job().id);
      message.success(`已打开 ${path}`);
    } catch (err) {
      message.error(describeError(err));
    } finally {
      action.value = "";
    }
  }

  function remove(): void {
    if (busy.value) return;
    dialog.warning({
      title: "移除记录",
      content:
        "将删除该任务未完成的临时文件（.part / .ytdl），已完成的成品文件不会删除。",
      positiveText: "移除",
      negativeText: "取消",
      onPositiveClick: async () => {
        if (busy.value) return;
        action.value = "remove";
        try {
          const cleaned = await api.jobRemove(job().id);
          message.success(
            cleaned > 0 ? `已移除记录，清理临时文件 ${cleaned} 个` : "已移除记录",
          );
        } catch (err) {
          message.error(describeError(err));
        } finally {
          action.value = "";
        }
      },
    });
  }

  function requestRedownload(): void {
    if (busy.value) return;
    dialog.warning({
      title: "重新下载",
      content: "将删除未完成的临时文件并从零开始下载，已完成的成品文件不受影响。",
      positiveText: "重新下载",
      negativeText: "取消",
      onPositiveClick: async () => {
        await redownload();
      },
    });
  }

  return { action, busy, cancel, openDir, remove, requestRedownload };
}
