<template>
  <div class="page">
    <PageHead
      title="翻译"
      subtitle="中英文互译，使用设置中的默认服务，失败时自动回退。"
      icon="translate"
    >
      <n-button @click="historyOpen = true">
        <template #icon><AppIcon name="history" :size="16" /></template>
        翻译历史
      </n-button>
    </PageHead>

    <div class="translate-grid">
      <TranslateSourceCard
        v-model:input="input"
        v-model:direction="direction"
        :loading="loading"
        :speaking="sourceSpeaking"
        :undo-available="undoAvailable"
        @translate="doTranslate"
        @clear="clearInput"
        @undo="undoClear"
        @speak="toggleSpeak('source')"
        @swap="swapDirection"
      />

      <TranslateResultCard
        :result="result"
        :loading="loading"
        :error="error"
        :provider-name="resultProviderName"
        :meta-text="resultMeta"
        :foot-hint="resultFootHint"
        :speaking="targetSpeaking"
        @close-error="error = ''"
        @speak="toggleSpeak('target')"
        @copy="copyResult"
      />
    </div>

    <RecentTranslations
      :entries="recentEntries"
      :loading="recentLoading"
      :error="recentError"
      :history-enabled="historyEnabled"
      @open="historyOpen = true"
      @load="loadEntry"
      @favorite="toggleFavorite"
      @copy="copyEntry"
      @retry="refreshRecent"
    />

    <HistoryModal
      v-model:show="historyOpen"
      :refresh-key="historyRefreshKey"
      @load="loadEntry"
      @changed="refreshRecent"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { NButton, useMessage } from "naive-ui";
import type {
  Direction,
  HistoryEntry,
  TranslateRequest,
  TranslateResult,
} from "../types";
import { api, events } from "../api";
import { store } from "../store";
import { describeError } from "../utils/errors";
import PageHead from "../components/PageHead.vue";
import AppIcon from "../components/AppIcon.vue";
import TranslateSourceCard from "../components/translate/TranslateSourceCard.vue";
import TranslateResultCard from "../components/translate/TranslateResultCard.vue";
import RecentTranslations from "../components/translate/RecentTranslations.vue";
import HistoryModal from "../components/translate/HistoryModal.vue";
import { detectLang, ttsLang, useSpeechPlayback } from "../composables/useSpeechPlayback";

const message = useMessage();

let translateSeq = 0;

const input = ref("");
const direction = ref("auto");
const loading = ref(false);
const error = ref("");
const result = ref<TranslateResult | null>(null);
const loadedEntry = ref<HistoryEntry | null>(null);
const undoSnapshot = ref<{ input: string; direction: string } | null>(null);
let undoTimer: ReturnType<typeof setTimeout> | undefined;
const undoAvailable = computed(() => undoSnapshot.value !== null);

const recentEntries = ref<HistoryEntry[]>([]);
const recentLoading = ref(false);
const recentError = ref("");
const historyOpen = ref(false);
const historyRefreshKey = ref(0);

const historyEnabled = computed(() => store.config?.history.enabled ?? true);

const directionHint = computed(() => {
  if (direction.value === "zh2en") return "中 → 英";
  if (direction.value === "en2zh") return "英 → 中";
  return "自动识别方向";
});

const resultProviderName = computed(
  () => loadedEntry.value?.providerName ?? result.value?.providerName ?? "",
);

const resultMeta = computed(() => {
  if (loadedEntry.value) {
    return `${formatEntryTime(loadedEntry.value.createdAt)} · 历史记录`;
  }
  if (result.value) {
    return `${result.value.latencyMs}ms · ${langLabel(result.value.source)} → ${langLabel(
      result.value.target,
    )}`;
  }
  return directionHint.value;
});

const resultFootHint = computed(() =>
  result.value ? "可选中文本手动复制" : "失败时会按服务顺序自动回退",
);

const LANG_LABELS: Record<string, string> = { zh: "中文", en: "英文" };

function langLabel(code: string): string {
  if (!code) return "—";
  return LANG_LABELS[code] ?? code.toUpperCase();
}

function formatEntryTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso || "—";
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
}

function errorText(err: unknown): string {
  return describeError(err);
}

const ttsRate = computed(() => {
  const saved = Number(store.config?.translate.ttsRate ?? 1);
  return Number.isFinite(saved) ? Math.min(2, Math.max(0.5, saved)) : 1;
});

const { speakingSide, toggle: toggleSpeech, stop: stopSpeaking } = useSpeechPlayback(ttsRate);

const sourceSpeaking = computed(() => speakingSide.value === "source");
const targetSpeaking = computed(() => speakingSide.value === "target");

const sourceTtsLang = computed(() => {
  if (direction.value === "zh2en") return ttsLang("zh");
  if (direction.value === "en2zh") return ttsLang("en");
  return ttsLang(detectLang(input.value));
});

function toggleSpeak(side: "source" | "target"): void {
  const text = side === "source" ? input.value : result.value?.text ?? "";
  if (!text.trim()) return;
  const lang = side === "source" ? sourceTtsLang.value : ttsLang(result.value?.target ?? "zh");
  if (!toggleSpeech(text, lang, side)) {
    message.warning("当前环境不支持语音播放");
  }
}

function directionOf(value: string): Direction {
  return value === "zh2en" || value === "en2zh" ? value : "auto";
}

function swapDirection(): void {
  if (direction.value === "zh2en") {
    direction.value = "en2zh";
  } else if (direction.value === "en2zh") {
    direction.value = "zh2en";
  } else {
    direction.value = /[\u3400-\u9fff]/.test(input.value) ? "en2zh" : "zh2en";
  }
}

function clearUndo(): void {
  undoSnapshot.value = null;
  if (undoTimer) {
    clearTimeout(undoTimer);
    undoTimer = undefined;
  }
}

function undoClear(): void {
  const snapshot = undoSnapshot.value;
  if (!snapshot) return;
  invalidateTranslate();
  stopSpeaking();
  input.value = snapshot.input;
  direction.value = snapshot.direction;
  result.value = null;
  loadedEntry.value = null;
  error.value = "";
  clearUndo();
}

function clearInput(): void {
  if (input.value) {
    clearUndo();
    undoSnapshot.value = { input: input.value, direction: direction.value };
    undoTimer = setTimeout(clearUndo, 8000);
  }
  invalidateTranslate();
  stopSpeaking();
  input.value = "";
  result.value = null;
  loadedEntry.value = null;
  error.value = "";
}

function invalidateTranslate(): void {
  translateSeq += 1;
  loading.value = false;
}

async function doTranslate(): Promise<void> {
  if (loading.value) return;
  const text = input.value.trim();
  if (!text) {
    message.warning("请输入要翻译的内容");
    return;
  }

  stopSpeaking();
  clearUndo();
  const seq = ++translateSeq;
  loading.value = true;
  error.value = "";
  loadedEntry.value = null;
  const request: TranslateRequest = {
    text,
    direction: directionOf(direction.value),
    provider: null,
  };

  try {
    const value = await api.translate(request);
    if (seq !== translateSeq) return;
    result.value = value;
    error.value = "";
    historyRefreshKey.value += 1;
    await refreshRecent();
  } catch (err) {
    if (seq !== translateSeq) return;
    result.value = null;
    error.value = errorText(err);
  } finally {
    if (seq === translateSeq) loading.value = false;
  }
}

async function copyResult(): Promise<void> {
  const text = result.value?.text ?? "";
  if (!text) return;
  if (typeof navigator.clipboard?.writeText !== "function") {
    message.error("当前环境不支持自动复制，请手动选择文本复制");
    return;
  }
  try {
    await navigator.clipboard.writeText(text);
    message.success("译文已复制");
  } catch {
    message.error("复制失败，请手动选择文本复制");
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

let recentSeq = 0;

async function refreshRecent(): Promise<void> {
  if (!historyEnabled.value) {
    recentEntries.value = [];
    recentError.value = "";
    recentLoading.value = false;
    return;
  }
  const seq = ++recentSeq;
  recentLoading.value = true;
  recentError.value = "";
  try {
    const page = await api.historyQuery({ limit: 5 });
    if (seq !== recentSeq) return;
    recentEntries.value = page.entries;
  } catch (err) {
    if (seq !== recentSeq) return;
    recentEntries.value = [];
    recentError.value = errorText(err);
  } finally {
    if (seq === recentSeq) recentLoading.value = false;
  }
}

function loadEntry(entry: HistoryEntry): void {
  invalidateTranslate();
  stopSpeaking();
  clearUndo();
  input.value = entry.text;
  direction.value = entry.direction;
  result.value = entry.result
    ? {
        text: entry.result,
        provider: entry.provider,
        providerName: entry.providerName,
        source: entry.source,
        target: entry.target,
        latencyMs: 0,
      }
    : null;
  loadedEntry.value = entry;
  error.value = "";
  historyOpen.value = false;
}

async function toggleFavorite(entry: HistoryEntry): Promise<void> {
  const next = !entry.favorite;
  try {
    await api.historySetFavorite(entry.id, next);
    entry.favorite = next;
    historyRefreshKey.value += 1;
  } catch (err) {
    message.error(errorText(err));
  }
}

watch(historyEnabled, () => {
  void refreshRecent();
});

// 原文被编辑或方向切换后，正在朗读的旧文本立即停止。
watch([input, direction], () => {
  if (speakingSide.value === "source") stopSpeaking();
});

let quickUnlisten: (() => void) | undefined;

onMounted(() => {
  void refreshRecent();
  void events
    .onQuickTranslate((text) => {
      invalidateTranslate();
      stopSpeaking();
      clearUndo();
      input.value = text;
      direction.value = "auto";
      result.value = null;
      loadedEntry.value = null;
      error.value = "";
      message.info("已从剪贴板载入内容，确认后点击翻译");
    })
    .then((off) => {
      quickUnlisten = off;
    })
    .catch(() => {
      // 事件桥异常由 App.vue 顶部提示条统一反馈。
    });
});

onBeforeUnmount(() => {
  quickUnlisten?.();
  stopSpeaking();
  clearUndo();
});
</script>

<style scoped>
.translate-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
@media (max-width: 900px) { .translate-grid { grid-template-columns: minmax(0, 1fr); } }
</style>
