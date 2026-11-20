<template>
  <div class="card schema-card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="table" :size="16" />
        结构
        <span v-if="database" class="tag tag-primary db-tag" :title="database">{{ database }}</span>
      </h3>
      <span class="hint">{{ countText }}</span>
    </div>
    <n-input
      v-model:value="keyword"
      class="schema-search"
      size="small"
      clearable
      placeholder="搜索表 / 字段"
    />

    <div v-if="!filteredTables.length" class="empty tree-empty">
      <AppIcon name="search" :size="20" />
      <b>{{ emptyText }}</b>
    </div>
    <div v-else class="tree">
      <div
        v-for="table in filteredTables"
        :key="table.name"
        class="tree-item"
        :class="{ open: isOpen(table), active: table.name === active }"
      >
        <div class="tree-head" :title="table.name" @click="onHead(table)">
          <AppIcon name="chevron-right" :size="16" class="chev" />
          <span class="tree-badge" :class="{ view: table.kind === 'view' }">
            {{ table.kind === "view" ? "V" : "T" }}
          </span>
          <span class="name ellipsis">{{ table.name }}</span>
          <span class="hint">{{ table.columns.length }}</span>
        </div>
        <div v-if="isOpen(table)" class="tree-cols">
          <div v-for="col in columnsOf(table)" :key="col.name" class="tree-col">
            <AppIcon v-if="col.primaryKey" name="key" :size="16" class="key" />
            <span v-else class="key-slot"></span>
            <span class="col-name ellipsis" :title="col.name">{{ col.name }}</span>
            <span class="type">{{ col.dataType }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { NInput } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import type { DbTable } from "../../types";

const props = defineProps<{
  tables: DbTable[];
  active: string;
  /** 当前库名，展示在标题标签上 */
  database?: string;
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

const tableCount = computed(() => props.tables.filter((table) => table.kind === "table").length);
const viewCount = computed(() => props.tables.length - tableCount.value);

const countText = computed(() => {
  if (searching.value) return `${filteredTables.value.length} / ${props.tables.length} 个对象`;
  return `${tableCount.value} 表${viewCount.value ? ` · ${viewCount.value} 视图` : ""}`;
});

const emptyText = computed(() =>
  searching.value ? "没有匹配的表或字段" : props.emptyHint || "没有可浏览的表",
);

// 搜索结果过多时全部展开会生成成千上万个节点，此时保持折叠由用户按需展开
const expandAll = computed(() => searching.value && filteredTables.value.length <= 30);

function matches(text: string): boolean {
  return text.toLowerCase().includes(query.value);
}

function isOpen(table: DbTable): boolean {
  if (expandAll.value) return true;
  return !!expanded[table.name];
}

function columnsOf(table: DbTable): DbTable["columns"] {
  if (!searching.value || matches(table.name)) return table.columns;
  return table.columns.filter((col) => matches(col.name));
}

/** 点击表头：展开并执行默认查询；再次点击已展开的当前表则收起。 */
function onHead(table: DbTable): void {
  if (isOpen(table) && !expandAll.value) {
    expanded[table.name] = false;
    return;
  }
  expanded[table.name] = true;
  if (table.name !== props.active) emit("select", table);
}
</script>

<style scoped>
.schema-card {
  margin-top: 0;
}
.db-tag {
  max-width: 130px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.schema-search {
  margin-bottom: 8px;
}
.tree {
  max-height: 420px;
  overflow-y: auto;
  margin: 0 -4px;
  padding: 0 4px;
}
.tree-item {
  border-radius: 7px;
}
.tree-head {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 8px;
  border-radius: 7px;
  cursor: pointer;
  font-size: var(--fs-13);
  color: var(--text-2);
}
.tree-head:hover {
  background: var(--surface-soft);
}
.tree-item.active > .tree-head {
  background: var(--primary-soft);
  color: var(--primary);
  font-weight: 600;
}
.chev {
  width: 14px;
  height: 14px;
  color: var(--faint);
}
.tree-item.open > .tree-head .chev {
  transform: rotate(90deg);
}
.tree-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  flex: none;
  border-radius: 5px;
  background: var(--primary-soft);
  color: var(--primary);
  font-size: 11px;
  font-weight: 700;
}
.tree-badge.view {
  background: var(--info-soft);
  color: var(--info);
}
.name {
  flex: 1;
  min-width: 0;
}
.tree-cols {
  padding: 2px 0 6px 30px;
}
.tree-col {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
  font-size: var(--fs-12);
  color: var(--muted);
}
.key {
  width: 12px;
  height: 12px;
  color: var(--warning);
}
.key-slot {
  width: 12px;
  flex: none;
}
.col-name {
  min-width: 0;
}
.type {
  margin-left: auto;
  flex: none;
  font-family: var(--font-mono);
  color: var(--faint);
}
.tree-empty {
  padding: 24px 8px;
}
.tree-empty .ic {
  width: 20px;
  height: 20px;
  margin-bottom: 2px;
}
</style>
