<template>
  <div class="database-select" :title="selectedLabel">
    <n-select
      :value="value"
      size="tiny"
      filterable
      :consistent-menu-width="false"
      :style="{ width: selectWidth }"
      :options="options"
      :loading="loading"
      placeholder="选择或搜索数据库"
      @update:value="onSelect"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NSelect } from "naive-ui";
import type { DbDatabase } from "../../types";

const props = defineProps<{
  databases: DbDatabase[];
  value: string;
  loading?: boolean;
}>();

const emit = defineEmits<{
  "update:value": [value: string];
}>();

const options = computed(() =>
  props.databases.map((db) => ({
    label: `${db.name}（${db.tableCount} 个对象）`,
    value: db.name,
  })),
);

const selectedLabel = computed(
  () => options.value.find((option) => option.value === props.value)?.label ?? "选择数据库",
);

// 库名长度差异很大，按文本宽度给选择器定宽，避免长库名被截断
const selectWidth = computed(() => {
  let width = 0;
  for (const ch of selectedLabel.value) {
    width += ch.charCodeAt(0) > 0xff ? 12 : 7;
  }
  return `${Math.min(380, Math.max(190, width + 44))}px`;
});

function onSelect(value: string | number | null): void {
  if (typeof value === "string") emit("update:value", value);
}
</script>

<style scoped>
.database-select {
  flex: none;
}
</style>
