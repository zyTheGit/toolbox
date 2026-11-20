<template>
  <div class="bench">
    <LlmRequestCard
      :providers="draft.providers"
      :selected-ids="selectedIds"
      :ready-ids="readyIds"
      :system-prompt="systemPrompt"
      :prompt-text="promptText"
      :template-id="templateId"
      :template-options="templateOptions"
      :variables="variables"
      :var-values="varValues"
      :temperature="temperature"
      :max-tokens="maxTokens"
      :running="running"
      :has-results="results.length > 0"
      @update:selected-ids="selectedIds = $event"
      @update:system-prompt="systemPrompt = $event"
      @update:prompt-text="promptText = $event"
      @update:template-id="applyTemplate"
      @update:variable="(name, value) => (varValues[name] = value)"
      @update:temperature="temperature = $event"
      @update:max-tokens="maxTokens = $event"
      @run="run"
      @clear="clearResults"
    />

    <div class="side">
      <LlmResultsCard
        :results="results"
        :pending="pendingProviders"
        :meta="resultsMeta"
      />

      <LlmUsagePanel
        :usage="usage"
        :days="usageDays"
        :loading="usageLoading"
        @refresh="refreshUsage"
      />

      <details class="collapse">
        <summary><AppIcon name="chevron-right" :size="16" />模型配置与 Prompt 模板</summary>
        <div class="collapse-body stack">
          <p class="hint">
            这里的改动会进入 AI 配置草稿，点击页头「保存 AI 配置」后写入本机；未保存时页头会提示。
          </p>
          <LlmProviderSettings v-model:providers="draft.providers" :entries="store.secretEntries" />
          <div class="divider" />
          <h4 class="sub-title"><AppIcon name="braces" :size="16" />Prompt 模板</h4>
          <PromptTemplateManager v-model:templates="draft.templates" />
        </div>
      </details>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import LlmRequestCard from "./LlmRequestCard.vue";
import LlmResultsCard from "./LlmResultsCard.vue";
import LlmUsagePanel from "./LlmUsagePanel.vue";
import LlmProviderSettings from "./LlmProviderSettings.vue";
import PromptTemplateManager from "./PromptTemplateManager.vue";
import { api } from "../../api";
import { loadSecretEntries, store } from "../../store";
import { describeError } from "../../utils/errors";
import { llmDraft } from "../../composables/useLlmDraft";
import {
  cloneLlmConfig,
  missingVariables,
  substituteVariables,
  templateVariables,
} from "../../utils/llm";
import type { LlmResponse, LlmUsageSummary } from "../../types";

const message = useMessage();

const draft = llmDraft;
const selectedIds = ref<string[]>([]);
const systemPrompt = ref("");
const promptText = ref("");
const templateId = ref<string | null>(null);
const varValues = reactive<Record<string, string>>({});
const temperature = ref<number | null>(null);
const maxTokens = ref<number | null>(null);
const results = ref<LlmResponse[]>([]);
const running = ref(false);
const usage = ref<LlmUsageSummary | null>(null);
const usageLoading = ref(false);
const usageDays = 7;

const variables = computed(() => templateVariables(promptText.value));

const templateOptions = computed(() =>
  draft.value.templates.map((item) => ({ label: item.name, value: item.id })),
);

const readyIds = computed(() =>
  draft.value.providers
    .filter((provider) => keyReady(provider.keyEntryId))
    .map((provider) => provider.id),
);

const pendingProviders = computed(() =>
  running.value
    ? draft.value.providers.filter((provider) => selectedIds.value.includes(provider.id))
    : [],
);

const resultsMeta = computed(() => {
  if (running.value) return `${pendingProviders.value.length} 个模型并行请求中…`;
  if (!results.value.length) return "";
  const totalMs = Math.max(...results.value.map((item) => item.latencyMs));
  const tokens = results.value
    .filter((item) => !item.error)
    .reduce((sum, item) => sum + item.totalTokens, 0);
  const tokenText = tokens ? ` · 合计 ${tokens} tok` : "";
  return `${results.value.length} 个模型 · 总耗时 ${totalMs} ms${tokenText}`;
});

function keyReady(keyEntryId: string): boolean {
  if (!keyEntryId) return false;
  return store.secretEntries.some((entry) => entry.id === keyEntryId);
}

function initFromConfig(): void {
  draft.value = cloneLlmConfig(store.config?.llm);
  const withKeys = draft.value.providers
    .filter((provider) => keyReady(provider.keyEntryId))
    .map((provider) => provider.id);
  selectedIds.value = withKeys.length
    ? withKeys
    : draft.value.providers.map((provider) => provider.id);
  const first = draft.value.templates[0];
  if (first && !promptText.value) {
    templateId.value = first.id;
    promptText.value = first.content;
    syncVariables();
  }
}

function syncVariables(): void {
  for (const name of variables.value) {
    if (varValues[name] === undefined) varValues[name] = "";
  }
}

function applyTemplate(id: string | null): void {
  templateId.value = id;
  const template = draft.value.templates.find((item) => item.id === id);
  if (!template) return;
  promptText.value = template.content;
  syncVariables();
}

function clearResults(): void {
  if (running.value) return;
  results.value = [];
}

async function refreshUsage(): Promise<void> {
  if (usageLoading.value) return;
  usageLoading.value = true;
  try {
    usage.value = await api.llmUsage(usageDays);
  } catch {
    usage.value = null;
  } finally {
    usageLoading.value = false;
  }
}

async function run(): Promise<void> {
  if (running.value) return;
  const providerIds = selectedIds.value.filter((id) =>
    draft.value.providers.some((provider) => provider.id === id),
  );
  if (!providerIds.length) {
    message.warning("请至少选择一个模型");
    return;
  }
  const missing = missingVariables(promptText.value, varValues);
  if (missing.length) {
    message.warning(`请填写变量：${missing.join("、")}`);
    return;
  }
  const prompt = substituteVariables(promptText.value, varValues);
  if (!prompt.trim()) {
    message.warning("Prompt 不能为空");
    return;
  }
  const previous = results.value;
  results.value = [];
  running.value = true;
  try {
    results.value = await api.llmRun({
      providerIds,
      system: systemPrompt.value,
      prompt,
      temperature: temperature.value,
      maxTokens: maxTokens.value,
    });
    void refreshUsage();
  } catch (err) {
    // 请求整体失败时恢复上一次结果，避免用户丢失已完成的对比
    results.value = previous;
    message.error(`请求失败：${describeError(err)}`);
  } finally {
    running.value = false;
  }
}

onMounted(async () => {
  try {
    await loadSecretEntries(true);
  } catch {
    // 凭据读取失败时仍可使用已保存的模型配置，运行时会给出明确错误。
  }
  initFromConfig();
  await refreshUsage();
});
</script>

<style scoped>
.bench {
  display: grid;
  grid-template-columns: 380px minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}
.side {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}
.divider {
  height: 1px;
  background: var(--border);
}
.sub-title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: var(--fs-13);
  font-weight: 600;
  color: var(--text-2);
}
.sub-title .ic {
  width: 16px;
  height: 16px;
  color: var(--primary);
}
@media (max-width: 1100px) {
  .bench {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
