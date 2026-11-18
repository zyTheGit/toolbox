import { ref, type Ref } from "vue";
import { useDialog, useMessage } from "naive-ui";

import { api } from "../api";
import { store } from "../store";
import { cloneLlmConfig } from "../utils/llm";
import { describeError } from "../utils/errors";
import { saveConfigWithConflict } from "../utils/configConflict";
import type { AppConfig, LlmConfig, ConfigResult } from "../types";

/** AI 页只管自己的 llm 段：冲突时按段合并，不整包覆盖其他设置。 */
export function useLlmConfigSave(draft: Ref<LlmConfig>) {
  const message = useMessage();
  const dialog = useDialog();
  const saving = ref(false);

  function applyResult(result: ConfigResult): void {
    store.configRevision = result.configRevision;
    store.config = result.config;
    store.tools = result.tools;
    store.providers = result.providers;
    store.secrets = result.secrets;
    draft.value = cloneLlmConfig(result.config.llm);
  }

  async function save(): Promise<void> {
    if (!store.config || saving.value) return;
    const payload = JSON.parse(JSON.stringify(store.config)) as AppConfig;
    payload.llm = JSON.parse(JSON.stringify(draft.value));
    saving.value = true;
    try {
      const result = await saveConfigWithConflict(
        dialog,
        (base) => api.saveConfig(payload, null, "llm", base),
        store.configRevision,
        (conflict) => {
          store.config = conflict.config;
          store.configRevision = conflict.configRevision;
          draft.value = cloneLlmConfig(conflict.config.llm);
          message.info("已载入磁盘上的最新 AI 配置");
        },
      );
      if (!result) return;
      applyResult(result);
      message.success("AI 配置已保存");
    } catch (err) {
      message.error(`保存失败：${describeError(err)}`);
    } finally {
      saving.value = false;
    }
  }

  return { saving, save };
}
