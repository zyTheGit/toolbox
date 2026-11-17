<template>
  <div class="result-card">
    <div class="result-head">
      <span class="title">结果</span>
      <span v-if="result" class="meta">
        {{ rangeText }} · {{ result.elapsedMs }} ms
      </span>
    </div>
    <n-spin :show="loading">
      <div v-if="!result" class="placeholder">执行查询后在这里查看结果</div>
      <n-empty v-else-if="!result.rows.length" size="small" description="没有数据" />
      <template v-else>
        <div class="table-wrap">
          <div class="row head">
            <div
              v-for="(col, index) in displayColumns"
              :key="`${col.name}-${index}`"
              class="cell"
              :style="{ width: widthOf(col.name) + 'px' }"
            >
              <span class="col-label">{{ col.name }}</span>
              <span class="resize" @pointerdown="startResize($event, col.name)"></span>
            </div>
          </div>
          <div v-for="(row, rowIndex) in result.rows" :key="rowIndex" class="row">
            <div
              v-for="(cell, cellIndex) in row"
              :key="cellIndex"
              class="cell"
              :style="{ width: widthOf(displayColumns[cellIndex]?.name ?? String(cellIndex)) + 'px' }"
            >
              <span v-if="cell === null" class="null">NULL</span>
              <span v-else class="value" :title="String(cell)">{{ cell }}</span>
            </div>
          </div>
        </div>
        <div class="pager">
          <n-button size="tiny" :disabled="result.offset === 0" @click="page(-1)">上一页</n-button>
          <n-button size="tiny" :disabled="!result.hasMore" @click="page(1)">下一页</n-button>
          <span class="pager-hint">每页 {{ result.limit }} 行</span>
        </div>
      </template>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { NButton, NEmpty, NSpin } from "naive-ui";
import type { DbKind, DbQueryResult } from "../../types";

const props = defineProps<{
  result: DbQueryResult | null;
  kind: DbKind | "";
  loading: boolean;
}>();

const emit = defineEmits<{
  page: [offset: number];
}>();

const STORAGE_KEY = "toolbox.db.columnWidths";
const widths = ref<Record<string, number>>(readWidths());
let drag: { key: string; startX: number; startWidth: number } | null = null;

const displayColumns = computed(() => {
  if (!props.result) return [];
  if (props.result.columns.length) {
    return props.result.columns.map((name) => ({ name }));
  }
  const width = props.result.rows[0]?.length ?? 0;
  return Array.from({ length: width }, (_, index) => ({ name: `列 ${index + 1}` }));
});

const rangeText = computed(() => {
  const result = props.result;
  if (!result || !result.rows.length) return "0 行";
  return `第 ${result.offset + 1}–${result.offset + result.rows.length} 行`;
});

watch(
  widths,
  (value) => localStorage.setItem(STORAGE_KEY, JSON.stringify(value)),
  { deep: true },
);

function readWidths(): Record<string, number> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Record<string, number>) : {};
  } catch {
    return {};
  }
}

function widthOf(name: string): number {
  return widths.value[`${props.kind}:${name}`] ?? 168;
}

function startResize(event: PointerEvent, name: string): void {
  event.preventDefault();
  drag = { key: `${props.kind}:${name}`, startX: event.clientX, startWidth: widthOf(name) };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", stopResize, { once: true });
}

function onMove(event: PointerEvent): void {
  if (!drag) return;
  const next = drag.startWidth + event.clientX - drag.startX;
  widths.value = { ...widths.value, [drag.key]: Math.max(64, Math.min(640, Math.round(next))) };
}

function stopResize(): void {
  drag = null;
  window.removeEventListener("pointermove", onMove);
}

function page(direction: number): void {
  const result = props.result;
  if (!result) return;
  const offset = Math.max(0, result.offset + direction * result.limit);
  emit("page", offset);
}

onBeforeUnmount(stopResize);
</script>

<style scoped>
.result-card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 14px;
  min-width: 0;
}
.result-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 10px;
}
.title {
  font-size: 13px;
  font-weight: 600;
  color: #333b4d;
}
.meta {
  font-size: 12px;
  color: var(--muted);
}
.placeholder {
  padding: 24px 8px;
  text-align: center;
  color: var(--muted);
  font-size: 13px;
}
.table-wrap {
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  max-height: calc(100vh - 430px);
  min-height: 120px;
}
.row {
  display: flex;
  min-width: max-content;
}
.row + .row {
  border-top: 1px solid var(--border);
}
.row.head {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--panel-soft);
  border-bottom: 1px solid var(--border);
}
.cell {
  position: relative;
  flex: none;
  padding: 6px 10px;
  font-size: 12px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.row.head .cell {
  font-weight: 600;
  color: #333b4d;
  user-select: none;
}
.resize {
  position: absolute;
  top: 0;
  right: 0;
  width: 6px;
  height: 100%;
  cursor: col-resize;
}
.resize:hover {
  background: var(--primary-soft);
}
.null {
  color: #a6adbd;
  font-style: italic;
}
.pager {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}
.pager-hint {
  font-size: 12px;
  color: var(--muted);
}
</style>
