<template>
  <n-drawer :show="show" :width="520" placement="right" @update:show="onShowChange">
    <n-drawer-content title="翻译历史" :native-scrollbar="false" closable>
      <template v-if="historyEnabled">
        <div class="filters">
          <n-input
            v-model:value="keyword"
            size="small"
            clearable
            placeholder="搜索原文或译文"
          />
          <div class="filter-row">
            <n-select
              class="filter-select"
              size="small"
              :value="direction"
              :options="directionOptions"
              @update:value="onDirectionChange"
            />
            <n-select
              class="filter-select"
              size="small"
              :value="provider"
              :options="providerOptions"
              @update:value="onProviderChange"
            />
            <div class="fav-toggle">
              <n-switch v-model:value="favoritesOnly" size="small" />
              <span class="hint">仅收藏</span>
            </div>
            <n-button
              class="clear-btn"
              size="small"
              quaternary
              type="error"
              :disabled="total === 0"
              @click="confirmClear"
            >
              清空
            </n-button>
          </div>
          <div class="hint">共 {{ total }} 条记录</div>
        </div>

        <div v-if="loading" class="drawer-loading">
          <n-spin size="small" />
          <span class="muted">正在加载…</span>
        </div>
        <p v-else-if="error" class="error-text">{{ error }}</p>
        <div v-else-if="entries.length" class="hist-list">
          <HistoryItem
            v-for="entry in entries"
            :key="entry.id"
            :entry="entry"
            :retranslating="retranslatingId === entry.id"
            @load="emit('load', entry)"
            @copy="copyEntry(entry)"
            @retranslate="retranslate(entry)"
            @favorite="toggleFavorite(entry)"
            @delete="removeEntry(entry)"
          />
          <div class="load-more">
            <n-button
              v-if="entries.length < total"
              size="small"
              :loading="loadingMore"
              @click="loadMore"
            >
              加载更多
            </n-button>
            <span v-else class="hint">已加载全部 {{ total }} 条</span>
          </div>
        </div>
        <EmptyState v-else title="没有匹配的记录" description="调整搜索或筛选条件后再试。" />
      </template>
      <EmptyState v-else title="历史记录已关闭" description="可在「设置」中开启翻译历史后查看。" />
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  NButton,
  NDrawer,
  NDrawerContent,
  NInput,
  NSelect,
  NSpin,
  NSwitch,
  useDialog,
  useMessage,
} from "naive-ui";
import type {
  Direction,
  HistoryEntry,
  HistoryPage,
  HistoryQuery,
  ProviderId,
} from "../../types";
import { api } from "../../api";
import { store } from "../../store";
import { describeError } from "../../utils/errors";
import EmptyState from "../EmptyState.vue";
import HistoryItem from "./HistoryItem.vue";

const PAGE_SIZE = 50;

const props = defineProps<{ show: boolean; refreshKey?: number }>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "load", entry: HistoryEntry): void;
  (e: "changed"): void;
}>();

const message = useMessage();
const dialog = useDialog();

const keyword = ref("");
const direction = ref("");
const provider = ref("");
const favoritesOnly = ref(false);
const entries = ref<HistoryEntry[]>([]);
const total = ref(0);
const loading = ref(false);
const loadingMore = ref(false);
const error = ref("");
const retranslatingId = ref("");

const historyEnabled = computed(() => store.config?.history.enabled ?? true);

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

function onDirectionChange(value: SelectValue): void {
  direction.value = keyOf(value);
}

function onProviderChange(value: SelectValue): void {
  provider.value = keyOf(value);
}

function onShowChange(value: boolean): void {
  emit("update:show", value);
}

function errorText(err: unknown): string {
  return describeError(err);
}

function buildQuery(offset: number): HistoryQuery {
  return {
    keyword: keyword.value.trim() || undefined,
    direction: direction.value as Direction | "",
    provider: provider.value as ProviderId | "",
    favoritesOnly: favoritesOnly.value,
    limit: PAGE_SIZE,
    offset,
  };
}

let requestSeq = 0;

async function reload(): Promise<void> {
  if (!historyEnabled.value) {
    entries.value = [];
    total.value = 0;
    error.value = "";
    loading.value = false;
    loadingMore.value = false;
    return;
  }
  const seq = ++requestSeq;
  loading.value = true;
  loadingMore.value = false;
  error.value = "";
  try {
    const page: HistoryPage = await api.historyQuery(buildQuery(0));
    if (seq !== requestSeq) return;
    entries.value = page.entries;
    total.value = page.total;
  } catch (err) {
    if (seq !== requestSeq) return;
    entries.value = [];
    total.value = 0;
    error.value = errorText(err);
  } finally {
    if (seq === requestSeq) loading.value = false;
  }
}

async function loadMore(): Promise<void> {
  if (!historyEnabled.value || loading.value || loadingMore.value) return;
  const seq = requestSeq;
  loadingMore.value = true;
  try {
    const page = await api.historyQuery(buildQuery(entries.value.length));
    if (seq !== requestSeq) return;
    entries.value = [...entries.value, ...page.entries];
    total.value = page.total;
  } catch (err) {
    if (seq === requestSeq) message.error(errorText(err));
  } finally {
    if (seq === requestSeq) loadingMore.value = false;
  }
}

async function toggleFavorite(entry: HistoryEntry): Promise<void> {
  const next = !entry.favorite;
  try {
    await api.historySetFavorite(entry.id, next);
    entry.favorite = next;
    emit("changed");
    if (favoritesOnly.value && !next) {
      entries.value = entries.value.filter((item) => item.id !== entry.id);
      total.value = Math.max(0, total.value - 1);
    }
  } catch (err) {
    message.error(errorText(err));
  }
}

async function removeEntry(entry: HistoryEntry): Promise<void> {
  try {
    await api.historyDelete(entry.id);
    entries.value = entries.value.filter((item) => item.id !== entry.id);
    total.value = Math.max(0, total.value - 1);
    emit("changed");
    if (!entries.value.length && total.value > 0) await reload();
  } catch (err) {
    message.error(errorText(err));
  }
}

async function copyEntry(entry: HistoryEntry): Promise<void> {
  if (!entry.result) {
    message.warning("这条记录没有译文");
    return;
  }
  if (typeof navigator.clipboard?.writeText !== "function") {
    message.error("当前环境不支持自动复制，请载入后手动选择文本复制");
    return;
  }
  try {
    await navigator.clipboard.writeText(entry.result);
    message.success("译文已复制");
  } catch {
    message.error("复制失败，请载入后手动选择文本复制");
  }
}

async function retranslate(entry: HistoryEntry): Promise<void> {
  if (retranslatingId.value) return;
  retranslatingId.value = entry.id;
  try {
    await api.translate({
      text: entry.text,
      direction: entry.direction,
      provider: entry.provider,
    });
    message.success("重译成功，已刷新历史记录");
    await reload();
    emit("changed");
  } catch (err) {
    message.error(errorText(err));
  } finally {
    retranslatingId.value = "";
  }
}

function confirmClear(): void {
  dialog.warning({
    title: "清空翻译历史",
    content: "将删除全部历史记录，此操作无法撤销。",
    positiveText: "清空",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await api.historyClear();
        entries.value = [];
        total.value = 0;
        emit("changed");
        message.success("已清空翻译历史");
      } catch (err) {
        message.error(errorText(err));
      }
    },
  });
}

let debounceTimer: ReturnType<typeof setTimeout> | undefined;

watch(keyword, () => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = undefined;
    void reload();
  }, 250);
});

watch([direction, provider, favoritesOnly], () => {
  void reload();
});

watch(
  () => props.show,
  (open) => {
    if (open) void reload();
  },
);

watch(
  () => props.refreshKey ?? 0,
  () => {
    if (props.show) void reload();
  },
);

watch(historyEnabled, () => {
  if (props.show) void reload();
});

onBeforeUnmount(() => {
  if (debounceTimer) clearTimeout(debounceTimer);
});
</script>

<style scoped>
.filters {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-bottom: 12px;
  margin-bottom: 4px;
  border-bottom: 1px solid var(--border);
}

.filter-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.filter-select {
  width: 118px;
  flex: none;
}

.fav-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
}

.clear-btn {
  margin-left: auto;
}

.drawer-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 48px 0;
}

.hist-list {
  padding-top: 4px;
}

.load-more {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 18px 0 4px;
}
</style>
