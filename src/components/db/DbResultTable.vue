<template>
  <div class="card result-card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="list" :size="16" />
        查询结果
      </h3>
      <span v-if="result" class="hint">{{ metaText }}</span>
    </div>
    <n-spin :show="loading">
      <div v-if="!result" class="result-placeholder">
        <EmptyState
          icon="play"
          title="暂无查询结果"
          description="在左侧点选表，或输入 SQL 后执行查询。"
        />
      </div>
      <div v-else-if="!result.rows.length" class="result-placeholder">
        <EmptyState title="没有数据" description="查询成功，但没有返回任何行。" />
      </div>
      <template v-else>
        <div class="table-wrap">
          <table class="tbl" :style="{ width: `${tableWidth}px` }">
            <colgroup>
              <col
                v-for="(col, index) in displayColumns"
                :key="`${col.name}-${index}`"
                :style="{ width: `${widthOf(col.name)}px` }"
              />
            </colgroup>
            <thead>
              <tr>
                <th v-for="(col, index) in displayColumns" :key="`${col.name}-${index}`">
                  <span class="th-label" :title="col.name">{{ col.name }}</span>
                  <span class="resize" @pointerdown="startResize($event, col.name)"></span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, rowIndex) in result.rows" :key="rowIndex">
                <td
                  v-for="(cell, cellIndex) in row"
                  :key="cellIndex"
                  :class="{ num: typeof cell === 'number' }"
                >
                  <span v-if="cell === null" class="null">NULL</span>
                  <span v-else class="value" :title="String(cell)">{{ cell }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="result-foot">
          <span>NULL 以灰色斜体显示 · 数字列等宽字体</span>
          <div class="pager">
            <n-button
              size="small"
              class="pager-btn"
              title="上一页"
              :disabled="loading || result.offset === 0"
              @click="page(-1)"
            >
              <AppIcon name="chevron-left" :size="16" />
            </n-button>
            <span class="page-label">第 {{ pageNo }} 页</span>
            <n-button
              size="small"
              class="pager-btn"
              title="下一页"
              :disabled="loading || !result.hasMore"
              @click="page(1)"
            >
              <AppIcon name="chevron-right" :size="16" />
            </n-button>
          </div>
        </div>
      </template>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { NButton, NSpin } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
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

const pageNo = computed(() =>
  props.result ? Math.floor(props.result.offset / props.result.limit) + 1 : 1,
);

const metaText = computed(() => {
  const result = props.result;
  if (!result) return "";
  if (!result.rows.length) return `0 行 · ${result.elapsedMs} ms`;
  const more = result.hasMore ? "（还有更多）" : "";
  return `第 ${result.offset + 1}–${result.offset + result.rows.length} 行${more} · ${result.elapsedMs} ms`;
});

const tableWidth = computed(() =>
  displayColumns.value.reduce((total, col) => total + widthOf(col.name), 0),
);

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
  min-width: 0;
}
.table-wrap {
  max-height: calc(100vh - 430px);
  min-height: 120px;
  overflow: auto;
}
.tbl {
  table-layout: fixed;
  min-width: 100%;
}
.tbl th {
  position: sticky;
  top: 0;
  z-index: 1;
  padding: 0;
  background: var(--surface-soft);
}
.th-label {
  display: block;
  padding: 8px 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tbl td {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
.result-placeholder {
  padding: 12px 0;
}
.result-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 12px;
  font-size: var(--fs-12);
  color: var(--muted);
}
.pager {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pager-btn {
  width: 28px;
  padding: 0;
}
.page-label {
  color: var(--text-2);
}
</style>
