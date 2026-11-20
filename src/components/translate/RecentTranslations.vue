<template>
  <section class="card recent-card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="history" :size="16" />
        最近翻译
        <span v-if="historyEnabled && entries.length" class="count">· {{ entries.length }}</span>
      </h3>
      <n-button quaternary size="small" @click="emit('open')">
        <span class="view-all">
          查看全部
          <AppIcon name="chevron-right" :size="16" />
        </span>
      </n-button>
    </div>

    <div v-if="!historyEnabled">
      <EmptyState
        icon="history"
        title="历史记录已关闭"
        description="可在「设置」中开启翻译历史。"
      />
    </div>
    <template v-else>
      <div v-if="loading" class="recent-loading">
        <n-spin size="small" />
      </div>
      <div v-else-if="entries.length" class="recent-list">
        <HistoryItem
          v-for="entry in entries"
          :key="entry.id"
          :entry="entry"
          compact
          @load="emit('load', entry)"
          @favorite="emit('favorite', entry)"
          @copy="emit('copy', entry)"
        />
      </div>
      <EmptyState
        v-else-if="error"
        icon="alert"
        title="最近翻译加载失败"
        :description="error"
      >
        <n-button size="small" :loading="loading" @click="emit('retry')">重试</n-button>
      </EmptyState>
      <EmptyState
        v-else
        icon="inbox"
        title="暂无翻译记录"
        description="翻译成功后会自动保存到这里。"
      />
    </template>
  </section>
</template>

<script setup lang="ts">
import { NButton, NSpin } from "naive-ui";
import type { HistoryEntry } from "../../types";
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
import HistoryItem from "./HistoryItem.vue";

defineProps<{
  entries: HistoryEntry[];
  loading: boolean;
  error: string;
  historyEnabled: boolean;
}>();

const emit = defineEmits<{
  (e: "open"): void;
  (e: "load", entry: HistoryEntry): void;
  (e: "favorite", entry: HistoryEntry): void;
  (e: "copy", entry: HistoryEntry): void;
  (e: "retry"): void;
}>();
</script>

<style scoped>
.recent-card {
  margin-top: 16px;
}

.view-all {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.recent-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 28px 0;
}

.recent-list {
  display: flex;
  flex-direction: column;
}
</style>
