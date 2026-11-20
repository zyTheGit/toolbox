<template>
  <div class="provider">
    <div class="provider-head">
      <n-input
        :value="provider.name"
        size="small"
        placeholder="显示名称"
        class="name-input"
        @update:value="emitValue('name', $event)"
      />
      <span class="tag" :class="ready ? 'tag-primary' : 'tag-warn'">
        {{ ready ? "凭据就绪" : "缺少凭据" }}
      </span>
      <span class="grow" />
      <n-button quaternary circle size="small" title="移除模型" @click="emit('remove')">
        <template #icon><AppIcon name="trash" :size="16" /></template>
      </n-button>
    </div>
    <div class="form-grid">
      <div class="field">
        <label>接口地址</label>
        <n-input
          :value="provider.baseUrl"
          size="small"
          placeholder="https://api.example.com/v1"
          @update:value="emitValue('baseUrl', $event)"
        />
      </div>
      <div class="field">
        <label>模型名</label>
        <n-input
          :value="provider.model"
          size="small"
          placeholder="gpt-4o-mini / glm-4-flash"
          @update:value="emitValue('model', $event)"
        />
      </div>
      <div class="field">
        <label>凭据</label>
        <n-select
          :value="provider.keyEntryId"
          size="small"
          :options="keyOptions"
          clearable
          placeholder="选择 API Key 条目"
          @update:value="emitValue('keyEntryId', $event ?? '')"
        />
      </div>
      <div class="field">
        <label>默认温度</label>
        <n-input-number
          :value="provider.temperature"
          size="small"
          :min="0"
          :max="2"
          :step="0.1"
          @update:value="emitValue('temperature', $event ?? 0.7)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NInput, NInputNumber, NSelect } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import type { LlmProviderConfig, SecretEntryView } from "../../types";

const props = defineProps<{
  provider: LlmProviderConfig;
  entries: SecretEntryView[];
}>();

const emit = defineEmits<{
  update: [provider: LlmProviderConfig];
  remove: [];
}>();

const keyOptions = computed(() =>
  props.entries.map((entry) => ({
    label: `${entry.name}（${entry.service}）`,
    value: entry.id,
  })),
);

const ready = computed(
  () =>
    Boolean(props.provider.keyEntryId) &&
    props.entries.some((entry) => entry.id === props.provider.keyEntryId),
);

function emitValue(key: keyof LlmProviderConfig, value: string | number): void {
  emit("update", { ...props.provider, [key]: value });
}
</script>

<style scoped>
.provider {
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  background: var(--surface);
  padding: 12px;
}
.provider-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.name-input {
  max-width: 260px;
}
</style>
