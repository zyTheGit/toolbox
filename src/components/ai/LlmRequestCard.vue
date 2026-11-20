<template>
  <section class="card">
    <h3 class="card-title request-title">
      <AppIcon name="terminal" :size="16" />请求
    </h3>

    <div class="stack form">
      <div class="field">
        <label>Prompt 模板</label>
        <n-select
          size="small"
          clearable
          :value="templateId"
          :options="templateOptions"
          placeholder="自定义（不使用模板）"
          @update:value="selectTemplate"
        />
        <span class="hint">模板支持 &#123;&#123;变量&#125;&#125; 占位符，可在「模型配置与 Prompt 模板」中维护。</span>
      </div>

      <div class="field">
        <label>System Prompt（可选）</label>
        <n-input
          type="textarea"
          size="small"
          :autosize="{ minRows: 2, maxRows: 4 }"
          :value="systemPrompt"
          placeholder="例如：你是一名严谨的技术顾问，回答尽量简洁。"
          @update:value="emit('update:systemPrompt', $event)"
        />
      </div>

      <div class="field">
        <label>Prompt</label>
        <n-input
          type="textarea"
          :autosize="{ minRows: 4, maxRows: 12 }"
          :value="promptText"
          placeholder="输入要发送的内容，可使用 {{name}} 变量"
          @update:value="emit('update:promptText', $event)"
        />
      </div>

      <div v-if="variables.length" class="var-grid">
        <div v-for="name in variables" :key="name" class="field">
          <label>变量 · {{ name }}</label>
          <n-input
            size="small"
            :value="varValues[name] ?? ''"
            :placeholder="`填写 ${name} 的值`"
            @update:value="emit('update:variable', name, $event)"
          />
        </div>
      </div>

      <div class="form-grid">
        <div class="field">
          <label>温度</label>
          <n-input-number
            size="small"
            :value="temperature"
            :min="0"
            :max="2"
            :step="0.1"
            clearable
            placeholder="跟随模型"
            @update:value="emit('update:temperature', $event)"
          />
        </div>
        <div class="field">
          <label>最大 Token</label>
          <n-input-number
            size="small"
            :value="maxTokens"
            :min="1"
            :max="32768"
            clearable
            placeholder="跟随模型"
            @update:value="emit('update:maxTokens', $event)"
          />
        </div>
      </div>

      <div class="field">
        <label>发送到</label>
        <label v-for="provider in providers" :key="provider.id" class="model-check">
          <input
            type="checkbox"
            :checked="selectedIds.includes(provider.id)"
            @change="toggle(provider.id)"
          />
          <span class="grow">
            <span class="name">{{ provider.name }}</span>
            <span class="model">{{ provider.model || "未设置模型" }}</span>
          </span>
          <span v-if="!readyIds.includes(provider.id)" class="tag tag-warn">未配凭据</span>
        </label>
        <p v-if="!providers.length" class="hint">
          还没有模型，在「模型配置与 Prompt 模板」中添加后即可选择。
        </p>
      </div>

      <div class="row run-row">
        <n-button
          type="primary"
          class="send-btn"
          :loading="running"
          :disabled="!providers.length"
          @click="emit('run')"
        >
          <template #icon><AppIcon name="zap" :size="16" /></template>
          {{ running ? "请求中…" : "并行请求" }}
        </n-button>
        <n-button v-if="hasResults" quaternary :disabled="running" @click="emit('clear')">
          清空结果
        </n-button>
      </div>
      <p class="hint">
        实验功能：真实调用需先在「API Key 管理器」配置凭据；请求为非流式、超时 120 秒，任一模型失败不影响其它模型。
      </p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { NButton, NInput, NInputNumber, NSelect } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import type { LlmProviderConfig } from "../../types";

const props = defineProps<{
  providers: LlmProviderConfig[];
  selectedIds: string[];
  /** 已配置凭据的服务商 id，用于标注「未配凭据」。 */
  readyIds: string[];
  systemPrompt: string;
  promptText: string;
  templateId: string | null;
  templateOptions: Array<{ label: string; value: string }>;
  variables: string[];
  varValues: Record<string, string>;
  temperature: number | null;
  maxTokens: number | null;
  running: boolean;
  hasResults: boolean;
}>();

const emit = defineEmits<{
  "update:selectedIds": [ids: string[]];
  "update:systemPrompt": [text: string];
  "update:promptText": [text: string];
  "update:templateId": [id: string | null];
  "update:variable": [name: string, value: string];
  "update:temperature": [value: number | null];
  "update:maxTokens": [value: number | null];
  run: [];
  clear: [];
}>();

function selectTemplate(value: string | number | null): void {
  emit("update:templateId", value === null ? null : String(value));
}

function toggle(id: string): void {
  const next = props.selectedIds.includes(id)
    ? props.selectedIds.filter((item) => item !== id)
    : [...props.selectedIds, id];
  emit("update:selectedIds", next);
}
</script>

<style scoped>
.request-title {
  margin-bottom: 12px;
}
.form {
  gap: 12px;
}
.var-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 12px;
}
.model-check {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  cursor: pointer;
}
.model-check + .model-check {
  margin-top: 8px;
}
/* 覆盖 .field > label 的字段标题样式，勾选行按正文样式显示 */
.field > .model-check {
  font-weight: 400;
  color: var(--text);
}
.model-check:hover {
  border-color: var(--border-strong);
}
.model-check input {
  flex: none;
  width: 15px;
  height: 15px;
  margin: 0;
  accent-color: var(--primary);
  cursor: pointer;
}
.model-check .name {
  display: block;
  font-size: var(--fs-13);
  font-weight: 500;
  line-height: 1.4;
}
.model-check .model {
  display: block;
  font-family: var(--font-mono);
  font-size: var(--fs-12);
  line-height: 1.5;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.run-row {
  flex-wrap: wrap;
}
.send-btn {
  flex: 1;
}
@media (max-width: 520px) {
  .var-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
