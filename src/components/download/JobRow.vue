<template>
  <div class="job" :class="job.status">
    <div class="job-head">
      <div class="job-main">
        <div class="mono job-url ellipsis" :title="job.url">{{ job.url }}</div>
        <div class="job-meta muted">
          <span>{{ job.subtitleOnly ? "仅下载字幕" : qualityLabel }}</span>
          <span v-if="subtitleLabel">{{ subtitleLabel }}</span>
          <span v-if="job.name" class="mono job-file ellipsis" :title="job.name">{{ job.name }}</span>
          <span>{{ createdText }}</span>
          <span v-if="job.file" class="mono job-file ellipsis" :title="job.file">{{ job.file }}</span>
        </div>
      </div>
      <div class="job-side">
        <StatusPill :status="job.status" />
        <div class="job-actions">
          <n-button
            v-if="isActive"
            size="tiny"
            tertiary
            type="warning"
            :disabled="busy"
            @click="cancel"
          >
            取消
          </n-button>
          <n-button size="tiny" quaternary :disabled="busy" @click="openDir">打开目录</n-button>
          <n-button
            v-if="canRetry"
            size="tiny"
            tertiary
            type="primary"
            :disabled="busy"
            @click="retry"
          >
            重试
          </n-button>
          <n-button
            v-if="isTerminal"
            size="tiny"
            quaternary
            type="error"
            :disabled="busy"
            @click="remove"
          >
            移除
          </n-button>
          <n-button size="tiny" quaternary @click="showLog = !showLog">
            {{ showLog ? "收起日志" : "展开日志" }}
          </n-button>
        </div>
      </div>
    </div>

    <div v-if="job.status === 'queued'" class="job-hint hint">排队等待，前序任务结束后自动开始</div>
    <template v-if="failure">
      <div class="job-hint error-text">{{ failure.message }}</div>
      <div v-if="failure.action" class="job-hint hint">下一步：{{ failure.action }}</div>
    </template>
    <div v-if="job.notice" class="job-hint notice-hint">{{ job.notice }}</div>
    <div v-if="needsCredentials" class="job-hint credentials-hint">
      <span class="credentials-tag">需重填凭据</span>
      <span>该任务使用了代理 / Cookie / Referer 等凭据，凭据只保存在内存中，重启后重试需要重新填写。</span>
    </div>

    <div v-if="job.status === 'running'" class="job-progress">
      <n-progress
        type="line"
        :percentage="percent"
        :show-indicator="false"
        :height="6"
        :border-radius="3"
      />
      <div class="job-stats mono" :title="stats">{{ stats || "正在解析…" }}</div>
    </div>

    <div v-if="showLog" class="job-log-box">
      <div v-if="job.status === 'failed' && job.error" class="error-text job-error">{{ job.error }}</div>
      <pre v-if="logLines.length" class="mono job-log">{{ logLines.join("\n") }}</pre>
      <div v-else class="hint">暂无日志输出</div>
    </div>

    <RetryCredentialsModal
      :show="showCredentials"
      :initial="retryInitial"
      :submitting="retrySubmitting"
      :error="retryError"
      @update:show="showCredentials = $event"
      @submit="submitCredentials"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NButton, NProgress, useDialog, useMessage } from "naive-ui";
import StatusPill from "../StatusPill.vue";
import RetryCredentialsModal from "./RetryCredentialsModal.vue";
import { api } from "../../api";
import type { Job } from "../../types";
import { parseError, describeError, type AppError } from "../../utils/errors";
import { useJobRetry } from "./useJobRetry";

const props = defineProps<{ job: Job }>();

const message = useMessage();
const dialog = useDialog();
const showLog = ref(false);
const {
  busy,
  showCredentials,
  submitting: retrySubmitting,
  error: retryError,
  initial: retryInitial,
  retry,
  submitCredentials,
} = useJobRetry(
  () => props.job,
  {
    success: (text) => message.success(text),
    error: (text) => message.error(text),
  },
);

const QUALITY_LABEL: Record<string, string> = {
  best: "最佳画质",
  "1080": "1080p 及以下",
  "720": "720p 及以下",
  "480": "480p 及以下",
  audio: "仅音频",
};

const SUBTITLE_LABEL: Record<string, string> = {
  zh: "中文字幕",
  "zh-mt": "中文机翻",
  en: "英文字幕",
  both: "中英字幕",
};

const isActive = computed(
  () => props.job.status === "queued" || props.job.status === "running",
);
const isTerminal = computed(() => !isActive.value);
const canRetry = computed(
  () => props.job.status === "failed" || props.job.status === "interrupted",
);
const needsCredentials = computed(
  () =>
    Boolean(props.job.hasCredentials) &&
    (props.job.status === "failed" || props.job.status === "interrupted"),
);

const FAILURE_RULES: Array<[RegExp, AppError["action"]]> = [
  [/429|too many requests|rate.?limit|限流/i, "站点限流（YouTube 字幕常见）：等待几分钟后重试，或在高级选项里选择 Cookie 来源"],
  [/yt-dlp|ffmpeg|not found|未找到|找不到/, "到「设置 → 下载与工具」安装或更新工具"],
  [/permission|denied|拒绝访问|只读|read-only|权限/, "检查目录权限，或更换保存目录"],
  [/directory|folder|no such file|目录|路径/, "更换保存目录后重试"],
  [/timeout|timed out|超时/, "稍后重试，或改用更低的画质"],
  [/network|connection|resolve|dns|tls|ssl|网络|连接/, "检查网络或代理后重试"],
];

const failure = computed<AppError | null>(() => {
  if (!canRetry.value || !props.job.error) return null;
  const parsed = parseError(props.job.error);
  if (parsed.action) return parsed;
  const guessed = FAILURE_RULES.find(([pattern]) => pattern.test(parsed.message))?.[1] ?? "";
  return { ...parsed, action: guessed };
});

const qualityLabel = computed(() => QUALITY_LABEL[props.job.quality] ?? props.job.quality);
const subtitleLabel = computed(() => SUBTITLE_LABEL[props.job.subtitles] ?? "");

const percent = computed(() => {
  const value = Number(props.job.progress?.percent);
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(100, Math.round(value)));
});

const stats = computed(() => {
  const progress = props.job.progress;
  if (!progress) return "";
  const parts: string[] = [];
  if (progress.speed) parts.push(`速度 ${progress.speed}`);
  if (progress.eta) parts.push(`剩余 ${progress.eta}`);
  const size = [progress.downloaded, progress.total].filter(Boolean).join(" / ");
  if (size) parts.push(size);
  return parts.join(" · ");
});

const logLines = computed(() => props.job.log.slice(-200));

const createdText = computed(() => formatTime(props.job.createdAt));

function formatTime(iso: string): string {
  if (!iso) return "";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function errorText(err: unknown): string {
  return describeError(err);
}

async function cancel(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    await api.jobCancel(props.job.id);
    message.success("已请求取消");
  } catch (err) {
    message.error(errorText(err));
  } finally {
    busy.value = false;
  }
}

async function openDir(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const path = await api.jobOpen(props.job.id);
    message.success(`已打开 ${path}`);
  } catch (err) {
    message.error(errorText(err));
  } finally {
    busy.value = false;
  }
}

function remove(): void {
  if (busy.value) return;
  dialog.warning({
    title: "移除记录",
    content: "确定移除这条下载记录？已下载的文件不会被删除。",
    positiveText: "移除",
    negativeText: "取消",
    onPositiveClick: async () => {
      busy.value = true;
      try {
        await api.jobRemove(props.job.id);
        message.success("已移除记录");
      } catch (err) {
        message.error(errorText(err));
      } finally {
        busy.value = false;
      }
    },
  });
}
</script>

<style scoped>
.job {
  padding: 12px 14px;
  border-radius: 10px;
  background: var(--panel-soft);
  border: 1px solid var(--border);
}

.job + .job {
  margin-top: 8px;
}

.job.running {
  border-color: #ccd5fb;
  background: #f7f8ff;
}

.job-head {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.job-main {
  flex: 1;
  min-width: 0;
}

.job-url {
  font-size: 12.5px;
  color: #2b3346;
}

.job-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 4px;
  font-size: 12px;
  min-width: 0;
}

.job-file {
  max-width: 320px;
}

.job-side {
  flex: none;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
}

.job-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.job-hint {
  margin-top: 8px;
}

.credentials-hint {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 8px;
  background: #fff7e6;
  border: 1px solid #ffe1b3;
  color: #8a5a00;
  font-size: 12px;
  line-height: 1.6;
}

.notice-hint {
  padding: 8px 10px;
  border-radius: 8px;
  background: #eef4ff;
  border: 1px solid #d6e4ff;
  color: #2b4a8a;
  font-size: 12px;
  line-height: 1.6;
}

.credentials-tag {
  flex: none;
  font-weight: 600;
  color: #b86b00;
}

.job-progress {
  margin-top: 10px;
}

.job-stats {
  margin-top: 6px;
  color: var(--muted);
  font-size: 11.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.job-log-box {
  margin-top: 10px;
  border-top: 1px dashed var(--border);
  padding-top: 10px;
}

.job-error {
  margin-bottom: 8px;
}

.job-log {
  margin: 0;
  max-height: 220px;
  overflow: auto;
  padding: 10px 12px;
  border-radius: 8px;
  background: #1f2430;
  color: #d8dee9;
  font-size: 11.5px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
