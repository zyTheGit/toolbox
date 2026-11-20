<template>
  <div class="page">
    <PageHead
      title="定时任务"
      icon="clock"
      subtitle="按每天 / 每周等规则重复执行，或指定年月日时分秒只执行一次。"
    >
      <n-button :loading="pausing" @click="togglePaused">
        <template #icon>
          <AppIcon :name="store.tasksPaused ? 'play' : 'pause'" :size="16" />
        </template>
        {{ store.tasksPaused ? "全部恢复" : "全部暂停" }}
      </n-button>
      <n-button type="primary" @click="openCreate">
        <template #icon>
          <AppIcon name="plus" :size="16" />
        </template>
        新建任务
      </n-button>
    </PageHead>

    <div v-if="store.tasksPaused || !allowCommands" class="notices">
      <div v-if="store.tasksPaused" class="alert alert-warn">
        <AppIcon name="pause" :size="16" />
        <span>全部定时任务已暂停，手动运行仍可用。</span>
      </div>
      <div v-if="!allowCommands" class="alert alert-warn">
        <AppIcon name="alert" :size="16" />
        <span>命令执行已在设置中关闭，终端任务不会触发。可在「设置 → 命令执行」中开启。</span>
      </div>
    </div>

    <div class="filter-row">
      <button class="chip" :class="{ on: filter === 'all' }" @click="filter = 'all'">
        全部 {{ counts.all }}
      </button>
      <button class="chip" :class="{ on: filter === 'on' }" @click="filter = 'on'">
        已启用 {{ counts.on }}
      </button>
      <button class="chip" :class="{ on: filter === 'off' }" @click="filter = 'off'">
        已停用 {{ counts.off }}
      </button>
    </div>

    <div class="card">
      <EmptyState
        v-if="!store.tasks.length"
        icon="clock"
        title="还没有定时任务"
        description="新建一个提醒或终端任务，到点自动执行。"
      >
        <n-button type="primary" size="small" @click="openCreate">
          <template #icon>
            <AppIcon name="plus" :size="16" />
          </template>
          新建任务
        </n-button>
      </EmptyState>

      <EmptyState
        v-else-if="!visibleTasks.length"
        icon="filter"
        title="没有符合条件的任务"
        description="切换筛选条件查看其他任务。"
      >
        <n-button size="small" @click="filter = 'all'">查看全部</n-button>
      </EmptyState>

      <template v-else>
        <TaskListItem
          v-for="task in visibleTasks"
          :key="task.id"
          :task="task"
          :allow-commands="allowCommands"
          :busy="busy[task.id]"
          @toggle="handleToggle(task, $event)"
          @run="handleRun(task)"
          @cancel="handleCancel(task)"
          @edit="openEdit(task)"
          @clone="openClone(task)"
          @delete="handleDelete(task)"
          @runs="openRuns(task)"
        />
      </template>
    </div>

    <TaskFormModal
      v-model:show="formShow"
      :task="editing"
      :template="cloning"
      :allow-commands="allowCommands"
      @saved="syncTasks"
    />

    <TaskRunsDrawer ref="runsDrawer" v-model:show="runsShow" :task="runsTask" />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { NButton, useMessage } from "naive-ui";
import { api, events } from "../api";
import { store } from "../store";
import type { Task } from "../types";
import AppIcon from "../components/AppIcon.vue";
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

type TaskFilter = "all" | "on" | "off";
const filter = ref<TaskFilter>("all");
const counts = computed(() => ({
  all: store.tasks.length,
  on: store.tasks.filter((task) => task.enabled).length,
  off: store.tasks.filter((task) => !task.enabled).length,
}));
const visibleTasks = computed(() =>
  filter.value === "all"
    ? store.tasks
    : store.tasks.filter((task) => (filter.value === "on" ? task.enabled : !task.enabled)),
);

const formShow = ref(false);
const editing = ref<Task | null>(null);
const cloning = ref<Task | null>(null);
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

// 事件桥正常时由 tasks-updated 推送，避免命令返回后再整表拉取造成重复刷新。
async function syncTasks(): Promise<void> {
  if (!store.eventsOk) await refreshTasks();
}

async function togglePaused(): Promise<void> {
  if (pausing.value) return;
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
  cloning.value = null;
  formShow.value = true;
}

function openEdit(task: Task): void {
  if (task.running) return;
  editing.value = task;
  cloning.value = null;
  formShow.value = true;
}

/** 复制任务：打开新建表单并预填源任务，保存时生成一条独立任务 */
function openClone(task: Task): void {
  editing.value = null;
  cloning.value = task;
  formShow.value = true;
}

function openRuns(task: Task): void {
  runsTask.value = task;
  runsShow.value = true;
}

// 通知点击可能早于任务列表加载，等任务数据就绪后再定位。
watch(
  () => [store.pendingTaskId, store.tasks] as const,
  () => {
    const id = store.pendingTaskId;
    if (!id) return;
    const task = store.tasks.find((item) => item.id === id);
    if (!task) return;
    store.pendingTaskId = null;
    openRuns(task);
  },
  { immediate: true },
);

async function handleToggle(task: Task, enabled: boolean): Promise<void> {
  if (busy[task.id]) return;
  busy[task.id] = "toggle";
  try {
    await api.taskUpdate(task.id, { ...taskToInput(task), enabled });
    await syncTasks();
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
    await syncTasks();
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
    await syncTasks();
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
    await syncTasks();
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

.filter-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 14px;
}
</style>
