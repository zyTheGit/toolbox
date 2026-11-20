<template>
  <div class="save-bar" :class="{ dirty }">
    <span class="state">
      <AppIcon :name="dirty ? 'alert' : 'check'" :size="16" />
      {{ dirty ? "有未保存的修改" : "所有修改已保存" }}
    </span>
    <div class="row">
      <n-button quaternary :disabled="!dirty || saving" @click="emit('discard')">放弃修改</n-button>
      <n-button
        type="primary"
        :loading="saving"
        :disabled="!dirty || saving"
        @click="emit('save')"
      >
        <template #icon><AppIcon name="check" :size="16" /></template>
        保存设置
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton } from "naive-ui";
import AppIcon from "../AppIcon.vue";

defineProps<{ dirty: boolean; saving: boolean }>();

const emit = defineEmits<{ discard: []; save: [] }>();
</script>

<style scoped>
.row {
  flex: none;
  justify-content: flex-end;
}
</style>
