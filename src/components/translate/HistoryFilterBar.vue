<template>
  <div class="hist-filters">
    <n-input
      class="search"
      :value="keyword"
      clearable
      placeholder="搜索原文或译文"
      aria-label="搜索翻译历史"
      @update:value="onKeywordChange"
    />
    <n-select
      class="dir-select"
      :value="direction"
      :options="directionOptions"
      aria-label="按方向筛选"
      @update:value="onDirectionChange"
    />
    <n-select
      class="provider-select"
      :value="provider"
      :options="providerOptions"
      aria-label="按服务筛选"
      @update:value="onProviderChange"
    />
    <n-checkbox
      :checked="favoritesOnly"
      size="small"
      @update:checked="emit('update:favoritesOnly', $event)"
    >
      仅收藏
    </n-checkbox>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NCheckbox, NInput, NSelect } from "naive-ui";
import { store } from "../../store";

defineProps<{
  keyword: string;
  direction: string;
  provider: string;
  favoritesOnly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:keyword", value: string): void;
  (e: "update:direction", value: string): void;
  (e: "update:provider", value: string): void;
  (e: "update:favoritesOnly", value: boolean): void;
}>();

const directionOptions = [
  { label: "全部方向", value: "" },
  { label: "自动识别", value: "auto" },
  { label: "中译英", value: "zh2en" },
  { label: "英译中", value: "en2zh" },
];

const providerOptions = computed(() => [
  { label: "全部服务", value: "" },
  ...store.providers.map((item) => ({ label: item.name, value: item.id as string })),
]);

type SelectValue = string | number | Array<string | number> | null;

function keyOf(value: SelectValue): string {
  if (Array.isArray(value)) return value.length ? String(value[0]) : "";
  return value === null ? "" : String(value);
}

function onKeywordChange(value: string): void {
  emit("update:keyword", value);
}

function onDirectionChange(value: SelectValue): void {
  emit("update:direction", keyOf(value));
}

function onProviderChange(value: SelectValue): void {
  emit("update:provider", keyOf(value));
}
</script>

<style scoped>
.hist-filters {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}

.search {
  flex: 1 1 180px;
  min-width: 0;
}

.dir-select {
  width: 130px;
  flex: none;
}

.provider-select {
  width: 140px;
  flex: none;
}

@media (max-width: 560px) {
  .dir-select,
  .provider-select {
    width: calc(50% - 4px);
  }
}
</style>
