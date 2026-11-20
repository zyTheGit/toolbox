<template>
  <div class="database-select" :title="selectedText">
    <n-select
      :value="value || null"
      size="small"
      filterable
      :consistent-menu-width="false"
      :options="options"
      :loading="loading"
      :disabled="disabled"
      :placeholder="placeholder"
      @update:value="onSelect"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NSelect } from "naive-ui";
import type { DbDatabase } from "../../types";

const props = withDefaults(
  defineProps<{
    databases: DbDatabase[];
    value: string;
    loading?: boolean;
    disabled?: boolean;
    placeholder?: string;
  }>(),
  { placeholder: "选择数据库" },
);

const emit = defineEmits<{
  "update:value": [value: string];
}>();

const options = computed(() =>
  props.databases.map((db) => ({
    label: `${db.name}（${db.tableCount} 个对象）`,
    value: db.name,
  })),
);

const selectedText = computed(
  () => options.value.find((option) => option.value === props.value)?.label ?? props.placeholder,
);

function onSelect(value: string | number | null): void {
  if (typeof value === "string" && value !== props.value) emit("update:value", value);
}
</script>

<style scoped>
.database-select {
  flex: none;
  width: 170px;
}
</style>
