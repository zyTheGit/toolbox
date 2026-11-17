<template>
  <div class="task-item">
    <div class="task-main">
      <div class="task-title-row">
        <span class="task-name ellipsis">{{ displayName }}</span>
        <n-tag size="small" :type="typeMeta.tag" :bordered="false">{{ typeMeta.label }}</n-tag>
        <n-tag v-if="task.running" size="small" type="info" :bordered="false">运行中</n-tag>
        <n-tag v-else-if="!task.enabled" size="small" :bordered="false">已停用</n-tag>
      </div>

      <div class="task-meta">
        <span class="cron-chip mono">{{ once ? "单次" : summary || task.cron }}</span>
        <span v-if="once && task.runAt">指定 {{ formatDateTime(task.runAt, true) }}</span>
        <span>下次运行 {{ task.nextRun ? formatDateTime(task.nextRun, once) : "—" }}</span>
      </div>

      <div class="task-last">
        <StatusPill :status="task.lastStatus" />
        <span>上次 {{ task.lastRunAt ? formatDateTime(task.lastRunAt, true) : "—" }}</span>
        <span v-if="lastError" class="task-msg error-text" :title="task.lastError">{{ lastError }}</span>
        <span v-else-if="lastOutput" class="task-msg task-output" :title="task.lastOutput">{{ lastOutput }}</span>
      </div>

      <div v-if="commandBlocked" class="hint blocked-hint">
        命令执行已在设置中关闭，此任务不会触发，也无法立即运行。
      </div>
    </div>

    <div class="task-side">
      <n-switch
        :value="task.enabled"
        :disabled="task.running || busy"
        @update:value="onToggle"
      />
      <div class="task-actions">
        <n-button
          size="small"
          type="primary"
          :disabled="task.running || commandBlocked || busy"
          :loading="busyAction === 'run'"
          @click="emit('run')"
        >
          立即运行
        </n-button>
        <n-button
          v-if="task.running"
          size="small"
          type="error"
          ghost
          :loading="busyAction === 'cancel'"
          :disabled="busy"
          @click="emit('cancel')"
        >
          取消
        </n-button>
        <n-button size="small" :disabled="task.running || busy" @click="emit('edit')">编辑</n-button>
        <n-button size="small" :disabled="busy" @click="emit('runs')">运行记录</n-button>
        <n-popconfirm
          positive-text="删除"
          negative-text="取消"
          @positive-click="emit('delete')"
        >
          <template #trigger>
            <n-button size="small" type="error" ghost :disabled="task.running || busy">删除</n-button>
          </template>
          删除任务「{{ displayName }}」？运行记录也会一并移除。
        </n-popconfirm>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NPopconfirm, NSwitch, NTag } from "naive-ui";
import type { Task } from "../../types";
import StatusPill from "../StatusPill.vue";
import { TASK_TYPE_META, cronSummary, formatDateTime, truncate } from "./task-utils";

const props = defineProps<{
  task: Task;
  allowCommands: boolean;
  busy?: string;
}>();

const emit = defineEmits<{
  (e: "toggle", enabled: boolean): void;
  (e: "run"): void;
  (e: "cancel"): void;
  (e: "edit"): void;
  (e: "delete"): void;
  (e: "runs"): void;
}>();

const displayName = computed(() => props.task.name || "未命名任务");
const once = computed(() => props.task.schedule === "once");
const typeMeta = computed(() => TASK_TYPE_META[props.task.type] ?? TASK_TYPE_META.reminder);
const summary = computed(() => cronSummary(props.task.cron));
const busyAction = computed(() => props.busy ?? "");
const busy = computed(() => Boolean(busyAction.value));
const commandBlocked = computed(() => props.task.type === "command" && !props.allowCommands);
const lastError = computed(() => truncate(props.task.lastError));
const lastOutput = computed(() => truncate(props.task.lastOutput));

function onToggle(value: boolean): void {
  emit("toggle", value);
}
</script>

<style scoped>
.task-item {
  display: flex;
  align-items: flex-start;
  gap: 18px;
  padding: 16px 20px;
}

.task-item + .task-item {
  border-top: 1px solid var(--border);
}

.task-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.task-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.task-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  max-width: 420px;
}

.task-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 12px;
  font-size: 12px;
  color: var(--muted);
}

.cron-chip {
  padding: 1px 6px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--panel-soft);
  color: #394152;
}

.task-last {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 10px;
  font-size: 12px;
  color: var(--muted);
  min-width: 0;
}

.task-msg {
  max-width: 460px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-output {
  color: #4a5468;
}

.blocked-hint {
  color: var(--warning);
}

.task-side {
  flex: none;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 10px;
}

.task-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}
</style>