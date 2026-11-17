<template>
  <div class="result" :class="{ failed: Boolean(response.error) }">
    <div class="result-head">
      <div class="title">
        <span class="provider">{{ response.providerName }}</span>
        <n-tag size="tiny" :bordered="false">{{ response.model }}</n-tag>
      </div>
      <div class="meta">
        <span class="mono">{{ response.latencyMs }} ms</span>
        <span v-if="response.totalTokens" class="mono">
          {{ response.promptTokens }} + {{ response.completionTokens }} = {{ response.totalTokens }} tok
        </span>
        <n-button
          v-if="response.content"
          size="tiny"
          quaternary
          @click="copy"
        >
          复制
        </n-button>
      </div>
    </div>
    <n-alert v-if="response.error" type="error" :bordered="false">{{ response.error }}</n-alert>
    <pre v-else class="content">{{ response.content }}</pre>
  </div>
</template>

<script setup lang="ts">
import { NAlert, NButton, NTag, useMessage } from "naive-ui";
import { copyText } from "../../utils/clipboard";
import { describeError } from "../../utils/errors";
import type { LlmResponse } from "../../types";

const props = defineProps<{ response: LlmResponse }>();
const message = useMessage();

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
.result {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}
.result.failed {
  border-color: #f0c8c8;
  background: #fffafa;
}
.result-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-wrap: wrap;
}
.title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.provider {
  font-size: 13px;
  font-weight: 600;
}
.meta {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--muted);
  font-size: 11px;
}
.content {
  margin: 0;
  max-height: 360px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 13px;
  line-height: 1.6;
  font-family: inherit;
}
</style>
