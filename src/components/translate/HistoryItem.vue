<template>
  <div class="hist-item" :class="{ compact }">
    <div class="hist-main">
      <div class="hist-text" :class="compact ? 'clamp-1' : 'clamp-4'">{{ entry.text }}</div>
      <div class="hist-result" :class="compact ? 'clamp-1' : 'clamp-4'">{{ entry.result }}</div>
      <div class="hist-meta muted">
        <span class="mono">{{ formatTime(entry.createdAt) }}</span>
        <span class="sep">·</span>
        <span>{{ entry.providerName }}</span>
        <span class="sep">·</span>
        <span>{{ langLabel(entry.source) }} → {{ langLabel(entry.target) }}</span>
      </div>
    </div>

    <div class="hist-actions">
      <n-button
        class="star-btn"
        quaternary
        circle
        size="small"
        :title="entry.favorite ? '取消收藏' : '收藏'"
        :aria-label="entry.favorite ? '取消收藏' : '收藏'"
        @click="emit('favorite')"
      >
        <span class="star" :class="{ on: entry.favorite }">{{ entry.favorite ? "★" : "☆" }}</span>
      </n-button>
      <n-button text size="small" @click="emit('load')">载入</n-button>
      <n-button
        v-if="!compact"
        text
        size="small"
        :loading="retranslating"
        :disabled="retranslating"
        @click="emit('retranslate')"
      >
        重译
      </n-button>
      <n-button v-if="!compact" text size="small" @click="emit('copy')">复制</n-button>
      <span v-if="!compact" class="action-sep" aria-hidden="true"></span>
      <n-popconfirm
        positive-text="删除"
        negative-text="取消"
        @positive-click="emit('delete')"
      >
        <template #trigger>
          <n-button text size="small" type="error">删除</n-button>
        </template>
        删除这条翻译记录？
      </n-popconfirm>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton, NPopconfirm } from "naive-ui";
import type { HistoryEntry } from "../../types";

defineProps<{ entry: HistoryEntry; compact?: boolean; retranslating?: boolean }>();

const emit = defineEmits<{
  (e: "load"): void;
  (e: "retranslate"): void;
  (e: "favorite"): void;
  (e: "copy"): void;
  (e: "delete"): void;
}>();

const LANG_LABELS: Record<string, string> = { zh: "中文", en: "英文" };

function formatTime(iso: string): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
}

function langLabel(code: string): string {
  if (!code) return "—";
  return LANG_LABELS[code] ?? code.toUpperCase();
}
</script>

<style scoped>
.hist-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 0;
  border-bottom: 1px solid var(--border);
}

.hist-item:last-child {
  border-bottom: 0;
}

.hist-item.compact {
  padding: 10px 0;
  gap: 10px;
}

.hist-main {
  flex: 1;
  min-width: 0;
}

.hist-text {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.hist-result {
  margin-top: 4px;
  font-size: 13px;
  color: #46506a;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.hist-meta {
  margin-top: 6px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  line-height: 1.4;
}

.hist-meta .mono {
  font-size: 12px;
}

.sep {
  color: #c3c9d4;
}

.clamp-1,
.clamp-4 {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  overflow: hidden;
  white-space: normal;
}

.clamp-1 {
  -webkit-line-clamp: 1;
}

.clamp-4 {
  -webkit-line-clamp: 4;
}

.hist-actions {
  flex: none;
  display: flex;
  align-items: center;
  gap: 6px;
  padding-top: 1px;
}

.hist-actions :deep(.n-button) {
  min-height: 28px;
}

.action-sep {
  width: 1px;
  height: 16px;
  margin: 0 4px;
  background: var(--border);
}

.star-btn {
  margin-right: 2px;
  min-width: 28px;
}

.star {
  font-size: 15px;
  line-height: 1;
  color: #b3bac7;
}

.star.on {
  color: #e0a13c;
}
</style>
