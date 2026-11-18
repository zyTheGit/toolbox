import { ref } from "vue";
import { api } from "../../api";
import type { Job, JobRetryRequest } from "../../types";
import { describeError } from "../../utils/errors";
import {
  isCredentialsError,
  prefilledCredentials,
  rememberCredentials,
  type CredentialDraft,
} from "./sessionCredentials";

interface Notifier {
  success: (text: string) => void;
  error: (text: string) => void;
}

/**
 * 下载任务重试：遇到 `[CREDENTIALS]` 时打开重填对话框。
 *
 * 凭据只保存在本会话的内存凭据表中（后端进程内），前端仅预填最近一次输入。
 */
export function useJobRetry(job: () => Job, notify: Notifier) {
  const busy = ref(false);
  const showCredentials = ref(false);
  const submitting = ref(false);
  const error = ref("");
  const initial = ref<CredentialDraft>(prefilledCredentials());

  function errorText(err: unknown): string {
    return describeError(err);
  }

  function openCredentials(): void {
    initial.value = prefilledCredentials();
    error.value = "";
    showCredentials.value = true;
  }

  /** 直接重试；内存中仍有凭据时会一次成功，不会弹窗。 */
  async function retry(): Promise<void> {
    if (busy.value) return;
    busy.value = true;
    try {
      await api.jobRetry(job().id);
      notify.success("已重新加入队列");
    } catch (err) {
      if (isCredentialsError(err)) {
        openCredentials();
      } else {
        notify.error(errorText(err));
      }
    } finally {
      busy.value = false;
    }
  }

  /** 在对话框中补填凭据后重试。 */
  async function submitCredentials(draft: CredentialDraft): Promise<void> {
    if (submitting.value) return;
    submitting.value = true;
    error.value = "";
    const request: JobRetryRequest = {
      referer: draft.referer.trim(),
      cookiesBrowser: draft.cookiesBrowser,
      proxy: draft.proxy.trim(),
      extraArgs: draft.extraArgs.trim(),
    };
    try {
      await api.jobRetry(job().id, request);
      rememberCredentials(draft);
      showCredentials.value = false;
      notify.success("已重新加入队列");
    } catch (err) {
      if (isCredentialsError(err)) {
        error.value = "凭据仍不完整，请至少填写一项后重试";
      } else {
        error.value = errorText(err);
        notify.error(errorText(err));
      }
    } finally {
      submitting.value = false;
    }
  }

  return {
    busy,
    showCredentials,
    submitting,
    error,
    initial,
    retry,
    submitCredentials,
  };
}
