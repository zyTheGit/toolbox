<template>
  <div class="card query-card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="terminal" :size="16" />
        SQL 查询
        <span class="tag tag-primary">{{ dbKindLabel(kind) }}</span>
      </h3>
      <div class="row limit-row">
        <span class="hint">行数</span>
        <n-select
          :value="limit"
          size="small"
          class="limit-select"
          :options="LIMIT_OPTIONS"
          :disabled="running"
          @update:value="onLimit"
        />
      </div>
    </div>
    <n-input
      :value="sql"
      type="textarea"
      class="sql-editor"
      :autosize="{ minRows: 4, maxRows: 12 }"
      placeholder="SELECT * FROM ..."
      spellcheck="false"
      @update:value="emit('update:sql', $event)"
      @keydown="onKeydown"
    />
    <div class="spread query-foot">
      <span class="hint">{{ runLabel }} 执行 · 仅允许只读语句</span>
      <n-button
        type="primary"
        size="small"
        :loading="running"
        :disabled="!sql.trim()"
        @click="emit('run')"
      >
        <template #icon>
          <AppIcon name="play" :size="16" />
        </template>
        执行
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NInput, NSelect } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { dbKindLabel } from "./sqlText";
import { useShortcuts } from "../../composables/useShortcuts";
import type { DbKind } from "../../types";

const props = defineProps<{
  kind: DbKind;
  sql: string;
  limit: number;
  running: boolean;
}>();

const emit = defineEmits<{
  "update:sql": [value: string];
  "update:limit": [value: number];
  run: [];
}>();

const LIMIT_OPTIONS = [50, 100, 200, 500].map((value) => ({
  label: `${value} 行`,
  value,
}));

function onLimit(value: string | number | null): void {
  if (typeof value === "number" && value !== props.limit) emit("update:limit", value);
}

const { label, matches } = useShortcuts();
const runLabel = computed(() => label("dbRun"));

function onKeydown(event: KeyboardEvent): void {
  if (!matches(event, "dbRun")) return;
  event.preventDefault();
  emit("run");
}
</script>

<style scoped>
.query-card {
  min-width: 0;
}
.limit-row {
  gap: 6px;
}
.limit-select {
  width: 88px;
}
.sql-editor :deep(textarea) {
  font-family: var(--font-mono);
  font-size: var(--fs-12);
  line-height: 1.6;
}
.query-foot {
  margin-top: 12px;
}
</style>
