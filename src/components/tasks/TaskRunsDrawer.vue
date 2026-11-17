<template>
  <n-drawer
    :show="show"
    :width="560"
    placement="right"
    @update:show="onShowChange"
  >
    <n-drawer-content :title="`运行记录 · ${displayName}`" closable>
      <div v-if="loading" class="drawer-state">
        <n-spin size="medium" />
      </div>
      <n-alert v-else-if="loadError" type="error" :bordered="false" class="drawer-alert">
        {{ loadError }}
      </n-alert>
      <EmptyState
        v-else-if="!runs.length"
        title="暂无运行记录"
        description="任务触发后会在这里显示执行详情。"
      />
      <div v-else class="run-list">
        <div v-for="run in runs" :key="run.id" class="run-card">
          <div class="run-head">
            <div class="run-head-main">
              <n-tag size="small" :type="run.trigger === 'manual' ? 'info' : 'default'" :bordered="false">
                {{ triggerLabel(run.trigger) }}
              </n-tag>
              <StatusPill :status="run.status" />
              <span class="run-time">{{ formatDateTime(run.startedAt, true) }}</span>
              <span class="run-time">· {{ durationText(run) }}</span>
            </div>
            <n-button text size="tiny" @click="toggle(run.id)">
              {{ expanded.has(run.id) ? "收起" : "展开" }}
            </n-button>
          </div>

          <div v-if="hasMeta(run)" class="run-meta">
            <span v-if="run.exitCode !== null && run.exitCode !== undefined">exitCode {{ run.exitCode }}</span>
            <span v-if="run.jobId" class="mono">jobId {{ run.jobId }}</span>
          </div>

          <div v-if="run.command" class="run-command mono">{{ run.command }}</div>

          <div v-if="expanded.has(run.id)" class="run-detail">
            <div v-if="run.output" class="run-block">
              <div class="run-block-title">输出</div>
              <pre class="run-pre mono">{{ run.output }}</pre>
            </div>
            <div v-if="run.error" class="run-block">
              <div class="run-block-title run-block-error">错误</div>
              <pre class="run-pre mono run-pre-error">{{ run.error }}</pre>
            </div>
            <div v-if="!run.output && !run.error" class="hint">本次运行没有输出。</div>
          </div>
        </div>
      </div>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NDrawer,
  NDrawerContent,
  NSpin,
  NTag,
} from "naive-ui";
import { api } from "../../api";
import type { Task, TaskRun } from "../../types";
import EmptyState from "../EmptyState.vue";
import StatusPill from "../StatusPill.vue";
import { errorText, formatDateTime } from "./task-utils";

const props = defineProps<{
  show: boolean;
  task: Task | null;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
}>();

const runs = ref<TaskRun[]>([]);
const loading = ref(false);
const loadError = ref("");
const expanded = ref<Set<string>>(new Set());

const displayName = computed(() => props.task?.name || "未命名任务");

watch(
  () => [props.show, props.task?.id] as const,
  ([show]) => {
    if (show) {
      expanded.value = new Set();
      void refresh();
    }
  },
);

async function refresh(): Promise<void> {
  const task = props.task;
  if (!task) {
    runs.value = [];
    return;
  }
  loading.value = true;
  loadError.value = "";
  try {
    runs.value = await api.taskRuns(task.id);
  } catch (err) {
    loadError.value = errorText(err);
    runs.value = [];
  } finally {
    loading.value = false;
  }
}

function hasMeta(run: TaskRun): boolean {
  return (run.exitCode !== null && run.exitCode !== undefined) || Boolean(run.jobId);
}

function triggerLabel(trigger: string): string {
  if (trigger === "schedule") return "定时";
  if (trigger === "manual") return "手动";
  return trigger || "未知";
}

function onShowChange(value: boolean): void {
  emit("update:show", value);
}

function durationText(run: TaskRun): string {
  if (run.status === "running") return "进行中";
  const ms = run.durationMs;
  if (!ms) return "—";
  if (ms < 1000) return `${ms} ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)} 秒`;
  const minutes = Math.floor(ms / 60000);
  const seconds = Math.round((ms % 60000) / 1000);
  if (minutes < 60) return `${minutes} 分 ${seconds} 秒`;
  return `${Math.floor(minutes / 60)} 小时 ${minutes % 60} 分`;
}

function toggle(id: string): void {
  const next = new Set(expanded.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expanded.value = next;
}

defineExpose({ refresh });
</script>

<style scoped>
.drawer-state {
  display: flex;
  justify-content: center;
  padding: 48px 0;
  color: var(--muted);
}

.drawer-alert {
  margin-bottom: 12px;
}

.run-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.run-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 14px;
  background: var(--panel);
}

.run-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.run-head-main {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  min-width: 0;
}

.run-time {
  font-size: 12px;
  color: var(--muted);
}

.run-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 8px;
  font-size: 12px;
  color: var(--muted);
}

.run-command {
  margin-top: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  background: var(--panel-soft);
  border: 1px solid var(--border);
  color: #394152;
  word-break: break-all;
  white-space: pre-wrap;
}

.run-detail {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.run-block-title {
  font-size: 12px;
  font-weight: 600;
  color: #4a5468;
  margin-bottom: 4px;
}

.run-block-error {
  color: var(--danger);
}

.run-pre {
  margin: 0;
  padding: 10px;
  max-height: 300px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel-soft);
  color: #2c3344;
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.6;
}

.run-pre-error {
  color: #b03030;
  background: #fdf5f5;
  border-color: #f0d4d4;
}
</style>