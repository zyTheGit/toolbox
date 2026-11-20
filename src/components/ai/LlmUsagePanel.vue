<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="table" :size="16" />用量统计<span class="count">· 最近 {{ days }} 天</span>
      </h3>
      <n-button
        quaternary
        circle
        size="tiny"
        title="刷新用量"
        :loading="loading"
        @click="emit('refresh')"
      >
        <template #icon><AppIcon name="refresh" :size="16" /></template>
      </n-button>
    </div>

    <template v-if="rows.length">
      <div class="table-wrap">
        <table class="tbl">
          <thead>
            <tr>
              <th>模型</th>
              <th>请求数</th>
              <th>Token</th>
              <th class="share-col">占比</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in rows" :key="`${item.providerName}-${item.model}`">
              <td>
                <span class="mono">{{ item.model || "—" }}</span>
                <span class="provider">{{ item.providerName }}</span>
              </td>
              <td class="num">{{ formatNumber(item.calls) }}</td>
              <td class="num">{{ formatNumber(tokensOf(item)) }}</td>
              <td><div class="usage-bar" :style="{ width: shareOf(item) }" /></td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="hint totals">{{ totals }}</p>
    </template>

    <EmptyState
      v-else
      icon="table"
      title="还没有用量记录"
      description="成功调用后会在这里按模型聚合 Token 用量。"
    />
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
import type { LlmUsageModel, LlmUsageSummary } from "../../types";

const props = defineProps<{
  usage: LlmUsageSummary | null;
  days: number;
  loading: boolean;
}>();

const emit = defineEmits<{
  refresh: [];
}>();

const rows = computed<LlmUsageModel[]>(() => props.usage?.models ?? []);

const maxTokens = computed(() =>
  Math.max(1, ...rows.value.map((item) => tokensOf(item))),
);

const totals = computed(() => {
  const value = props.usage;
  if (!value) return "";
  return `共 ${formatNumber(value.totalCalls)} 次调用 · 输入 ${formatNumber(
    value.totalPromptTokens,
  )} tok · 输出 ${formatNumber(value.totalCompletionTokens)} tok`;
});

function tokensOf(item: LlmUsageModel): number {
  return item.promptTokens + item.completionTokens;
}

/** 占比条按用量最大的模型归一，保留最小 6% 让低用量模型也能看见。 */
function shareOf(item: LlmUsageModel): string {
  return `${Math.max(6, Math.round((tokensOf(item) / maxTokens.value) * 100))}%`;
}

function formatNumber(value: number): string {
  return value.toLocaleString("zh-CN");
}
</script>

<style scoped>
.share-col {
  width: 30%;
}
.usage-bar {
  height: 6px;
  border-radius: 3px;
  background: linear-gradient(90deg, var(--primary), var(--info));
}
.provider {
  margin-left: 8px;
  font-size: var(--fs-12);
  color: var(--faint);
}
.totals {
  margin: 10px 0 0;
}
</style>
