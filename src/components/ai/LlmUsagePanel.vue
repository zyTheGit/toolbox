<template>
  <div class="card">
    <div class="spread">
      <h2 class="section-title">用量统计（最近 {{ days }} 天）</h2>
      <n-button size="tiny" quaternary :loading="loading" @click="emit('refresh')">刷新</n-button>
    </div>
    <div v-if="usage && usage.totalCalls" class="usage">
      <div class="usage-total">
        <span>调用 {{ usage.totalCalls }} 次</span>
        <span>输入 {{ usage.totalPromptTokens }} tok</span>
        <span>输出 {{ usage.totalCompletionTokens }} tok</span>
      </div>
      <div class="usage-list">
        <div v-for="item in usage.models" :key="`${item.providerName}-${item.model}`" class="usage-row">
          <span class="usage-name">{{ item.providerName }} · {{ item.model }}</span>
          <span class="mono">{{ item.calls }} 次</span>
          <span class="mono">{{ item.promptTokens }} + {{ item.completionTokens }} tok</span>
        </div>
      </div>
    </div>
    <p v-else class="hint">暂无用量记录，成功调用后会在这里按模型聚合。</p>
  </div>
</template>

<script setup lang="ts">
import { NButton } from "naive-ui";
import type { LlmUsageSummary } from "../../types";

defineProps<{
  usage: LlmUsageSummary | null;
  days: number;
  loading: boolean;
}>();

const emit = defineEmits<{
  refresh: [];
}>();
</script>

<style scoped>
.usage {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.usage-total {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
  font-size: 13px;
  font-weight: 600;
}
.usage-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.usage-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 12px;
  color: var(--muted);
  border-top: 1px dashed var(--border);
  padding-top: 6px;
}
.usage-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #333b4d;
}
</style>
