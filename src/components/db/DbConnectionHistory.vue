<template>
  <n-button size="small" :disabled="!entries.length" @click="open = true">
    最近连接
    <span v-if="entries.length" class="count">{{ entries.length }}</span>
  </n-button>

  <n-drawer v-model:show="open" placement="right" width="min(400px, 92vw)">
    <n-drawer-content title="最近连接" closable>
      <n-input
        v-model:value="keyword"
        size="small"
        clearable
        placeholder="搜索名称或连接目标"
        class="search"
      />
      <div v-if="!entries.length" class="empty">连接成功后会出现在这里</div>
      <div v-else-if="!filtered.length" class="empty">没有匹配的记录</div>
      <div v-else class="list">
        <div v-for="entry in filtered" :key="entry.id" class="item">
          <div class="item-head">
            <span class="kind">{{ kindLabel(entry.kind) }}</span>
            <span class="date">{{ entry.updatedAt.slice(0, 10) }}</span>
          </div>
          <template v-if="editingId === entry.id">
            <n-input
              v-model:value="draftName"
              size="small"
              maxlength="60"
              placeholder="记录名称"
              :disabled="renaming"
              class="rename-input"
              @keydown.enter.prevent="saveRename(entry)"
              @keydown.escape.prevent="cancelRename"
            />
            <div class="edit-actions">
              <n-button
                size="tiny"
                type="primary"
                :loading="renaming"
                @click="saveRename(entry)"
              >
                保存
              </n-button>
              <n-button size="tiny" quaternary :disabled="renaming" @click="cancelRename">
                取消
              </n-button>
            </div>
          </template>
          <template v-else>
            <button
              type="button"
              class="fill"
              :disabled="revealing"
              @click="fill(entry)"
            >
              <span class="text">
                <span class="name" :title="entry.name">{{ entry.name }}</span>
                <span
                  v-if="entry.label && entry.label !== entry.name"
                  class="label"
                  :title="entry.label"
                >
                  {{ entry.label }}
                </span>
              </span>
              <span class="action">回填</span>
            </button>
            <button type="button" class="rename" @click="startRename(entry)">改名</button>
          </template>
        </div>
      </div>
      <p class="hint">点击记录会把完整连接串回填到上方输入框，连接串包含密码。</p>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NButton, NDrawer, NDrawerContent, NInput } from "naive-ui";
import { api } from "../../api";
import { describeError } from "../../utils/errors";
import type { DbHistoryEntry, DbKind } from "../../types";

const emit = defineEmits<{
  fill: [kind: DbKind, connection: string];
  error: [message: string];
}>();

const entries = ref<DbHistoryEntry[]>([]);
const open = ref(false);
const keyword = ref("");
const revealing = ref(false);
const renaming = ref(false);
const editingId = ref("");
const draftName = ref("");

const filtered = computed(() => {
  const query = keyword.value.trim().toLowerCase();
  if (!query) return entries.value;
  return entries.value.filter((entry) =>
    [entry.name, entry.label, kindLabel(entry.kind)].some((value) =>
      value.toLowerCase().includes(query),
    ),
  );
});

async function refresh(): Promise<void> {
  try {
    entries.value = await api.dbHistoryList();
  } catch (err) {
    emit("error", `读取连接历史失败：${describeError(err)}`);
  }
}

async function fill(entry: DbHistoryEntry): Promise<void> {
  if (revealing.value) return;
  revealing.value = true;
  try {
    const connection = await api.dbHistoryReveal(entry.id);
    emit("fill", entry.kind, connection);
    open.value = false;
  } catch (err) {
    emit("error", `回填连接失败：${describeError(err)}`);
  } finally {
    revealing.value = false;
  }
}

function startRename(entry: DbHistoryEntry): void {
  editingId.value = entry.id;
  draftName.value = entry.name;
}

function cancelRename(): void {
  editingId.value = "";
  draftName.value = "";
}

async function saveRename(entry: DbHistoryEntry): Promise<void> {
  if (renaming.value) return;
  const name = draftName.value.trim();
  if (!name) {
    emit("error", "请输入记录名称");
    return;
  }
  if (name === entry.name) {
    cancelRename();
    return;
  }
  renaming.value = true;
  try {
    entries.value = await api.dbHistoryRename(entry.id, name);
    cancelRename();
  } catch (err) {
    emit("error", `重命名失败：${describeError(err)}`);
  } finally {
    renaming.value = false;
  }
}

function kindLabel(value: DbKind): string {
  if (value === "sqlite") return "SQLite";
  return value === "postgres" ? "PostgreSQL" : "MySQL";
}

onMounted(() => {
  void refresh();
});

defineExpose({ refresh });
</script>

<style scoped>
.count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  margin-left: 4px;
  padding: 0 4px;
  border-radius: 999px;
  background: var(--primary-soft);
  color: var(--primary);
  font-size: 11px;
}
.search {
  margin-bottom: 12px;
}
.empty {
  padding: 40px 0;
  text-align: center;
  font-size: 13px;
  color: var(--muted);
}
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 4px 10px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #fafbfd;
  color: #3c4557;
  transition:
    border-color 0.15s ease,
    background 0.15s ease;
}
.item:hover {
  border-color: var(--primary);
  background: var(--primary-soft);
}
.item-head {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.kind {
  color: var(--primary);
  font-size: 12px;
  font-weight: 600;
}
.date {
  color: var(--muted);
  font-size: 11px;
}
.fill {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding: 0;
  border: none;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.fill:disabled {
  opacity: 0.6;
  cursor: default;
}
.text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 600;
}
.label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--muted);
  font-size: 11px;
}
.action {
  margin-left: auto;
  color: var(--primary);
  font-size: 12px;
  flex: none;
}
.rename {
  align-self: center;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
  cursor: pointer;
}
.rename:hover {
  color: var(--primary);
}
.rename-input {
  grid-column: 1 / -1;
}
.edit-actions {
  grid-column: 1 / -1;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.hint {
  margin: 16px 0 0;
  font-size: 12px;
  color: var(--muted);
}
</style>
