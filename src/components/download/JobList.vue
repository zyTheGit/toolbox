<template>
  <div class="card">
    <div class="section-title spread">
      <span>队列 / 最近记录</span>
      <span class="hint queue-hint">
        同时下载 {{ downloadConcurrency }} 个<template v-if="activeJobs.length"> · 进行中 {{ activeJobs.length }}</template>
      </span>
    </div>

    <EmptyState
      v-if="!store.jobs.length"
      title="暂无下载任务"
      description="在上方粘贴视频链接或分享文本，点击开始下载后会在这里看到进度。"
    >
      <template #icon>
        <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 4v10" />
          <path d="M8 10l4 4 4-4" />
          <path d="M4 18h16" />
          <path d="M4 21h16" />
        </svg>
      </template>
    </EmptyState>

    <template v-else>
      <div v-if="activeJobs.length" class="group">
        <div class="group-label">进行中 · {{ activeJobs.length }}</div>
        <JobRow v-for="job in activeJobs" :key="job.id" :job="job" />
      </div>
      <div v-if="historyJobs.length" class="group">
        <div class="group-label">历史 · {{ historyJobs.length }}</div>
        <JobRow v-for="job in historyJobs" :key="job.id" :job="job" />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import EmptyState from "../EmptyState.vue";
import JobRow from "./JobRow.vue";
import { store } from "../../store";
import type { Job } from "../../types";

function isActive(job: Job): boolean {
  return job.status === "queued" || job.status === "running";
}

const activeJobs = computed(() => store.jobs.filter(isActive));
const historyJobs = computed(() => store.jobs.filter((job) => !isActive(job)));
const downloadConcurrency = computed(() => store.config?.downloadConcurrency ?? 2);
</script>

<style scoped>
.queue-hint {
  flex: none;
}

.group + .group {
  margin-top: 16px;
}

.group-label {
  font-size: 12px;
  color: var(--muted);
  font-weight: 600;
  margin-bottom: 6px;
}
</style>
