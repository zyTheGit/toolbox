<template>
  <div class="provider">
    <div class="provider-head">
      <n-input :value="provider.name" size="small" placeholder="显示名称" class="name-input" @update:value="emitValue('name', $event)" />
      <n-button size="tiny" quaternary type="error" @click="emit('remove')">移除</n-button>
    </div>
    <div class="provider-grid">
      <div class="field">
        <span class="field-label">接口地址</span>
        <n-input :value="provider.baseUrl" size="small" placeholder="https://api.example.com/v1" @update:value="emitValue('baseUrl', $event)" />
      </div>
      <div class="field">
        <span class="field-label">模型名</span>
        <n-input :value="provider.model" size="small" placeholder="gpt-4o-mini / glm-4-flash" @update:value="emitValue('model', $event)" />
      </div>
      <div class="field">
        <span class="field-label">凭据</span>
        <n-select :value="provider.keyEntryId" size="small" :options="keyOptions" clearable placeholder="选择 API Key 条目" @update:value="emitValue('keyEntryId', $event ?? '')" />
      </div>
      <div class="field">
        <span class="field-label">默认温度</span>
        <n-input-number :value="provider.temperature" size="small" :min="0" :max="2" :step="0.1" @update:value="emitValue('temperature', $event ?? 0.7)" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NInput, NInputNumber, NSelect } from "naive-ui";
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

function emitValue(key: keyof LlmProviderConfig, value: string | number): void {
  emit("update", { ...props.provider, [key]: value });
}
</script>

<style scoped>
.provider {
  border: 1px solid var(--border);
  border-radius: 8px;
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
.provider-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 14px;
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
@media (max-width: 900px) {
  .provider-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
