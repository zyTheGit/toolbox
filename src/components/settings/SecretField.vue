<template>
  <label class="field">
    <span class="field-label">
      <span>{{ label }}</span>
      <n-tag v-if="cleared" size="tiny" type="warning" :bordered="false">保存后清除</n-tag>
      <n-tag v-else-if="stored" size="tiny" type="success" :bordered="false">已保存</n-tag>
    </span>
    <div class="row">
      <n-input
        :value="modelValue"
        type="password"
        show-password-on="click"
        :placeholder="cleared ? '保存后清除密钥' : stored ? '已保存，留空保持不变' : placeholder"
        autocomplete="off"
        @update:value="emit('update:modelValue', $event)"
      />
      <n-button
        v-if="stored && !cleared"
        size="small"
        quaternary
        type="error"
        @click="emit('clear')"
      >
        清除
      </n-button>
    </div>
  </label>
</template>

<script setup lang="ts">
import { NButton, NInput, NTag } from "naive-ui";

defineProps<{
  label: string;
  modelValue: string;
  stored: boolean;
  cleared: boolean;
  placeholder?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  clear: [];
}>();
</script>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.field-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
  color: #4b5468;
}

.row {
  display: flex;
  align-items: center;
  gap: 6px;
}
</style>
