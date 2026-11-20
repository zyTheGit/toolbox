<template>
  <div class="llm-card" :class="{ failed: Boolean(response.error) }">
    <div class="llm-head">
      <span class="tag tag-primary">{{ response.providerName }}</span>
      <span class="tag">{{ response.model }}</span>
      <span v-if="response.error" class="pill pill-err">失败</span>
      <span v-else class="hint meta">{{ meta }}</span>
      <span class="grow"></span>
      <n-button
        v-if="response.content"
        quaternary
        circle
        size="tiny"
        title="复制回答"
        @click="copy"
      >
        <template #icon><AppIcon name="copy" :size="16" /></template>
      </n-button>
    </div>

    <div v-if="response.error" class="alert alert-err">
      <AppIcon name="alert" :size="16" />
      <span>{{ response.error }}</span>
    </div>
    <div v-else class="llm-body">{{ response.content }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { copyText } from "../../utils/clipboard";
import { describeError } from "../../utils/errors";
import type { LlmResponse } from "../../types";

const props = defineProps<{ response: LlmResponse }>();
const message = useMessage();

const meta = computed(() => {
  const value = props.response;
  const latency = `${value.latencyMs} ms`;
  if (!value.totalTokens) return latency;
  return `${latency} · ${value.promptTokens}+${value.completionTokens}=${value.totalTokens} tok`;
});

async function copy(): Promise<void> {
  try {
    await copyText(props.response.content);
    message.success("已复制回复内容");
  } catch (err) {
    message.error(describeError(err));
  }
}
</script>

<style scoped>
.llm-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
}
.llm-card.failed {
  border-color: var(--danger);
}
.llm-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.llm-body {
  font-size: var(--fs-13);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 320px;
  overflow: auto;
}
</style>
