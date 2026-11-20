<template>
  <div class="job-log">
    <div v-if="failed && job.error" class="error-text log-error">{{ job.error }}</div>
    <div v-if="lines.length" class="term log-term">{{ lines.join("\n") }}</div>
    <div v-else class="hint">暂无日志输出</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Job } from "../../types";

const props = defineProps<{ job: Job }>();

const failed = computed(
  () => props.job.status === "failed" || props.job.status === "interrupted",
);

// 只渲染最近 200 行，避免长任务日志拖慢滚动。
const lines = computed(() => props.job.log.slice(-200));
</script>

<style scoped>
.job-log {
  margin-top: 10px;
}

.log-error {
  margin-bottom: 8px;
}

.log-term {
  max-height: 220px;
  overflow: auto;
}
</style>
