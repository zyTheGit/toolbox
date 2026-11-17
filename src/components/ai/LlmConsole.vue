<template>
  <div class="card">
    <div class="spread head">
      <div>
        <h2 class="section-title">LLM 调试台</h2>
        <p class="hint">同一 Prompt 并行请求多个模型，并排对比回复、耗时与 Token 用量。</p>
      </div>
      <div class="head-actions">
        <n-tag v-if="dirty" size="small" type="warning" :bordered="false">AI 配置未保存</n-tag>
        <n-button size="small" :disabled="!dirty || saving" :loading="saving" @click="saveConfig">
          保存 AI 配置
        </n-button>
      </div>
    </div>

    <div class="form">
      <div class="field">
        <span class="field-label">Prompt 模板</span>
        <div class="template-row">
          <n-select
            v-model:value="templateId"
            size="small"
            clearable
            placeholder="选择模板（可选）"
            :options="templateOptions"
            class="template-select"
            @update:value="applyTemplate"
          />
          <span class="hint">模板支持 &#123;&#123;变量&#125;&#125; 占位符</span>
        </div>
      </div>

      <label class="field">
        <span class="field-label">System Prompt（可选）</span>
        <n-input
          v-model:value="systemPrompt"
          type="textarea"
          size="small"
          :autosize="{ minRows: 2, maxRows: 4 }"
          placeholder="例如：你是一名严谨的技术顾问，回答尽量简洁。"
        />
      </label>

      <label class="field">
        <span class="field-label">Prompt</span>
        <n-input
          v-model:value="promptText"
          type="textarea"
          :autosize="{ minRows: 6, maxRows: 14 }"
          placeholder="输入要发送的内容，可使用 {{name}} 变量"
        />
      </label>

      <div v-if="variables.length" class="var-grid">
        <label v-for="name in variables" :key="name" class="field">
          <span class="field-label">变量 · {{ name }}</span>
          <n-input
            v-model:value="varValues[name]"
            size="small"
            :placeholder="`填写 ${name} 的值`"
          />
        </label>
      </div>

      <div class="param-row">
        <label class="field param">
          <span class="field-label">温度（留空跟随模型）</span>
          <n-input-number
            v-model:value="temperature"
            size="small"
            :min="0"
            :max="2"
            :step="0.1"
            clearable
            placeholder="默认"
          />
        </label>
        <label class="field param">
          <span class="field-label">最大输出 Token（可选）</span>
          <n-input-number
            v-model:value="maxTokens"
            size="small"
            :min="1"
            :max="32768"
            clearable
            placeholder="默认"
          />
        </label>
      </div>

      <div class="field">
        <span class="field-label">发送到</span>
        <n-checkbox-group v-model:value="selectedIds">
          <div class="provider-picks">
            <n-checkbox v-for="provider in draft.providers" :key="provider.id" :value="provider.id">
              <span class="pick-name">{{ provider.name }}</span>
              <n-tag
                size="tiny"
                :bordered="false"
                :type="keyReady(provider.keyEntryId) ? 'success' : 'default'"
              >
                {{ keyReady(provider.keyEntryId) ? provider.model : "未配置凭据" }}
              </n-tag>
            </n-checkbox>
          </div>
        </n-checkbox-group>
      </div>

      <div class="run-row">
        <n-button type="primary" :loading="running" :disabled="!draft.providers.length" @click="run">
          并行请求
        </n-button>
        <n-button v-if="results.length" quaternary :disabled="running" @click="results = []">
          清空结果
        </n-button>
        <span class="hint">请求为非流式，超时 120 秒。</span>
      </div>
    </div>
  </div>

  <div v-if="results.length" class="card">
    <div class="spread">
      <h2 class="section-title">对比结果（{{ results.length }}）</h2>
    </div>
    <div class="results">
      <LlmResultCard v-for="item in results" :key="item.providerId" :response="item" />
    </div>
  </div>

  <LlmUsagePanel
    :usage="usage"
    :days="usageDays"
    :loading="usageLoading"
    @refresh="refreshUsage"
  />

  <div class="card">
    <n-collapse>
      <n-collapse-item title="模型配置" name="providers">
        <LlmProviderSettings
          v-model:providers="draft.providers"
          :entries="store.secretEntries"
        />
      </n-collapse-item>
      <n-collapse-item title="Prompt 模板" name="templates">
        <PromptTemplateManager v-model:templates="draft.templates" />
      </n-collapse-item>
    </n-collapse>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  NButton,
  NCheckbox,
  NCheckboxGroup,
  NCollapse,
  NCollapseItem,
  NInput,
  NInputNumber,
  NSelect,
  NTag,
  useMessage,
} from "naive-ui";
import { api } from "../../api";
import { loadSecretEntries, store } from "../../store";
import { describeError } from "../../utils/errors";
import {
  cloneLlmConfig,
  missingVariables,
  substituteVariables,
  templateVariables,
} from "../../utils/llm";
import type {
  AppConfig,
  LlmConfig,
  LlmResponse,
  LlmUsageSummary,
} from "../../types";
import LlmProviderSettings from "./LlmProviderSettings.vue";
import PromptTemplateManager from "./PromptTemplateManager.vue";
import LlmResultCard from "./LlmResultCard.vue";
import LlmUsagePanel from "./LlmUsagePanel.vue";

const message = useMessage();

const draft = ref<LlmConfig>(cloneLlmConfig(store.config?.llm));
const selectedIds = ref<string[]>([]);
const systemPrompt = ref("");
const promptText = ref("");
const templateId = ref<string | null>(null);
const varValues = reactive<Record<string, string>>({});
const temperature = ref<number | null>(null);
const maxTokens = ref<number | null>(null);
const results = ref<LlmResponse[]>([]);
const running = ref(false);
const saving = ref(false);
const usage = ref<LlmUsageSummary | null>(null);
const usageLoading = ref(false);
const usageDays = 7;

const dirty = computed(
  () => JSON.stringify(draft.value) !== JSON.stringify(store.config?.llm ?? null),
);

const variables = computed(() => templateVariables(promptText.value));

const templateOptions = computed(() =>
  draft.value.templates.map((item) => ({ label: item.name, value: item.id })),
);

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
  if (!selectedIds.value.length) {
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
  running.value = true;
  try {
    results.value = await api.llmRun({
      providerIds: [...selectedIds.value],
      system: systemPrompt.value,
      prompt,
      temperature: temperature.value,
      maxTokens: maxTokens.value,
    });
    void refreshUsage();
  } catch (err) {
    message.error(`请求失败：${describeError(err)}`);
  } finally {
    running.value = false;
  }
}

async function saveConfig(): Promise<void> {
  if (!store.config || saving.value) return;
  const payload = JSON.parse(JSON.stringify(store.config)) as AppConfig;
  payload.llm = JSON.parse(JSON.stringify(draft.value));
  saving.value = true;
  try {
    const result = await api.saveConfig(payload);
    store.config = result.config;
    store.tools = result.tools;
    store.providers = result.providers;
    store.secrets = result.secrets;
    draft.value = cloneLlmConfig(result.config.llm);
    message.success("AI 配置已保存");
  } catch (err) {
    message.error(`保存失败：${describeError(err)}`);
  } finally {
    saving.value = false;
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
.head {
  align-items: flex-start;
}
.head .hint {
  margin: 4px 0 0;
}
.head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 14px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}
.field-label {
  font-size: 12px;
  font-weight: 500;
  color: #4b5468;
}
.template-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.template-select {
  max-width: 240px;
}
.var-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 14px;
}
.param-row {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
}
.param {
  max-width: 220px;
}
.provider-picks {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 18px;
}
.pick-name {
  margin-right: 6px;
}
.run-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.results {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 12px;
  margin-top: 4px;
}
@media (max-width: 900px) {
  .var-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
