<template>
  <label class="field">
    <span class="field-label">{{ label }}</span>
    <n-input
      :value="modelValue"
      :type="password ? 'password' : 'text'"
      :show-password-on="password ? 'click' : undefined"
      :placeholder="placeholder"
      autocomplete="off"
      @update:value="emit('update:modelValue', $event)"
    />
  </label>
</template>

<script setup lang="ts">
import { NInput } from "naive-ui";

defineProps<{
  label: string;
  modelValue: string;
  placeholder?: string;
  password?: boolean;
}>();

const emit = defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.field-label {
  font-size: var(--fs-12);
  font-weight: 500;
  color: var(--text-2);
}
</style>
