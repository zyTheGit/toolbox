<template>
  <div class="page">
    <PageHead title="翻译" subtitle="中英文互译，使用设置中的默认服务，失败时自动回退。" />

    <div class="tr-grid">
      <section class="card tr-pane">
        <div class="pane-head">
          <div class="dir-row">
            <n-radio-group size="small" :value="direction" @update:value="onDirectionChange">
              <n-radio-button value="auto">自动</n-radio-button>
              <n-radio-button value="zh2en">中译英</n-radio-button>
              <n-radio-button value="en2zh">英译中</n-radio-button>
            </n-radio-group>
            <n-button
              size="small"
              quaternary
              title="交换翻译方向（自动时按文本推断）"
              @click="swapDirection"
            >
              交换
            </n-button>
          </div>
        </div>

        <n-input
          v-model:value="input"
          class="src-input"
          type="textarea"
          :autosize="{ minRows: 10, maxRows: 20 }"
          placeholder="输入要翻译的内容，Ctrl + Enter 翻译"
          :disabled="loading"
          @keydown.ctrl.enter.prevent="doTranslate"
        />

        <div class="pane-foot">
          <span class="hint">
            {{ input.length }} 字符 · Ctrl + Enter 翻译
            <n-button
              v-if="undoAvailable"
              text
              type="primary"
              size="tiny"
              class="undo-btn"
              @click="undoClear"
            >
              撤销清空
            </n-button>
          </span>
          <div class="row">
            <n-button size="small" quaternary :disabled="loading || !input" @click="clearInput">
              清空
            </n-button>
            <n-button
              size="small"
              type="primary"
              :loading="loading"
              :disabled="loading || !input.trim()"
              @click="doTranslate"
            >
              翻译
            </n-button>
          </div>
        </div>
      </section>

      <section class="card tr-pane result-pane">
        <div class="pane-head">
          <span class="pane-title">译文</span>
          <span v-if="loadedEntry" class="hint meta-text">
            {{ loadedEntry.providerName }} · {{ formatEntryTime(loadedEntry.createdAt) }} · 历史记录
          </span>
          <span v-else-if="result" class="hint meta-text">
            {{ result.providerName }} · {{ result.latencyMs }}ms · {{ langLabel(result.source) }}
            → {{ langLabel(result.target) }}
          </span>
          <span v-else class="hint">{{ directionHint }}</span>
        </div>

        <n-alert
          v-if="error"
          class="result-error"
          type="error"
          :bordered="false"
          closable
          @close="error = ''"
        >
          {{ error }}
        </n-alert>

        <div v-if="loading" class="result-body body-center">
          <n-spin size="small" />
          <span class="muted">正在翻译…</span>
        </div>
        <div v-else-if="result" class="result-body selectable">{{ result.text }}</div>
        <div v-else class="result-body result-empty">
          <EmptyState title="译文将显示在这里" description="输入内容后按 Ctrl + Enter，或点击「翻译」。" />
        </div>

        <div class="pane-foot">
          <span class="hint ellipsis">{{ resultFootHint }}</span>
          <div class="row">
            <n-button
              size="small"
              quaternary
              :disabled="!result || loading"
              :title="speaking ? '停止朗读' : '朗读译文'"
              @click="toggleSpeak"
            >
              {{ speaking ? "停止" : "朗读" }}
            </n-button>
            <n-button size="small" quaternary :disabled="!result || loading" @click="copyResult">
              复制译文
            </n-button>
          </div>
        </div>
      </section>
    </div>

    <section class="card recent-card">
      <div class="spread recent-head">
        <h3 class="section-title">最近翻译</h3>
        <n-button text type="primary" @click="drawerOpen = true">查看全部</n-button>
      </div>

      <template v-if="historyEnabled">
        <div v-if="recentLoading" class="recent-loading">
          <n-spin size="small" />
        </div>
        <div v-else-if="recentEntries.length" class="recent-list">
          <HistoryItem
            v-for="entry in recentEntries"
            :key="entry.id"
            :entry="entry"
            compact
            @load="loadEntry(entry)"
            @favorite="toggleFavorite(entry)"
            @delete="deleteEntry(entry)"
          />
        </div>
        <div v-else-if="recentError" class="recent-error">
          <EmptyState title="最近翻译加载失败" :description="recentError">
            <n-button size="small" :loading="recentLoading" @click="refreshRecent">重试</n-button>
          </EmptyState>
        </div>
        <EmptyState v-else title="暂无翻译记录" description="翻译成功后会自动保存到这里。" />
      </template>
      <EmptyState v-else title="历史记录已关闭" description="可在「设置」中开启翻译历史。" />
    </section>

    <HistoryDrawer
      v-model:show="drawerOpen"
      :refresh-key="drawerRefreshKey"
      @load="loadEntry"
      @changed="refreshRecent"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NInput,
  NRadioButton,
  NRadioGroup,
  NSpin,
  useMessage,
} from "naive-ui";
import type {
  Direction,
  HistoryEntry,
  TranslateRequest,
  TranslateResult,
} from "../types";
import { api } from "../api";
import { events } from "../api";
import { store } from "../store";
import { describeError } from "../utils/errors";
import PageHead from "../components/PageHead.vue";
import EmptyState from "../components/EmptyState.vue";
import HistoryItem from "../components/translate/HistoryItem.vue";
import HistoryDrawer from "../components/translate/HistoryDrawer.vue";

const message = useMessage();

let translateSeq = 0;

const input = ref("");
const direction = ref("auto");
const loading = ref(false);
const error = ref("");
const result = ref<TranslateResult | null>(null);
const loadedEntry = ref<HistoryEntry | null>(null);
const speaking = ref(false);
const undoSnapshot = ref<{ input: string; direction: string } | null>(null);
let undoTimer: ReturnType<typeof setTimeout> | undefined;
const undoAvailable = computed(() => undoSnapshot.value !== null);

const recentEntries = ref<HistoryEntry[]>([]);
const recentLoading = ref(false);
const recentError = ref("");
const drawerOpen = ref(false);
const drawerRefreshKey = ref(0);

const historyEnabled = computed(() => store.config?.history.enabled ?? true);

const directionHint = computed(() => {
  if (direction.value === "zh2en") return "中 → 英";
  if (direction.value === "en2zh") return "英 → 中";
  return "自动识别方向";
});

const resultFootHint = computed(() =>
  result.value ? "可选中文本手动复制" : "失败时会按服务顺序自动回退",
);

function onDirectionChange(value: string | number | boolean): void {
  direction.value = String(value);
}

function directionOf(value: string): Direction {
  return value === "zh2en" || value === "en2zh" ? value : "auto";
}

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

const TTS_LANG: Record<string, string> = { zh: "zh-CN", en: "en-US" };

const ttsRate = computed(() => {
  const saved = Number(store.config?.translate.ttsRate ?? 1);
  return Number.isFinite(saved) ? Math.min(2, Math.max(0.5, saved)) : 1;
});

let speechSeq = 0;

function speechSupported(): boolean {
  return typeof window !== "undefined" && "speechSynthesis" in window;
}

function pickVoice(lang: string): SpeechSynthesisVoice | null {
  const voices = window.speechSynthesis.getVoices();
  const normalized = lang.toLowerCase();
  const prefix = normalized.slice(0, 2);
  return (
    voices.find((voice) => voice.lang.replace("_", "-").toLowerCase() === normalized) ??
    voices.find((voice) => voice.lang.toLowerCase().startsWith(prefix)) ??
    null
  );
}

function splitForSpeech(text: string): string[] {
  const chunks: string[] = [];
  const boundaries = ["。", "！", "？", "；", ". ", "! ", "? ", "; ", "\n"];
  let rest = text.trim();
  while (rest.length > 220) {
    const head = rest.slice(0, 220);
    let cut = -1;
    for (const mark of boundaries) {
      cut = Math.max(cut, head.lastIndexOf(mark));
    }
    if (cut < 60) cut = 219;
    chunks.push(rest.slice(0, cut + 1).trim());
    rest = rest.slice(cut + 1).trim();
  }
  if (rest) chunks.push(rest);
  return chunks.filter(Boolean);
}

function stopSpeaking(): void {
  speechSeq += 1;
  speaking.value = false;
  if (speechSupported()) window.speechSynthesis.cancel();
}

function speakResult(): void {
  if (!result.value) return;
  if (!speechSupported()) {
    message.warning("当前环境不支持语音播放");
    return;
  }

  const chunks = splitForSpeech(result.value.text);
  if (!chunks.length) return;

  const lang = TTS_LANG[result.value.target] ?? "zh-CN";
  const voice = pickVoice(lang);
  const seq = ++speechSeq;
  window.speechSynthesis.cancel();
  speaking.value = true;

  chunks.forEach((chunk, index) => {
    const utterance = new SpeechSynthesisUtterance(chunk);
    utterance.lang = lang;
    if (voice) utterance.voice = voice;
    utterance.rate = ttsRate.value;
    const finish = (): void => {
      if (seq === speechSeq && index === chunks.length - 1) speaking.value = false;
    };
    utterance.onend = finish;
    utterance.onerror = finish;
    window.speechSynthesis.speak(utterance);
  });
}

function toggleSpeak(): void {
  if (speaking.value) {
    stopSpeaking();
    return;
  }
  speakResult();
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
    drawerRefreshKey.value += 1;
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
  drawerOpen.value = false;
}

async function toggleFavorite(entry: HistoryEntry): Promise<void> {
  const next = !entry.favorite;
  try {
    await api.historySetFavorite(entry.id, next);
    entry.favorite = next;
    drawerRefreshKey.value += 1;
  } catch (err) {
    message.error(errorText(err));
  }
}

async function deleteEntry(entry: HistoryEntry): Promise<void> {
  try {
    await api.historyDelete(entry.id);
    drawerRefreshKey.value += 1;
    await refreshRecent();
  } catch (err) {
    message.error(errorText(err));
  }
}

watch(historyEnabled, () => {
  void refreshRecent();
});

watch(ttsRate, () => {
  if (speaking.value) speakResult();
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
      // The event banner in App.vue reports a broken event bridge.
    });
});

onBeforeUnmount(() => {
  quickUnlisten?.();
  stopSpeaking();
  clearUndo();
});
</script>

<style scoped>
.tr-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.tr-grid > .card + .card {
  margin-top: 0;
}

.tr-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
}

.dir-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.pane-title {
  font-size: 13px;
  font-weight: 600;
  color: #333b4d;
}

.meta-text {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.undo-btn {
  margin-left: 8px;
}

.recent-error {
  padding: 0;
}

.pane-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: auto;
}

.result-error {
  flex: none;
}

.result-body {
  flex: 1;
  min-height: 220px;
  background: var(--panel-soft);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px 14px;
  font-size: 14px;
  line-height: 1.75;
  color: var(--text);
  white-space: pre-wrap;
  word-break: break-word;
  overflow: auto;
}

.selectable {
  user-select: text;
}

.body-center {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
}

.result-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}

.recent-card {
  margin-top: 16px;
}

.recent-head {
  align-items: center;
}

.recent-head .section-title {
  margin: 0;
}

.recent-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 28px 0;
}

.recent-list {
  display: flex;
  flex-direction: column;
}

.recent-list :deep(.hist-item:first-child) {
  border-top: 1px solid var(--border);
}

@media (max-width: 900px) {
  .tr-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
