<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="sparkles" :size="16" />对比结果</h3>
      <span v-if="meta" class="hint meta">{{ meta }}</span>
    </div>

    <div v-if="pending.length" class="results">
      <div v-for="provider in pending" :key="provider.id" class="llm-card">
        <div class="llm-head">
          <span class="tag tag-primary">{{ provider.name }}</span>
          <span class="tag">{{ provider.model || "未设置模型" }}</span>
          <span class="hint">等待响应…</span>
        </div>
        <div class="llm-body muted">正在请求…</div>
      </div>
    </div>

    <div v-else-if="results.length" class="results">
      <LlmResultCard v-for="item in results" :key="item.providerId" :response="item" />
    </div>

    <EmptyState
      v-else
      icon="sparkles"
      title="还没有对比结果"
      description="选择模型后点击「并行请求」，结果将并排展示。"
    />
  </section>
</template>

<script setup lang="ts">
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
import LlmResultCard from "./LlmResultCard.vue";
import type { LlmProviderConfig, LlmResponse } from "../../types";

defineProps<{
  results: LlmResponse[];
  /** 请求进行中的服务商，用于渲染等待中的卡片。 */
  pending: LlmProviderConfig[];
  meta: string;
}>();
</script>

<style scoped>
.results {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 10px;
}
.llm-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
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
}
</style>
