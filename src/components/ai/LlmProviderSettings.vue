<template>
  <div class="providers">
    <LlmProviderRow
      v-for="(item, i) in providers"
      :key="item.id"
      :provider="item"
      :entries="entries"
      @update="patch(i, $event)"
      @remove="remove(i)"
    />
    <div class="provider-foot">
      <n-button size="small" dashed @click="add">添加模型</n-button>
      <span class="hint">接口需兼容 OpenAI Chat Completions；凭据在「API Key 管理器」中维护。</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton } from "naive-ui";
import LlmProviderRow from "./LlmProviderRow.vue";
import { newId } from "../../utils/llm";
import type { LlmProviderConfig, SecretEntryView } from "../../types";

const props = defineProps<{
  providers: LlmProviderConfig[];
  entries: SecretEntryView[];
}>();

const emit = defineEmits<{
  "update:providers": [providers: LlmProviderConfig[]];
}>();

function commit(next: LlmProviderConfig[]): void {
  emit("update:providers", next);
}

function patch(index: number, next: LlmProviderConfig): void {
  commit(props.providers.map((item, i) => (i === index ? next : item)));
}

function add(): void {
  commit([
    ...props.providers,
    {
      id: newId("provider"),
      name: "自定义模型",
      baseUrl: "https://",
      model: "",
      keyEntryId: "",
      temperature: 0.7,
    },
  ]);
}

function remove(index: number): void {
  commit(props.providers.filter((_, i) => i !== index));
}
</script>

<style scoped>
.providers {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.provider-foot {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
</style>
