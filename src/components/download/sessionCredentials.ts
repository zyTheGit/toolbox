import { reactive } from "vue";

/**
 * 本会话最近一次输入的下载凭据（仅内存，禁止 localStorage / 文件持久化）。
 *
 * 凭据不入 jobs.json：应用重启后内存清空，任务卡会提示“需重填凭据”。
 */
export interface CredentialDraft {
  referer: string;
  cookiesBrowser: string;
  proxy: string;
  extraArgs: string;
}

export const sessionCredentials = reactive<CredentialDraft>({
  referer: "",
  cookiesBrowser: "",
  proxy: "",
  extraArgs: "",
});

/** 记录本次提交的凭据，供重试对话框预填。 */
export function rememberCredentials(draft: CredentialDraft): void {
  sessionCredentials.referer = draft.referer.trim();
  sessionCredentials.cookiesBrowser = draft.cookiesBrowser;
  sessionCredentials.proxy = draft.proxy.trim();
  sessionCredentials.extraArgs = draft.extraArgs.trim();
}

/** 重试对话框的预填值（复制一份，避免编辑时污染会话记录）。 */
export function prefilledCredentials(): CredentialDraft {
  return {
    referer: sessionCredentials.referer,
    cookiesBrowser: sessionCredentials.cookiesBrowser,
    proxy: sessionCredentials.proxy,
    extraArgs: sessionCredentials.extraArgs,
  };
}

/** 后端用 `[CREDENTIALS]` 标记“内存凭据缺失、需要重填”。 */
export function isCredentialsError(err: unknown): boolean {
  const text =
    err instanceof Error ? err.message : typeof err === "string" ? err : "";
  return text.includes("[CREDENTIALS]");
}
