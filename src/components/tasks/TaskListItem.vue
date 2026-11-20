<template>
  <div class="task-row">
    <div class="task-main">
      <div class="task-name">
        <span class="name-text">{{ displayName }}</span>
        <span class="tag" :class="{ 'tag-warn': isCommand }">{{ typeLabel }}</span>
        <span v-if="!task.enabled" class="tag">已停用</span>
      </div>

      <div class="task-sched">
        <span class="cron-chip">
          <AppIcon name="clock" />
          {{ scheduleText }}
        </span>
        <span v-if="once && task.runAt">指定 {{ formatDateTime(task.runAt, true) }}</span>
        <span>下次运行：{{ nextText }}</span>
      </div>

      <div v-if="task.running" class="task-live">
        <StatusPill status="running" />
        <span>开始于 {{ lastTime }}</span>
        <span class="hint">
          {{ isCommand ? "终端命令正在后台执行，完成后自动刷新结果。" : "提醒任务正在触发，完成后自动刷新结果。" }}
        </span>
      </div>

      <div v-else class="task-last">
        <span>上次：</span>
        <StatusPill :status="task.lastStatus" />
        <span>{{ lastTime }}</span>
        <span v-if="lastOutput && !lastError" class="task-output" :title="task.lastOutput">
          {{ lastOutput }}
        </span>
        <span v-if="lastError" class="task-err" :title="task.lastError">{{ lastError }}</span>
      </div>

      <div v-if="commandBlocked" class="hint blocked-hint">
        命令执行已在设置中关闭，此任务不会触发，也无法立即运行。
      </div>
    </div>

    <div class="task-side">
      <n-switch
        :value="task.enabled"
        :loading="busyAction === 'toggle'"
        :disabled="task.running || busy"
        :title="task.enabled ? '停用' : '启用'"
        @update:value="onToggle"
      />

      <div class="task-actions">
        <n-button
          size="small"
          :disabled="task.running || commandBlocked || busy"
          :loading="busyAction === 'run'"
          @click="emit('run')"
        >
          <template #icon>
            <AppIcon name="play" :size="16" />
          </template>
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

        <n-button
          class="icon-btn"
          size="small"
          quaternary
          :disabled="task.running || busy"
          title="编辑"
          aria-label="编辑"
          @click="emit('edit')"
        >
          <template #icon>
            <AppIcon name="pencil" :size="16" />
          </template>
        </n-button>

        <n-button
          class="icon-btn"
          size="small"
          quaternary
          :disabled="busy"
          title="复制任务"
          aria-label="复制任务"
          @click="emit('clone')"
        >
          <template #icon>
            <AppIcon name="copy" :size="16" />
          </template>
        </n-button>

        <n-button
          class="icon-btn"
          size="small"
          quaternary
          :disabled="busy"
          title="运行记录"
          aria-label="运行记录"
          @click="emit('runs')"
        >
          <template #icon>
            <AppIcon name="history" :size="16" />
          </template>
        </n-button>

        <n-popconfirm positive-text="删除" negative-text="取消" @positive-click="emit('delete')">
          <template #trigger>
            <n-button
              class="icon-btn"
              size="small"
              quaternary
              type="error"
              :loading="busyAction === 'delete'"
              :disabled="task.running || busy"
              title="删除"
              aria-label="删除"
            >
              <template #icon>
                <AppIcon name="trash" :size="16" />
              </template>
            </n-button>
          </template>
          删除任务「{{ displayName }}」？运行记录也会一并移除。
        </n-popconfirm>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NPopconfirm, NSwitch } from "naive-ui";
import type { Task } from "../../types";
import AppIcon from "../AppIcon.vue";
import StatusPill from "../StatusPill.vue";
import { TASK_TYPE_META, formatDateTime, truncate } from "./task-utils";
import { cronSummary } from "../../utils/cron";

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
  (e: "clone"): void;
  (e: "delete"): void;
  (e: "runs"): void;
}>();

const displayName = computed(() => props.task.name || "未命名任务");
const once = computed(() => props.task.schedule === "once");
const isCommand = computed(() => props.task.type === "command");
const typeLabel = computed(() => (TASK_TYPE_META[props.task.type] ?? TASK_TYPE_META.reminder).label);
const summary = computed(() => cronSummary(props.task.cron));
const scheduleText = computed(() => (once.value ? "仅一次" : summary.value || props.task.cron));
const nextText = computed(() => {
  if (!props.task.enabled || !props.task.nextRun) return "—";
  return formatDateTime(props.task.nextRun, once.value);
});
const lastTime = computed(() =>
  props.task.lastRunAt ? formatDateTime(props.task.lastRunAt, true) : "—",
);
const lastError = computed(() => props.task.lastError);
const lastOutput = computed(() => truncate(props.task.lastOutput, 140));
const busyAction = computed(() => props.busy ?? "");
const busy = computed(() => Boolean(busyAction.value));
const commandBlocked = computed(() => props.task.type === "command" && !props.allowCommands);

function onToggle(value: boolean): void {
  emit("toggle", value);
}
</script>

<style scoped>
.task-row {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  padding: 16px 0;
  border-bottom: 1px solid var(--border);
}

.task-row:first-child {
  padding-top: 0;
}

.task-row:last-child {
  border-bottom: 0;
  padding-bottom: 0;
}

.task-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.task-name {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: var(--fs-14);
  font-weight: 600;
  color: var(--text);
}

.name-text {
  min-width: 0;
  overflow-wrap: anywhere;
}

.task-sched {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: -2px;
  font-size: var(--fs-12);
  color: var(--muted);
}

.cron-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 1px 8px;
  border-radius: 6px;
  background: var(--primary-soft);
  color: var(--primary);
  font-size: var(--fs-12);
  font-weight: 500;
}

.cron-chip :deep(.ic) {
  width: 12px;
  height: 12px;
}

.task-live {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: var(--fs-12);
}

.task-last {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: var(--fs-12);
  color: var(--muted);
  min-width: 0;
}

.task-output {
  max-width: 420px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  color: var(--text-2);
}

.task-err {
  flex-basis: 100%;
  min-width: 0;
  color: var(--danger);
  font-family: var(--font-mono);
  font-size: var(--fs-12);
  white-space: pre-wrap;
  word-break: break-all;
}

.blocked-hint {
  color: var(--warning);
}

.task-side {
  flex: none;
  display: flex;
  align-items: center;
  gap: 6px;
  padding-top: 2px;
}

.task-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}

.task-side :deep(.icon-btn) {
  width: var(--h-ctl-sm);
  padding: 0;
}

@media (max-width: 760px) {
  .task-row {
    flex-direction: column;
    gap: 10px;
  }

  .task-side {
    padding-top: 0;
    flex-wrap: wrap;
  }

  .task-actions {
    justify-content: flex-start;
  }
}
</style>
