<template>
  <div class="card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="plus" :size="16" />
        新建下载
      </h3>
      <span class="hint">{{ linkSummary }}</span>
    </div>

    <div class="stack">
      <div class="field">
        <label>视频链接（每行一个，支持批量）</label>
        <n-input
          id="dl-urls"
          v-model:value="urlText"
          type="textarea"
          :rows="3"
          :disabled="submitting"
          aria-label="视频链接"
          placeholder="https://youtu.be/xxxx&#10;【视频标题】 https://b23.tv/xxxx&#10;https://x.com/user/status/xxxx"
        />
        <div class="hint">支持链接或分享文本，仅识别 http(s)，自动去重</div>
        <div
          v-if="parsed.invalid.length"
          class="invalid mono"
          :title="parsed.invalid.join('\n')"
        >
          无法识别：{{ invalidPreview }}
        </div>
      </div>

      <div class="form-grid">
        <div class="field">
          <label>保存目录</label>
          <div class="input-group">
            <n-input
              v-model:value="outDir"
              class="grow"
              aria-label="保存目录"
              placeholder="留空则使用默认下载目录"
              @update:value="outDirTouched = true"
            />
            <n-button
              :loading="browsing"
              :disabled="submitting"
              title="浏览目录"
              aria-label="浏览目录"
              @click="browse"
            >
              <template #icon><AppIcon name="folder" :size="16" /></template>
            </n-button>
          </div>
        </div>
        <div class="field">
          <label>文件名（可选）</label>
          <n-input
            v-model:value="fileName"
            aria-label="文件名"
            placeholder="留空使用视频标题"
          />
          <div class="hint">扩展名自动匹配；多个链接会追加 01、02 等序号</div>
        </div>
        <div class="field">
          <label>画质</label>
          <n-select
            v-model:value="quality"
            aria-label="画质与格式"
            :options="qualityOptions"
            :disabled="subtitleOnly"
          />
          <div class="hint">
            {{ subtitleOnly ? "仅字幕模式不下载视频/音频，画质设置不生效" : "视频优先下载 MP4，音频提取为 MP3（需 ffmpeg）" }}
          </div>
        </div>
        <div class="field">
          <label>字幕</label>
          <n-select v-model:value="subtitles" aria-label="字幕" :options="subtitleOptions" />
          <div class="hint">{{ subtitleHint }}</div>
        </div>
      </div>

      <n-checkbox
        v-model:checked="subtitleOnly"
        :disabled="!subtitles || submitting"
        size="small"
      >
        仅下载字幕（不下载视频）
      </n-checkbox>

      <details class="collapse">
        <summary>
          <AppIcon name="chevron-right" :size="16" />
          高级选项
        </summary>
        <div class="collapse-body form-grid">
          <div class="field">
            <label>Referer</label>
            <n-input v-model:value="referer" aria-label="Referer" placeholder="部分站点需要来源页" />
          </div>
          <div class="field">
            <label>Cookie 来源</label>
            <n-select v-model:value="cookiesBrowser" aria-label="Cookie 来源" :options="cookieOptions" />
          </div>
          <div class="field">
            <label>代理</label>
            <n-input
              v-model:value="proxy"
              aria-label="代理"
              placeholder="留空则使用设置中的代理"
              @update:value="proxyTouched = true"
            />
          </div>
          <div class="field">
            <label>分片线程</label>
            <n-input-number
              v-model:value="threads"
              :min="0"
              :max="16"
              :precision="0"
              :disabled="submitting"
              :placeholder="`默认 ${defaultThreads}`"
            />
            <div class="hint">m3u8 分片并发数，留空使用设置中的 {{ defaultThreads }}</div>
          </div>
          <div class="field span-2">
            <label>附加参数</label>
            <n-input
              v-model:value="extraArgs"
              class="mono"
              aria-label="附加参数"
              placeholder="例如 --no-check-certificates"
            />
          </div>
        </div>
      </details>

      <div class="spread form-foot">
        <span class="hint">{{ footHint }}</span>
        <n-button type="primary" :loading="submitting" :disabled="!canSubmit" @click="submit">
          <template #icon><AppIcon name="download" :size="16" /></template>
          开始下载
        </n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NButton, NCheckbox, NInput, NInputNumber, NSelect, useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { api } from "../../api";
import { store } from "../../store";
import { describeError } from "../../utils/errors";
import { extractUrls } from "../../utils/download-links";
import { rememberCredentials } from "./sessionCredentials";

const message = useMessage();

const urlText = ref("");
const quality = ref("best");
const subtitles = ref("");
const subtitleOnly = ref(false);
const outDir = ref("");
const outDirTouched = ref(false);
const fileName = ref("");
const referer = ref("");
const cookiesBrowser = ref("");
const proxy = ref("");
const proxyTouched = ref(false);
const threads = ref<number | null>(null);
const extraArgs = ref("");
const submitting = ref(false);
const browsing = ref(false);

const qualityOptions = [
  { label: "最佳 MP4（默认）", value: "best" },
  { label: "1080p MP4 及以下", value: "1080" },
  { label: "720p MP4 及以下", value: "720" },
  { label: "480p MP4 及以下", value: "480" },
  { label: "音频（MP3）", value: "audio" },
];

const subtitleOptions = [
  { label: "不下载字幕", value: "" },
  { label: "中文字幕（站点原字幕）", value: "zh" },
  { label: "中文（YouTube 机翻）", value: "zh-mt" },
  { label: "英文字幕", value: "en" },
  { label: "中英字幕", value: "both" },
];

const cookieOptions = [
  { label: "不使用", value: "" },
  { label: "Chrome", value: "chrome" },
  { label: "Edge", value: "edge" },
  { label: "Firefox", value: "firefox" },
];

watch(
  () => store.config?.downloadDir ?? "",
  (value) => {
    if (!outDirTouched.value) outDir.value = value;
  },
  { immediate: true },
);

watch(
  () => store.config?.proxy ?? "",
  (value) => {
    if (!proxyTouched.value) proxy.value = value;
  },
  { immediate: true },
);

watch(subtitles, (value) => {
  if (!value) subtitleOnly.value = false;
});

const parsed = computed(() => {
  const seen = new Set<string>();
  const valid: string[] = [];
  const invalid: string[] = [];
  for (const raw of urlText.value.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    const found = extractUrls(line);
    if (!found.length) {
      invalid.push(line);
      continue;
    }
    for (const url of found) {
      if (seen.has(url)) continue;
      seen.add(url);
      valid.push(url);
    }
  }
  return { valid, invalid };
});

const invalidPreview = computed(() => {
  const preview = parsed.value.invalid.slice(0, 3).join("、");
  return parsed.value.invalid.length > 3 ? `${preview} 等` : preview;
});

const linkSummary = computed(() => {
  const parts = [`共 ${parsed.value.valid.length} 个有效链接`];
  if (parsed.value.invalid.length) parts.push(`${parsed.value.invalid.length} 行未识别`);
  return parts.join(" · ");
});

const defaultThreads = computed(() => store.config?.fragmentThreads ?? 4);

const subtitleHint = computed(() => {
  if (subtitleOnly.value) return "只保存字幕文件，站点的自动字幕也会尝试";
  if (!subtitles.value) return "选择语言后会尝试下载站点字幕并转 SRT";
  if (subtitles.value === "zh-mt") {
    return "简体中文机翻，需先等约 60 秒规避 YouTube 限流；原字幕语言标错时可能无效";
  }
  return "尝试下载站点字幕并转 SRT（YouTube 等，转换需 ffmpeg）";
});

const ytdlpReady = computed(() => Boolean(store.tools?.ytdlp.ok));
const ffmpegReady = computed(() => Boolean(store.tools?.ffmpeg.ok));

const canSubmit = computed(
  () =>
    ytdlpReady.value &&
    parsed.value.valid.length > 0 &&
    !submitting.value &&
    (!subtitleOnly.value || Boolean(subtitles.value)),
);

const footHint = computed(() => {
  if (!ytdlpReady.value) return "未检测到 yt-dlp，请到「设置 → 下载与工具」中安装";
  if (subtitleOnly.value && !subtitles.value) return "仅下载字幕需要先选择字幕语言";
  if (parsed.value.valid.length > 0) {
    const invalid = parsed.value.invalid.length
      ? `，忽略 ${parsed.value.invalid.length} 行无法识别的链接`
      : "";
    const ffmpeg = ffmpegReady.value ? "" : "；ffmpeg 未安装，字幕转换与音频提取不可用";
    return `将加入 ${parsed.value.valid.length} 个下载任务${invalid}${ffmpeg}`;
  }
  if (!ffmpegReady.value) return "yt-dlp 已就绪；ffmpeg 未安装，字幕转换与音频提取不可用";
  return "yt-dlp 与 ffmpeg 已就绪，下载失败会给出下一步建议。";
});

async function browse(): Promise<void> {
  if (browsing.value) return;
  browsing.value = true;
  try {
    const picked = await api.pickFolder();
    if (picked) {
      outDir.value = picked;
      outDirTouched.value = true;
    }
  } catch (err) {
    message.error(describeError(err));
  } finally {
    browsing.value = false;
  }
}

async function submit(): Promise<void> {
  if (!canSubmit.value) return;
  submitting.value = true;
  try {
    const jobs = await api.downloadEnqueue({
      urls: parsed.value.valid,
      quality: quality.value,
      subtitles: subtitles.value,
      subtitleOnly: subtitleOnly.value,
      outDir: outDir.value.trim() || null,
      fileName: fileName.value.trim() || null,
      threads: threads.value && threads.value > 0 ? threads.value : null,
      referer: referer.value.trim(),
      cookiesBrowser: cookiesBrowser.value,
      proxy: proxy.value.trim(),
      extraArgs: extraArgs.value.trim(),
    });
    urlText.value = "";
    fileName.value = "";
    // 记住本会话最近一次输入的凭据，供重启后重试对话框预填（仅内存）。
    rememberCredentials({
      referer: referer.value,
      cookiesBrowser: cookiesBrowser.value,
      proxy: proxy.value,
      extraArgs: extraArgs.value,
    });
    message.success(`已加入 ${jobs.length} 个下载任务`);
  } catch (err) {
    message.error(describeError(err));
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.input-group {
  display: flex;
  gap: 8px;
}

.invalid {
  color: var(--danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.form-foot {
  flex-wrap: wrap;
}

@media (max-width: 760px) {
  .form-foot {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
