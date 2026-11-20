<template>
  <div class="card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="list" :size="16" />
        下载队列
        <span v-if="store.jobs.length" class="count">· {{ store.jobs.length }}</span>
      </h3>
      <span class="hint">同时下载 {{ downloadConcurrency }} 个</span>
    </div>

    <EmptyState
      v-if="!store.jobs.length"
      title="暂无下载任务"
      description="粘贴视频链接或分享文本，开始第一个下载。"
    />

    <template v-else>
      <div class="group-label">进行中 · {{ activeJobs.length }}</div>
      <div v-if="activeJobs.length">
        <JobRow v-for="job in activeJobs" :key="job.id" :job="job" />
      </div>
      <div v-else class="empty queue-empty">
        <AppIcon name="inbox" />
        <b>暂无进行中的下载</b>
      </div>

      <template v-if="historyJobs.length">
        <div class="group-label history-label">历史 · {{ historyJobs.length }}</div>
        <JobRow v-for="job in historyJobs" :key="job.id" :job="job" />
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
import JobRow from "./JobRow.vue";
import { isActiveJob } from "./jobDisplay";
import { store } from "../../store";

const activeJobs = computed(() => store.jobs.filter(isActiveJob));
const historyJobs = computed(() => store.jobs.filter((job) => !isActiveJob(job)));
const downloadConcurrency = computed(() => store.config?.downloadConcurrency ?? 2);
</script>

<style scoped>
.group-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-12);
  font-weight: 600;
  color: var(--faint);
  letter-spacing: 0.04em;
  margin: 0 0 8px;
}

.history-label {
  margin-top: 16px;
}

.queue-empty {
  padding: 20px;
  gap: 4px;
}

.queue-empty .ic {
  width: 24px;
  height: 24px;
  margin-bottom: 0;
}
</style>
