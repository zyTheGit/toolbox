import { computed, ref } from "vue";

import { store } from "../store";
import { cloneLlmConfig } from "../utils/llm";
import type { LlmConfig } from "../types";

/**
 * AI 页共享的 LLM 草稿：页头的保存状态与调试台、模型配置面板共用同一份数据，
 * 避免出现「页头显示已保存、面板里却有另一份未保存副本」的两套状态。
 */
export const llmDraft = ref<LlmConfig>(cloneLlmConfig(store.config?.llm));

/** 草稿与磁盘配置是否一致，供页头展示保存状态。 */
export const llmDirty = computed(
  () => JSON.stringify(llmDraft.value) !== JSON.stringify(store.config?.llm ?? null),
);
