<template>
  <section class="card tr-pane">
    <div class="out-meta">
      <span v-if="providerName" class="tag tag-primary">{{ providerName }}</span>
      <span class="hint meta-text" :title="metaText">{{ metaText }}</span>
    </div>

    <n-alert
      v-if="error"
      class="result-error"
      type="error"
      :bordered="false"
      closable
      @close="emit('close-error')"
    >
      {{ error }}
    </n-alert>

    <div v-if="loading" class="out-body body-center">
      <n-spin size="small" />
      <span class="muted">正在翻译…</span>
    </div>
    <div v-else-if="result" class="out-body out-text">{{ result.text }}</div>
    <div v-else class="out-body body-empty">
      <EmptyState
        icon="translate"
        title="译文将显示在这里"
        :description="`输入内容后按 ${submitLabel} 翻译。`"
      />
    </div>

    <div class="pane-foot">
      <span class="hint foot-hint ellipsis" :title="footHint">{{ footHint }}</span>
      <div class="foot-actions">
        <n-button
          quaternary
          circle
          size="small"
          :disabled="!result || loading"
          :title="speaking ? '停止朗读' : '朗读译文'"
          :aria-label="speaking ? '停止朗读' : '朗读译文'"
          @click="emit('speak')"
        >
          <AppIcon :name="speaking ? 'pause' : 'volume'" :size="16" />
        </n-button>
        <n-button size="small" :disabled="!result || loading" @click="emit('copy')">
          <template #icon><AppIcon name="copy" :size="16" /></template>
          复制译文
        </n-button>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NAlert, NButton, NSpin } from "naive-ui";
import type { TranslateResult } from "../../types";
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
import { useShortcuts } from "../../composables/useShortcuts";

defineProps<{
  result: TranslateResult | null;
  loading: boolean;
  error: string;
  providerName: string;
  metaText: string;
  footHint: string;
  speaking: boolean;
}>();

const { label } = useShortcuts();
const submitLabel = computed(() => label("translateSubmit"));

const emit = defineEmits<{
  (e: "close-error"): void;
  (e: "speak"): void;
  (e: "copy"): void;
}>();
</script>

<style scoped>
.tr-pane {
  display: flex;
  flex-direction: column;
  min-height: 320px;
  min-width: 0;
}

.out-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  min-height: 22px;
  margin-bottom: 10px;
}

.meta-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-error {
  flex: none;
  margin-bottom: 10px;
}

.out-body {
  flex: 1;
  min-height: 180px;
}

.out-text {
  padding: 4px 2px;
  color: var(--text);
  font-size: var(--fs-14);
  line-height: 1.75;
  white-space: pre-wrap;
  word-break: break-word;
  overflow: auto;
  user-select: text;
}

.body-center {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
}

.body-empty {
  display: flex;
  align-items: center;
  justify-content: center;
}

.body-empty :deep(.empty) {
  padding: 24px 12px;
}

.pane-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 12px;
  margin-top: 12px;
  border-top: 1px solid var(--border);
}

.foot-hint {
  flex: 1;
  min-width: 0;
}

.foot-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;
}
</style>
