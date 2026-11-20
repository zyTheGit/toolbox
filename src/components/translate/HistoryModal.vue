<template>
  <n-modal :show="show" :style="modalStyle" @update:show="onShowChange">
    <div class="hist-modal">
      <div class="modal-head">
        <h3>翻译历史</h3>
        <n-button
          quaternary
          circle
          size="small"
          title="关闭"
          aria-label="关闭"
          @click="close"
        >
          <AppIcon name="x" :size="16" />
        </n-button>
      </div>

      <template v-if="historyEnabled">
        <HistoryFilterBar
          v-model:keyword="keyword"
          v-model:direction="direction"
          v-model:provider="provider"
          v-model:favorites-only="favoritesOnly"
        />
        <div class="hist-summary">
          <span class="hint">共 {{ total }} 条记录</span>
        </div>

        <div v-if="loading" class="hist-loading">
          <n-spin size="small" />
          <span class="muted">正在加载…</span>
        </div>
        <div v-else-if="error" class="hist-error">
          <p class="error-text">{{ error }}</p>
          <n-button size="small" @click="reload">重试</n-button>
        </div>
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
        <EmptyState
          v-else
          icon="search"
          title="没有匹配的记录"
          description="调整搜索或筛选条件后再试。"
        />
      </template>
      <EmptyState
        v-else
        icon="history"
        title="历史记录已关闭"
        description="可在「设置」中开启翻译历史后查看。"
      />

      <div class="modal-foot hist-foot">
        <n-button
          v-if="historyEnabled"
          secondary
          type="error"
          size="small"
          :disabled="total === 0"
          @click="confirmClear"
        >
          <template #icon><AppIcon name="trash" :size="16" /></template>
          清空历史
        </n-button>
        <span class="grow"></span>
        <n-button @click="close">关闭</n-button>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  NButton,
  NModal,
  NSpin,
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
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
import HistoryFilterBar from "./HistoryFilterBar.vue";
import HistoryItem from "./HistoryItem.vue";

const PAGE_SIZE = 50;
const modalStyle = {
  width: "640px",
  maxWidth: "calc(100vw - 32px)",
  marginTop: "8vh",
  alignSelf: "flex-start",
};

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

function onShowChange(value: boolean): void {
  emit("update:show", value);
}

function close(): void {
  emit("update:show", false);
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
.hist-modal {
  max-height: 84vh;
  padding: 24px;
  overflow-y: auto;
  background: var(--surface);
  border-radius: var(--r-md);
  box-shadow: var(--shadow-2);
}

.hist-summary {
  margin-bottom: 6px;
}

.hist-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 48px 0;
}

.hist-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 32px 0;
  text-align: center;
}

.hist-error .error-text {
  margin: 0;
}

.hist-list {
  max-height: 46vh;
  overflow-y: auto;
}

.hist-list :deep(.hist-row:last-of-type) {
  border-bottom: 0;
}

.load-more {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 18px 0 4px;
}

.hist-foot {
  justify-content: flex-start;
}

@media (max-width: 560px) {
  .hist-modal {
    padding: 18px;
  }
}
</style>
