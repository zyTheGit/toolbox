<template>
  <section class="card tr-pane">
    <div class="spread pane-top">
      <div class="seg" role="group" aria-label="翻译方向">
        <button
          v-for="option in DIRECTION_OPTIONS"
          :key="option.value"
          type="button"
          :class="{ on: direction === option.value }"
          :aria-pressed="direction === option.value"
          @click="emit('update:direction', option.value)"
        >
          {{ option.label }}
        </button>
      </div>
      <n-button
        quaternary
        circle
        size="small"
        :disabled="loading"
        title="交换翻译方向（自动时按文本推断）"
        aria-label="交换翻译方向"
        @click="emit('swap')"
      >
        <AppIcon name="swap" :size="16" />
      </n-button>
    </div>

    <textarea
      class="src-textarea"
      :value="input"
      :disabled="loading"
      :placeholder="`输入要翻译的内容，${submitLabel} 翻译`"
      aria-label="要翻译的原文"
      @input="onInput"
      @keydown="onKeydown"
    ></textarea>

    <div class="pane-foot">
      <span class="hint foot-hint">
        {{ input.length }} 字符<template v-if="!loading"> · {{ submitLabel }} 翻译</template>
        <n-button
          v-if="undoAvailable"
          text
          type="primary"
          size="tiny"
          class="undo-btn"
          @click="emit('undo')"
        >
          撤销清空
        </n-button>
      </span>
      <div class="foot-actions">
        <n-button
          quaternary
          circle
          size="small"
          :disabled="loading || !input.trim()"
          :title="speaking ? '停止朗读' : '朗读原文'"
          :aria-label="speaking ? '停止朗读' : '朗读原文'"
          @click="emit('speak')"
        >
          <AppIcon :name="speaking ? 'pause' : 'volume'" :size="16" />
        </n-button>
        <n-button quaternary size="small" :disabled="loading || !input" @click="emit('clear')">
          <template #icon><AppIcon name="x" :size="16" /></template>
          清空
        </n-button>
        <n-button
          type="primary"
          :loading="loading"
          :disabled="loading || !input.trim()"
          @click="emit('translate')"
        >
          <template #icon><AppIcon name="zap" :size="16" /></template>
          翻译
        </n-button>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { useShortcuts } from "../../composables/useShortcuts";

defineProps<{
  input: string;
  direction: string;
  loading: boolean;
  speaking: boolean;
  undoAvailable: boolean;
}>();

const emit = defineEmits<{
  (e: "update:input", value: string): void;
  (e: "update:direction", value: string): void;
  (e: "translate"): void;
  (e: "clear"): void;
  (e: "undo"): void;
  (e: "speak"): void;
  (e: "swap"): void;
}>();

const DIRECTION_OPTIONS = [
  { label: "自动检测", value: "auto" },
  { label: "中译英", value: "zh2en" },
  { label: "英译中", value: "en2zh" },
];

const { label, matches } = useShortcuts();
const submitLabel = computed(() => label("translateSubmit"));

function onInput(event: Event): void {
  emit("update:input", (event.target as HTMLTextAreaElement).value);
}

function onKeydown(event: KeyboardEvent): void {
  if (!matches(event, "translateSubmit")) return;
  event.preventDefault();
  emit("translate");
}
</script>

<style scoped>
.tr-pane {
  display: flex;
  flex-direction: column;
  min-height: 320px;
  min-width: 0;
}

.pane-top {
  gap: 10px;
  margin-bottom: 10px;
}

.pane-top .seg {
  min-width: 0;
}

.src-textarea {
  flex: 1;
  width: 100%;
  min-height: 180px;
  padding: 4px 2px;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  line-height: 1.7;
  resize: none;
}

.src-textarea::placeholder {
  color: var(--faint);
}

.src-textarea:focus,
.src-textarea:focus-visible {
  outline: none;
}

.src-textarea:disabled {
  color: var(--faint);
  cursor: not-allowed;
}

.pane-foot {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  padding-top: 12px;
  margin-top: 12px;
  border-top: 1px solid var(--border);
}

.foot-hint {
  flex: 1 1 140px;
  min-width: 0;
}

.undo-btn {
  margin-left: 8px;
}

.foot-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}
</style>
