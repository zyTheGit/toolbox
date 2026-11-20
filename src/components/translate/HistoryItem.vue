<template>
  <div class="hist-row" :class="{ compact }">
    <span class="tag dir-tag">{{ directionLabel }}</span>
    <div class="hist-texts" :title="`${entry.text}\n${entry.result || '（无译文）'}`">
      <span class="src ellipsis">{{ entry.text }}</span>
      <span class="dst ellipsis" :class="{ empty: !entry.result }">
        {{ entry.result || "（无译文）" }}
      </span>
    </div>
    <span class="hint time">{{ formatTime(entry.createdAt) }}</span>
    <div class="hist-actions">
      <n-button
        class="star-btn"
        :class="{ faved: entry.favorite }"
        quaternary
        circle
        size="small"
        :title="entry.favorite ? '取消收藏' : '收藏'"
        :aria-label="entry.favorite ? '取消收藏' : '收藏'"
        @click="emit('favorite')"
      >
        <AppIcon name="star" :size="16" />
      </n-button>
      <n-button
        quaternary
        circle
        size="small"
        title="载入到翻译"
        aria-label="载入到翻译"
        @click="emit('load')"
      >
        <AppIcon name="upload" :size="16" />
      </n-button>
      <n-button
        quaternary
        circle
        size="small"
        :disabled="!entry.result"
        title="复制译文"
        aria-label="复制译文"
        @click="emit('copy')"
      >
        <AppIcon name="copy" :size="16" />
      </n-button>
      <n-button
        v-if="!compact"
        quaternary
        circle
        size="small"
        :loading="retranslating"
        :disabled="retranslating"
        title="重译"
        aria-label="重译"
        @click="emit('retranslate')"
      >
        <AppIcon v-if="!retranslating" name="refresh" :size="16" />
      </n-button>
      <n-popconfirm
        v-if="!compact"
        positive-text="删除"
        negative-text="取消"
        @positive-click="emit('delete')"
      >
        <template #trigger>
          <n-button
            quaternary
            circle
            size="small"
            type="error"
            title="删除"
            aria-label="删除"
          >
            <AppIcon name="trash" :size="16" />
          </n-button>
        </template>
        删除这条翻译记录？
      </n-popconfirm>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NPopconfirm } from "naive-ui";
import type { HistoryEntry } from "../../types";
import AppIcon from "../AppIcon.vue";

const props = defineProps<{ entry: HistoryEntry; compact?: boolean; retranslating?: boolean }>();

const emit = defineEmits<{
  (e: "load"): void;
  (e: "retranslate"): void;
  (e: "favorite"): void;
  (e: "copy"): void;
  (e: "delete"): void;
}>();

const directionLabel = computed(() => {
  const { source, target, direction } = props.entry;
  if (source === "zh" && target === "en") return "中译英";
  if (source === "en" && target === "zh") return "英译中";
  if (direction === "zh2en") return "中译英";
  if (direction === "en2zh") return "英译中";
  return "自动";
});

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

/** 近期用「今天 / 昨天」，同年用月日，跨年补全年份。 */
function formatTime(iso: string): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const clock = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  const now = new Date();
  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const time = date.getTime();
  if (time >= todayStart) return `今天 ${clock}`;
  if (time >= todayStart - 24 * 60 * 60 * 1000) return `昨天 ${clock}`;
  const day = `${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  if (date.getFullYear() !== now.getFullYear()) return `${date.getFullYear()}-${day}`;
  return `${day} ${clock}`;
}
</script>

<style scoped>
.hist-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}

.hist-row:last-child {
  border-bottom: 0;
}

.dir-tag {
  flex: none;
  min-width: 52px;
  justify-content: center;
}

.hist-texts {
  flex: 1 1 160px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.src,
.dst {
  font-size: var(--fs-13);
  line-height: 1.6;
}

.src {
  color: var(--text);
}

.dst {
  color: var(--muted);
}

.dst.empty {
  color: var(--faint);
}

.time {
  flex: none;
}

.hist-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex: none;
  margin-left: auto;
}

.star-btn {
  color: var(--faint);
}

.star-btn.faved {
  color: var(--warning);
}

.star-btn.faved :deep(.ic use) {
  fill: currentColor;
}
</style>
