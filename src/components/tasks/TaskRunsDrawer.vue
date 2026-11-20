<template>
  <n-drawer :show="show" :width="640" placement="right" @update:show="onShowChange">
    <n-drawer-content :title="`运行记录 · ${displayName}`" closable>
      <div v-if="loading" class="drawer-state">
        <n-spin size="medium" />
      </div>

      <n-alert v-else-if="loadError" type="error" :bordered="false" class="drawer-alert">
        {{ loadError }}
      </n-alert>

      <EmptyState
        v-else-if="!runs.length"
        icon="history"
        title="暂无运行记录"
        description="任务触发后会在这里显示执行详情。"
      />

      <div v-else class="table-wrap">
        <table class="tbl runs-table">
          <thead>
            <tr>
              <th>触发</th>
              <th>状态</th>
              <th>开始时间</th>
              <th>耗时</th>
              <th>退出码</th>
              <th class="detail-col"></th>
            </tr>
          </thead>
          <tbody>
            <template v-for="run in runs" :key="run.id">
              <tr class="run-row" @click="toggle(run.id)">
                <td>
                  <span class="tag" :class="{ 'tag-primary': run.trigger === 'manual' }">
                    {{ triggerLabel(run.trigger) }}
                  </span>
                </td>
                <td><StatusPill :status="run.status" /></td>
                <td class="num">{{ formatDateTime(run.startedAt, true) }}</td>
                <td class="num">{{ durationText(run) }}</td>
                <td class="num">{{ exitText(run) }}</td>
                <td class="detail-col">
                  <AppIcon
                    :name="expanded.has(run.id) ? 'chevron-down' : 'chevron-right'"
                    :size="16"
                  />
                </td>
              </tr>
              <tr v-if="expanded.has(run.id)" class="detail-row">
                <td colspan="6">
                  <div v-if="run.command" class="run-command mono">{{ run.command }}</div>
                  <div v-if="run.jobId" class="hint run-job mono">jobId {{ run.jobId }}</div>
                  <div v-if="run.output" class="run-block">
                    <div class="run-block-title">输出</div>
                    <pre class="run-pre mono">{{ run.output }}</pre>
                  </div>
                  <div v-if="run.error" class="run-block">
                    <div class="run-block-title run-block-error">错误</div>
                    <pre class="run-pre run-pre-error mono">{{ run.error }}</pre>
                  </div>
                  <div v-if="!run.output && !run.error" class="hint">本次运行没有输出。</div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NAlert, NDrawer, NDrawerContent, NSpin } from "naive-ui";
import { api } from "../../api";
import type { Task, TaskRun } from "../../types";
import AppIcon from "../AppIcon.vue";
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

function triggerLabel(trigger: string): string {
  if (trigger === "schedule") return "定时";
  if (trigger === "manual") return "手动";
  return trigger || "未知";
}

function exitText(run: TaskRun): string {
  return run.exitCode === null || run.exitCode === undefined ? "—" : String(run.exitCode);
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

function onShowChange(value: boolean): void {
  emit("update:show", value);
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

.runs-table {
  white-space: nowrap;
}

.runs-table .run-row {
  cursor: pointer;
}

.runs-table .detail-col {
  width: 32px;
  text-align: right;
  color: var(--faint);
}

.runs-table .detail-row td {
  background: var(--surface-soft);
  white-space: normal;
}

.runs-table .detail-row:hover td {
  background: var(--surface-soft);
}

.run-command {
  margin-bottom: 8px;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-2);
  word-break: break-all;
  white-space: pre-wrap;
}

.run-job {
  margin-bottom: 8px;
}

.run-block + .run-block {
  margin-top: 10px;
}

.run-block-title {
  margin-bottom: 4px;
  font-size: var(--fs-12);
  font-weight: 600;
  color: var(--text-2);
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
  background: var(--surface-soft);
  color: var(--text-2);
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.6;
}

.run-pre-error {
  color: var(--danger);
  background: var(--danger-soft);
  border-color: var(--border);
}
</style>
