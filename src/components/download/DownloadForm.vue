<template>
  <div class="card">
    <div class="section-title">新建下载</div>

    <div class="field">
      <div class="label-row">
        <span class="label">下载地址</span>
        <span class="hint">每行一个，仅支持 http(s)，自动去重</span>
      </div>
      <n-input
        v-model:value="urlText"
        aria-label="下载地址"
        type="textarea"
        :rows="4"
        :disabled="submitting"
        placeholder="https://example.com/live/index.m3u8&#10;每行一个地址，可批量入队"
      />
      <div class="hint">
        共 {{ parsed.valid.length }} 个有效链接<template v-if="parsed.invalid.length">，{{ parsed.invalid.length }} 行非法已忽略</template>
      </div>
      <div v-if="parsed.invalid.length" class="invalid mono" :title="parsed.invalid.join('\n')">
        非法链接：{{ invalidPreview }}
      </div>
    </div>

    <div class="form-grid">
      <div class="field">
        <span class="label">保存目录</span>
        <div class="row">
          <n-input
            v-model:value="outDir"
            class="grow"
            aria-label="保存目录"
            placeholder="留空则使用默认下载目录"
            @update:value="outDirTouched = true"
          />
          <n-button size="small" secondary @click="browse">浏览</n-button>
        </div>
      </div>
      <div class="field">
        <span class="label">画质 / 格式</span>
        <n-select v-model:value="quality" aria-label="画质与格式" :options="qualityOptions" />
        <div class="hint">视频优先下载 MP4，音频提取为 MP3（需 ffmpeg）</div>
      </div>
      <div class="field">
        <span class="label">文件名</span>
        <n-input v-model:value="fileName" aria-label="文件名" placeholder="留空使用视频标题，例如：我的视频" />
        <div class="hint">扩展名自动匹配；多个链接会追加 01、02 等序号</div>
      </div>
    </div>

    <details class="adv">
      <summary>高级选项（Referer / Cookie / 代理 / 附加参数）</summary>
      <div class="form-grid adv-grid">
        <div class="field">
          <span class="label">Referer</span>
          <n-input v-model:value="referer" aria-label="Referer" placeholder="部分站点需要来源页" />
        </div>
        <div class="field">
          <span class="label">Cookie 来源</span>
          <n-select v-model:value="cookiesBrowser" aria-label="Cookie 来源" :options="cookieOptions" />
        </div>
        <div class="field">
          <span class="label">代理</span>
          <n-input
            v-model:value="proxy"
            aria-label="代理"
            placeholder="留空则使用设置中的代理"
            @update:value="proxyTouched = true"
          />
        </div>
        <div class="field">
          <span class="label">附加参数</span>
          <n-input v-model:value="extraArgs" aria-label="附加参数" placeholder="例如 --no-check-certificates" />
        </div>
      </div>
    </details>

    <div class="form-foot">
      <span class="hint">{{ footHint }}</span>
      <n-button type="primary" :loading="submitting" :disabled="!canSubmit" @click="submit">
        开始下载
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NButton, NInput, NSelect, useMessage } from "naive-ui";
import { api } from "../../api";
import { store } from "../../store";
import { describeError } from "../../utils/errors";

const message = useMessage();

const urlText = ref("");
const quality = ref("best");
const outDir = ref("");
const outDirTouched = ref(false);
const fileName = ref("");
const referer = ref("");
const cookiesBrowser = ref("");
const proxy = ref("");
const proxyTouched = ref(false);
const extraArgs = ref("");
const submitting = ref(false);

const qualityOptions = [
  { label: "最佳 MP4（默认）", value: "best" },
  { label: "1080p MP4 及以下", value: "1080" },
  { label: "720p MP4 及以下", value: "720" },
  { label: "480p MP4 及以下", value: "480" },
  { label: "音频（MP3）", value: "audio" },
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

const parsed = computed(() => {
  const seen = new Set<string>();
  const valid: string[] = [];
  const invalid: string[] = [];
  for (const raw of urlText.value.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || seen.has(line)) continue;
    seen.add(line);
    try {
      const url = new URL(line);
      if (url.protocol === "http:" || url.protocol === "https:") valid.push(line);
      else invalid.push(line);
    } catch {
      invalid.push(line);
    }
  }
  return { valid, invalid };
});

const invalidPreview = computed(() => {
  const preview = parsed.value.invalid.slice(0, 3).join("、");
  return parsed.value.invalid.length > 3 ? `${preview} 等` : preview;
});

const ytdlpReady = computed(() => Boolean(store.tools?.ytdlp.ok));

const canSubmit = computed(
  () => ytdlpReady.value && parsed.value.valid.length > 0 && !submitting.value,
);

const footHint = computed(() => {
  if (!ytdlpReady.value) return "未检测到 yt-dlp，请到「设置 → 下载与工具」中安装";
  if (parsed.value.invalid.length > 0) return `提交时忽略 ${parsed.value.invalid.length} 行非法链接`;
  if (parsed.value.valid.length > 0) return `将加入 ${parsed.value.valid.length} 个下载任务`;
  return "粘贴链接后可开始下载";
});

function errorText(err: unknown): string {
  return describeError(err);
}

async function browse(): Promise<void> {
  try {
    const picked = await api.pickFolder();
    if (picked) {
      outDir.value = picked;
      outDirTouched.value = true;
    }
  } catch (err) {
    message.error(errorText(err));
  }
}

async function submit(): Promise<void> {
  if (!canSubmit.value) return;
  submitting.value = true;
  try {
    const jobs = await api.downloadEnqueue({
      urls: parsed.value.valid,
      quality: quality.value,
      outDir: outDir.value.trim() || null,
      fileName: fileName.value.trim() || null,
      threads: null,
      referer: referer.value.trim(),
      cookiesBrowser: cookiesBrowser.value,
      proxy: proxy.value.trim(),
      extraArgs: extraArgs.value.trim(),
    });
    urlText.value = "";
    fileName.value = "";
    message.success(`已加入 ${jobs.length} 个下载任务`);
  } catch (err) {
    message.error(errorText(err));
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 14px;
  min-width: 0;
}

.label {
  font-size: 12px;
  font-weight: 600;
  color: #4a5265;
}

.label-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

.invalid {
  color: var(--danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.adv {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0 14px;
  background: var(--panel-soft);
}

.adv summary {
  cursor: pointer;
  padding: 10px 0;
  font-size: 12px;
  font-weight: 600;
  color: #4a5265;
  user-select: none;
}

.adv-grid {
  padding-top: 4px;
}

.form-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 4px;
}
</style>
