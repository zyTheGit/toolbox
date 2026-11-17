<template>
  <div class="page">
    <PageHead title="定时任务" subtitle="按每天/每周等规则重复执行，或指定年月日时分秒只执行一次。">
      <n-button :loading="pausing" @click="togglePaused">
        {{ store.tasksPaused ? "全部恢复" : "全部暂停" }}
      </n-button>
      <n-button type="primary" @click="openCreate">新建任务</n-button>
    </PageHead>

    <div v-if="store.tasksPaused || !allowCommands" class="notices">
      <n-alert v-if="store.tasksPaused" type="warning" :bordered="false">
        全部定时任务已暂停，手动运行仍可用。
      </n-alert>
      <n-alert v-if="!allowCommands" type="default" :bordered="false">
        命令执行已在设置中关闭，终端任务不会触发。
      </n-alert>
    </div>

    <div class="card tasks-card">
      <EmptyState
        v-if="!store.tasks.length"
        title="还没有定时任务"
        description="可以设置一次性提醒，或按每天/每周等规则重复执行终端命令。"
      >
        <n-button type="primary" @click="openCreate">新建任务</n-button>
      </EmptyState>
      <template v-else>
        <TaskListItem
          v-for="task in store.tasks"
          :key="task.id"
          :task="task"
          :allow-commands="allowCommands"
          :busy="busy[task.id]"
          @toggle="handleToggle(task, $event)"
          @run="handleRun(task)"
          @cancel="handleCancel(task)"
          @edit="openEdit(task)"
          @delete="handleDelete(task)"
          @runs="openRuns(task)"
        />
      </template>
    </div>

    <TaskFormModal
      v-model:show="formShow"
      :task="editing"
      :allow-commands="allowCommands"
      @saved="refreshTasks"
    />

    <TaskRunsDrawer ref="runsDrawer" v-model:show="runsShow" :task="runsTask" />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { NAlert, NButton, useMessage } from "naive-ui";
import { api, events } from "../api";
import { store } from "../store";
import type { Task } from "../types";
import PageHead from "../components/PageHead.vue";
import EmptyState from "../components/EmptyState.vue";
import TaskListItem from "../components/tasks/TaskListItem.vue";
import TaskFormModal from "../components/tasks/TaskFormModal.vue";
import TaskRunsDrawer from "../components/tasks/TaskRunsDrawer.vue";
import { errorText, taskToInput } from "../components/tasks/task-utils";

const message = useMessage();

const allowCommands = computed(() => store.config?.security.allowCommands === true);
const pausing = ref(false);
const busy = reactive<Record<string, string>>({});

const formShow = ref(false);
const editing = ref<Task | null>(null);
const runsShow = ref(false);
const runsTask = ref<Task | null>(null);
const runsDrawer = ref<InstanceType<typeof TaskRunsDrawer> | null>(null);

let unlistenRun: (() => void) | null = null;

onMounted(async () => {
  try {
    unlistenRun = await events.onTaskRunUpdated((run) => {
      if (runsShow.value && run.taskId === runsTask.value?.id) {
        void runsDrawer.value?.refresh();
      }
    });
  } catch {
    // Run history still loads on demand when the Tauri event bridge is unavailable.
  }
});

onBeforeUnmount(() => {
  unlistenRun?.();
  unlistenRun = null;
});

async function refreshTasks(): Promise<void> {
  try {
    store.tasks = await api.tasksList();
  } catch {
    // The global tasks-updated event keeps the list fresh in the normal path.
  }
}

async function togglePaused(): Promise<void> {
  pausing.value = true;
  try {
    store.tasksPaused = await api.tasksSetPaused(!store.tasksPaused);
  } catch (err) {
    message.error(errorText(err));
  } finally {
    pausing.value = false;
  }
}

function openCreate(): void {
  editing.value = null;
  formShow.value = true;
}

function openEdit(task: Task): void {
  if (task.running) return;
  editing.value = task;
  formShow.value = true;
}

function openRuns(task: Task): void {
  runsTask.value = task;
  runsShow.value = true;
}

async function handleToggle(task: Task, enabled: boolean): Promise<void> {
  if (busy[task.id]) return;
  busy[task.id] = "toggle";
  try {
    await api.taskUpdate(task.id, { ...taskToInput(task), enabled });
    await refreshTasks();
  } catch (err) {
    message.error(errorText(err));
  } finally {
    delete busy[task.id];
  }
}

async function handleRun(task: Task): Promise<void> {
  if (busy[task.id]) return;
  if (task.type === "command" && !allowCommands.value) {
    message.warning("命令执行已在设置中关闭，无法运行终端任务。");
    return;
  }
  busy[task.id] = "run";
  try {
    const run = await api.taskRunNow(task.id);
    if (run.status === "skipped") {
      message.warning(run.error || "任务已跳过");
    } else {
      message.success("已开始运行");
    }
    await refreshTasks();
    if (runsShow.value && runsTask.value?.id === task.id) {
      void runsDrawer.value?.refresh();
    }
  } catch (err) {
    message.error(errorText(err));
  } finally {
    delete busy[task.id];
  }
}

async function handleCancel(task: Task): Promise<void> {
  if (busy[task.id]) return;
  busy[task.id] = "cancel";
  try {
    await api.taskCancel(task.id);
    message.success("已请求取消");
    await refreshTasks();
  } catch (err) {
    message.error(errorText(err));
  } finally {
    delete busy[task.id];
  }
}

async function handleDelete(task: Task): Promise<void> {
  if (busy[task.id]) return;
  busy[task.id] = "delete";
  try {
    await api.taskDelete(task.id);
    message.success("任务已删除");
    if (runsTask.value?.id === task.id) {
      runsShow.value = false;
      runsTask.value = null;
    }
    await refreshTasks();
  } catch (err) {
    message.error(errorText(err));
  } finally {
    delete busy[task.id];
  }
}
</script>

<style scoped>
.notices {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}

.tasks-card {
  padding: 0;
  overflow: hidden;
}
</style>