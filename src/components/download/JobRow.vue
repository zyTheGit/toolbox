<template>
  <article class="job" :class="job.status">
    <div class="job-head">
      <div class="job-main grow">
        <div class="job-title">
          <StatusPill :status="job.status" />
          <span class="job-url mono ellipsis grow" :title="job.url">{{ job.url }}</span>
        </div>
        <div class="job-meta">
          <span class="tag">{{ qualityText }}</span>
          <span v-if="subtitleText" class="tag">{{ subtitleText }}</span>
          <span v-if="job.name" class="job-name ellipsis" :title="job.name">{{ job.name }}</span>
          <span v-if="timeText" class="faint">{{ timeText }}</span>
        </div>
      </div>
      <div class="job-actions">
        <n-button
          v-if="isActive"
          size="small"
          :loading="action === 'cancel'"
          :disabled="busy"
          @click="cancel"
        >
          <template #icon><AppIcon name="x" :size="16" /></template>
          取消
        </n-button>
        <n-button
          v-if="canRetry"
          size="small"
          :loading="retryBusy"
          :disabled="busy"
          @click="retry"
        >
          <template #icon><AppIcon name="refresh" :size="16" /></template>
          {{ retryLabel }}
        </n-button>
        <n-dropdown
          v-if="canRetry"
          trigger="click"
          :options="moreOptions"
          @select="onMoreSelect"
        >
          <n-button
            size="small"
            quaternary
            title="更多操作"
            aria-label="更多操作"
            :disabled="busy"
          >
            <template #icon><AppIcon name="dots" :size="16" /></template>
          </n-button>
        </n-dropdown>
        <n-button
          v-if="canOpenDir"
          size="small"
          :loading="action === 'open'"
          :disabled="busy"
          @click="openDir"
        >
          <template #icon><AppIcon name="folder-open" :size="16" /></template>
          打开目录
        </n-button>
        <n-button
          size="small"
          quaternary
          :title="showLog ? '收起日志' : '展开日志'"
          :aria-label="showLog ? '收起日志' : '展开日志'"
          @click="showLog = !showLog"
        >
          <template #icon><AppIcon name="terminal" :size="16" /></template>
        </n-button>
        <n-button
          v-if="isTerminal"
          size="small"
          quaternary
          title="移除记录"
          aria-label="移除记录"
          :loading="action === 'remove'"
          :disabled="busy"
          @click="remove"
        >
          <template #icon><AppIcon name="trash" :size="16" /></template>
        </n-button>
      </div>
    </div>

    <template v-if="isActive">
      <template v-if="job.status === 'running'">
        <div class="progress job-progress"><i :style="{ width: percent + '%' }" /></div>
        <div class="job-stats">
          <span>速度 <b>{{ job.progress.speed || "—" }}</b></span>
          <span>剩余 <b>{{ job.progress.eta || "—" }}</b></span>
          <span>已下载 <b>{{ sizeText || "—" }}</b></span>
        </div>
      </template>
      <div v-else class="job-hint hint">排队等待，前序任务结束后自动开始</div>
    </template>

    <div v-if="failure" class="alert alert-err job-alert">
      <AppIcon name="alert" :size="16" />
      <span>
        {{ failure.message }}
        <template v-if="failure.action"><br />下一步：{{ failure.action }}</template>
      </span>
    </div>

    <div v-if="job.status === 'completed' && job.file" class="job-file">
      <AppIcon name="file" :size="16" />
      <span class="mono ellipsis" :title="job.file">{{ job.file }}</span>
    </div>

    <div v-if="job.notice" class="alert alert-info job-alert">
      <AppIcon name="info" :size="16" />
      <span>{{ job.notice }}</span>
    </div>

    <div v-if="needsCredentials" class="alert alert-warn job-alert">
      <AppIcon name="key" :size="16" />
      <span>该任务使用了代理 / Cookie / Referer 等凭据。凭据只保存在内存中，重启后继续或重试需要重新填写。</span>
    </div>

    <JobLog v-if="showLog" :job="job" />

    <RetryCredentialsModal
      :show="showCredentials"
      :initial="retryInitial"
      :submitting="retrySubmitting"
      :error="retryError"
      @update:show="showCredentials = $event"
      @submit="submitCredentials"
    />
  </article>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NButton, NDropdown, useMessage, type DropdownOption } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import StatusPill from "../StatusPill.vue";
import JobLog from "./JobLog.vue";
import RetryCredentialsModal from "./RetryCredentialsModal.vue";
import type { Job } from "../../types";
import {
  formatJobTime,
  isActiveJob,
  jobFailure,
  jobPercent,
  jobQuality,
  jobSize,
  jobSubtitle,
} from "./jobDisplay";
import { useJobRetry } from "./useJobRetry";
import { useJobActions } from "./useJobActions";

const props = defineProps<{ job: Job }>();

const message = useMessage();
const showLog = ref(false);

const {
  busy: retryBusy,
  showCredentials,
  submitting: retrySubmitting,
  error: retryError,
  initial: retryInitial,
  retry,
  redownload,
  submitCredentials,
} = useJobRetry(
  () => props.job,
  {
    success: (text) => message.success(text),
    error: (text) => message.error(text),
  },
);

const { action, busy, cancel, openDir, remove, requestRedownload } = useJobActions(
  () => props.job,
  () => retryBusy.value,
  redownload,
);

/** 中断任务用"继续"更贴合语义；失败与取消仍是"重试"。 */
const retryLabel = computed(() =>
  props.job.status === "interrupted" ? "继续" : "重试",
);
const moreOptions: DropdownOption[] = [
  { label: "重新下载（清空断点）", key: "redownload" },
];

function onMoreSelect(key: string | number): void {
  if (key === "redownload") requestRedownload();
}

const isActive = computed(() => isActiveJob(props.job));
const isTerminal = computed(() => !isActive.value);
/** 失败、中断与已取消都允许重新入队。 */
const canRetry = computed(
  () =>
    props.job.status === "failed" ||
    props.job.status === "interrupted" ||
    props.job.status === "canceled",
);
/** 已完成可查看成品目录，失败 / 中断可查看残留文件目录。 */
const canOpenDir = computed(
  () =>
    props.job.status === "completed" ||
    props.job.status === "failed" ||
    props.job.status === "interrupted",
);
const needsCredentials = computed(
  () =>
    Boolean(props.job.hasCredentials) &&
    (props.job.status === "failed" || props.job.status === "interrupted"),
);

const qualityText = computed(() => jobQuality(props.job));
const subtitleText = computed(() => jobSubtitle(props.job));
const percent = computed(() => jobPercent(props.job));
const sizeText = computed(() => jobSize(props.job));
const timeText = computed(() => {
  if (isActive.value) return "";
  return formatJobTime(props.job.finishedAt || props.job.createdAt);
});
const failure = computed(() => jobFailure(props.job));
</script>

<style scoped>
.job {
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  background: var(--surface);
  min-width: 0;
}

.job + .job {
  margin-top: 10px;
}

.job-head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.job-main {
  min-width: 0;
}

.job-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.job-url {
  color: var(--text-2);
}

.job-title .pill {
  flex: none;
}

.job-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 6px;
  font-size: var(--fs-12);
  color: var(--muted);
  min-width: 0;
}

.job-name {
  max-width: 360px;
}

.job-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 6px;
  flex: none;
}

.job-progress {
  margin-top: 10px;
}

.job-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  margin-top: 6px;
  font-size: var(--fs-12);
  color: var(--muted);
}

.job-stats b {
  color: var(--text-2);
  font-weight: 500;
  font-family: var(--font-mono);
}

.job-hint {
  margin-top: 8px;
}

.job-alert {
  margin-top: 10px;
}

.job-file {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
  font-size: var(--fs-12);
  color: var(--muted);
  min-width: 0;
}

.job-file .ic {
  flex: none;
  color: var(--faint);
}

@media (max-width: 620px) {
  .job-head {
    flex-direction: column;
    align-items: stretch;
  }

  .job-actions {
    justify-content: flex-start;
  }
}
</style>
