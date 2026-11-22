<template>
  <button
    ref="root"
    type="button"
    class="shortcut-input mono"
    :class="{ capturing, muted: disabled }"
    :disabled="disabled"
    :title="disabled ? disabledHint : '点击后按下新的组合键'"
    :aria-label="`${label}快捷键，当前 ${displayText || '未设置'}，点击后按下新组合键`"
    @click="begin"
    @keydown="onKeydown"
    @blur="end"
  >
    <span v-if="capturing" class="placeholder">{{ captureHint }}</span>
    <span v-else-if="displayText" class="combo">{{ displayText }}</span>
    <span v-else class="placeholder">点击后按下组合键</span>
  </button>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { store } from "../../store";
import {
  acceleratorFromCombo,
  formatModifierPreview,
  formatShortcut,
  hasModifier,
  isModifierCode,
  type KeyCombo,
} from "../../utils/shortcut";

const props = withDefaults(
  defineProps<{ modelValue: string; disabled?: boolean; label?: string; disabledHint?: string }>(),
  { disabled: false, label: "快捷键", disabledHint: "当前不可配置" },
);
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const root = ref<HTMLButtonElement | null>(null);
const capturing = ref(false);
const hint = ref("");
const preview = ref("");

const platform = computed(() => store.platform || "windows");
const displayText = computed(() => formatShortcut(props.modelValue, platform.value));
const captureHint = computed(() => hint.value || (preview.value ? `${preview.value} + …` : "请按下组合键…"));

function begin(): void {
  if (props.disabled || capturing.value) return;
  capturing.value = true;
  hint.value = "";
  preview.value = "";
  root.value?.focus();
}

function end(): void {
  capturing.value = false;
  hint.value = "";
  preview.value = "";
}

function onKeydown(event: KeyboardEvent): void {
  if (!capturing.value) return;
  event.preventDefault();
  event.stopPropagation();

  if (event.code === "Escape") {
    end();
    return;
  }
  if (isModifierCode(event.code)) {
    preview.value = formatModifierPreview(event, platform.value);
    return;
  }
  if ((event.code === "Backspace" || event.code === "Delete") && !hasModifier(event)) {
    emit("update:modelValue", "");
    end();
    return;
  }

  const accelerator = acceleratorFromCombo(event as KeyCombo, platform.value);
  if (!accelerator) {
    hint.value = hasModifier(event)
      ? "这个按键不支持，请换一个"
      : "需要同时按住 Ctrl / Alt / Shift / Win";
    return;
  }
  emit("update:modelValue", accelerator);
  end();
}
</script>

<style scoped>
.shortcut-input {
  width: 170px;
  height: 34px;
  padding: 0 11px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  overflow: hidden;
  white-space: nowrap;
  color: var(--text);
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-sm);
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.shortcut-input:hover:not(:disabled) {
  border-color: var(--primary);
}

.shortcut-input:focus-visible,
.shortcut-input.capturing {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px var(--primary-ring);
  outline: none;
}

.shortcut-input:disabled,
.shortcut-input.muted {
  color: var(--faint);
  background: var(--surface-soft);
  cursor: not-allowed;
}

.combo {
  overflow: hidden;
  text-overflow: ellipsis;
}

.placeholder {
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
}

@media (max-width: 760px) {
  .shortcut-input {
    width: 100%;
  }
}
</style>
