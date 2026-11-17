<template>
  <div class="schema-card">
    <div class="schema-head">
      <span class="title">结构</span>
      <span class="count">{{ countText }}</span>
    </div>
    <n-input
      v-model:value="keyword"
      class="schema-search"
      size="small"
      clearable
      placeholder="搜索表 / 字段"
    />
    <n-empty v-if="!filteredTables.length" size="small" :description="emptyText" />
    <div v-else class="schema-list">
      <div v-for="table in filteredTables" :key="table.name" class="table-block">
        <div
          class="table-item"
          :class="{ active: table.name === active }"
          @click="emit('select', table)"
        >
          <span v-if="expandAll" class="chev placeholder"></span>
          <span v-else class="chev" @click.stop="toggle(table.name)">
            {{ expanded[table.name] ? "▾" : "▸" }}
          </span>
          <span class="kind" :class="table.kind">{{ table.kind === "view" ? "V" : "T" }}</span>
          <span class="name" :title="table.name">{{ table.name }}</span>
          <span class="count">{{ table.columns.length }}</span>
        </div>
        <div v-if="expandAll || expanded[table.name]" class="cols">
          <div v-for="col in columnsOf(table)" :key="col.name" class="col">
            <span class="col-name" :title="col.name">{{ col.name }}</span>
            <span class="col-type">{{ col.dataType }}</span>
            <span v-if="col.primaryKey" class="pk">PK</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { NEmpty, NInput } from "naive-ui";
import type { DbTable } from "../../types";

const props = defineProps<{
  tables: DbTable[];
  active: string;
  emptyHint?: string;
}>();

const emit = defineEmits<{
  select: [table: DbTable];
}>();

const expanded = reactive<Record<string, boolean>>({});
const keyword = ref("");
const query = computed(() => keyword.value.trim().toLowerCase());
const searching = computed(() => query.value.length > 0);

const filteredTables = computed(() => {
  if (!query.value) return props.tables;
  return props.tables.filter(
    (table) => matches(table.name) || table.columns.some((col) => matches(col.name)),
  );
});

const countText = computed(() =>
  searching.value
    ? `${filteredTables.value.length} / ${props.tables.length} 个对象`
    : `${props.tables.length} 个对象`,
);

const emptyText = computed(() =>
  searching.value ? "没有匹配的表或字段" : props.emptyHint || "没有可浏览的表",
);

// 搜索结果过多时全部展开会生成成千上万个节点，此时保持折叠由用户按需展开
const expandAll = computed(() => searching.value && filteredTables.value.length <= 30);

function matches(text: string): boolean {
  return text.toLowerCase().includes(query.value);
}

function columnsOf(table: DbTable): DbTable["columns"] {
  if (!searching.value || matches(table.name)) return table.columns;
  return table.columns.filter((col) => matches(col.name));
}

function toggle(name: string): void {
  expanded[name] = !expanded[name];
}
</script>

<style scoped>
.schema-card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 14px;
}
.schema-head {
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
.count {
  font-size: 12px;
  color: var(--muted);
}
.schema-search {
  margin-bottom: 8px;
}
.schema-list {
  max-height: calc(100vh - 360px);
  overflow: auto;
  padding-right: 4px;
}
.table-block + .table-block {
  margin-top: 2px;
}
.table-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}
.table-item:hover {
  background: var(--panel-soft);
}
.table-item.active {
  background: var(--primary-soft);
  color: var(--primary);
}
.chev {
  width: 12px;
  color: var(--muted);
  font-size: 11px;
  flex: none;
}
.kind {
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
  flex: none;
  background: #eef1ff;
  color: #4f63e7;
}
.kind.view {
  background: #eefaf3;
  color: #1f9d6b;
}
.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cols {
  padding: 2px 8px 6px 34px;
}
.col {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
  font-size: 12px;
  color: var(--muted);
}
.col-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.col-type {
  margin-left: auto;
  font-size: 11px;
  color: #9aa1b2;
  flex: none;
}
.pk {
  font-size: 10px;
  padding: 0 4px;
  border-radius: 3px;
  background: #fff4e5;
  color: #b7791f;
  flex: none;
}
</style>
