<template>
  <n-button quaternary class="history-btn" title="最近连接" @click="open = true">
    <template #icon>
      <AppIcon name="history" :size="16" />
    </template>
    最近连接
    <span v-if="entries.length" class="count">{{ entries.length }}</span>
  </n-button>

  <n-modal
    v-model:show="open"
    preset="card"
    title="最近连接"
    :style="{ width: '480px', maxWidth: '92vw' }"
    :bordered="false"
  >
    <n-input
      v-model:value="keyword"
      size="small"
      clearable
      placeholder="搜索名称或连接目标"
      class="search"
    />

    <div v-if="!entries.length" class="recent-empty">
      <EmptyState
        icon="history"
        title="暂无连接记录"
        description="连接成功后会自动记录到这里，可改名并回填连接串。"
      />
    </div>
    <div v-else-if="!filtered.length" class="recent-empty">
      <EmptyState icon="search" title="没有匹配的记录" description="换个关键词试试。" />
    </div>
    <div v-else class="recent-list">
      <div
        v-for="entry in filtered"
        :key="entry.id"
        class="recent-item"
        :class="{ editing: editingId === entry.id }"
      >
        <template v-if="editingId === entry.id">
          <n-input
            v-model:value="draftName"
            size="small"
            maxlength="60"
            placeholder="记录名称"
            :disabled="renaming"
            @keydown.enter.prevent="saveRename(entry)"
            @keydown.escape.prevent="cancelRename"
          />
          <n-button size="small" type="primary" :loading="renaming" @click="saveRename(entry)">
            保存
          </n-button>
          <n-button size="small" quaternary :disabled="renaming" @click="cancelRename">
            取消
          </n-button>
        </template>
        <template v-else>
          <div class="recent-info">
            <div class="recent-title">
              <b class="name ellipsis" :title="entry.name">{{ entry.name }}</b>
              <span class="tag">{{ kindLabel(entry.kind) }}</span>
              <span v-if="dbNameOf(entry)" class="tag tag-primary" :title="dbNameOf(entry)">
                {{ dbNameOf(entry) }}
              </span>
            </div>
            <div class="recent-meta">
              <span
                v-if="entry.label && entry.label !== entry.name"
                class="mono target ellipsis"
                :title="entry.label"
              >
                {{ entry.label }}
              </span>
              <span class="date">更新于 {{ entry.updatedAt.slice(0, 10) }}</span>
            </div>
          </div>
          <div class="recent-actions">
            <n-button
              size="small"
              :loading="fillingId === entry.id"
              :disabled="!!fillingId && fillingId !== entry.id"
              @click="fill(entry)"
            >
              回填
            </n-button>
            <n-button size="small" quaternary :disabled="!!fillingId" @click="startRename(entry)">
              改名
            </n-button>
          </div>
        </template>
      </div>
    </div>

    <p class="hint foot">「回填」会把完整连接串（含密码）填入上方输入框。</p>

    <template #footer>
      <div class="modal-foot">
        <n-button @click="open = false">关闭</n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { NButton, NInput, NModal } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
import { api } from "../../api";
import { describeError } from "../../utils/errors";
import { dbKindLabel as kindLabel } from "./sqlText";
import type { DbHistoryEntry, DbKind } from "../../types";

const emit = defineEmits<{
  fill: [kind: DbKind, connection: string];
  error: [message: string];
}>();

const entries = ref<DbHistoryEntry[]>([]);
const open = ref(false);
const keyword = ref("");
const fillingId = ref("");
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
  if (fillingId.value) return;
  fillingId.value = entry.id;
  try {
    const connection = await api.dbHistoryReveal(entry.id);
    emit("fill", entry.kind, connection);
    open.value = false;
  } catch (err) {
    emit("error", `回填连接失败：${describeError(err)}`);
  } finally {
    fillingId.value = "";
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

/** 从脱敏连接目标里提取库名 / 文件名，仅用于展示标签，不做连接用途。 */
function dbNameOf(entry: DbHistoryEntry): string {
  const text = entry.label.trim();
  if (!text) return "";
  if (entry.kind === "sqlite") {
    const parts = text.split(/[\\/]/);
    return parts[parts.length - 1] || "";
  }
  const keywordMatch = text.match(/(?:^|\s)(?:dbname|database)\s*=\s*([^\s;]+)/i);
  if (keywordMatch) return keywordMatch[1];
  if (!text.includes("://")) return "";
  const rest = text.split("://")[1] ?? "";
  const path = rest.split(/[?#]/)[0] ?? "";
  const slash = path.lastIndexOf("/");
  if (slash < 0 || slash === path.length - 1) return "";
  const name = path.slice(slash + 1);
  return name.includes("@") ? "" : name;
}

onMounted(() => {
  void refresh();
});

defineExpose({ refresh });
</script>

<style scoped>
.history-btn {
  gap: 6px;
}
.count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  margin-left: 4px;
  padding: 0 4px;
  border-radius: var(--r-pill);
  background: var(--primary-soft);
  color: var(--primary);
  font-size: 11px;
}
.search {
  margin-bottom: 12px;
}
.recent-empty {
  padding: 8px 0;
}
.recent-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.recent-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  background: var(--surface-soft);
}
.recent-item.editing {
  grid-template-columns: minmax(0, 1fr) auto auto;
}
.recent-info {
  min-width: 0;
}
.recent-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.name {
  min-width: 0;
  font-size: var(--fs-13);
  font-weight: 600;
}
.recent-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 2px;
  min-width: 0;
}
.target {
  min-width: 0;
  color: var(--muted);
}
.date {
  margin-left: auto;
  flex: none;
  color: var(--faint);
  font-size: var(--fs-12);
}
.recent-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
}
.foot {
  margin: 12px 0 0;
}
</style>
