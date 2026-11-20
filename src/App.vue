<template>
  <n-config-provider
    :theme-overrides="themeOverrides"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <n-message-provider>
      <n-dialog-provider>
        <div class="app-shell">
          <aside class="sidebar">
            <div class="brand">
              <div class="brand-mark"><AppIcon name="logo" :size="20" /></div>
              <div>
                <div class="brand-name">
                  Toolbox
                  <span v-if="store.version" class="brand-ver">v{{ store.version }}</span>
                </div>
                <div class="brand-sub">桌面工具箱</div>
              </div>
            </div>

            <nav class="nav">
              <template v-for="group in NAV_GROUPS" :key="group.label">
                <div class="nav-group">{{ group.label }}</div>
                <button
                  v-for="item in group.items"
                  :key="item.key"
                  class="nav-item"
                  :class="{ active: page === item.key }"
                  :title="item.label"
                  :aria-current="page === item.key ? 'page' : undefined"
                  @click="page = item.key"
                >
                  <AppIcon :name="item.icon" />
                  <span>{{ item.label }}</span>
                  <em v-if="badgeOf(item.key)" class="nav-badge" :class="{ soft: item.soft }">
                    {{ badgeOf(item.key) }}
                  </em>
                </button>
              </template>
            </nav>
          </aside>

          <main class="main">
            <div v-if="!store.ready" class="boot">
              <n-spin size="large" />
              <p>正在加载…</p>
            </div>
            <template v-else>
              <div v-if="store.error" class="alert-wrap">
                <n-alert type="error" :bordered="false" closable @close="store.error = ''">
                  {{ store.error }}
                </n-alert>
              </div>
              <div v-if="store.warnings.length" class="alert-wrap">
                <n-alert
                  v-for="(warning, index) in store.warnings"
                  :key="warning"
                  type="warning"
                  :bordered="false"
                  closable
                  @close="dismissWarning(index)"
                >
                  {{ warning }}
                </n-alert>
              </div>
              <div v-if="!store.eventsOk" class="alert-wrap">
                <n-alert type="warning" :bordered="false">
                  <div class="event-alert">
                    <div>
                      实时更新不可用{{ store.eventsError ? `：${store.eventsError}` : "" }}。下载进度和任务状态可能不会自动刷新。
                    </div>
                    <div class="event-actions">
                      <n-button size="small" :loading="reconnecting" @click="reconnectEvents">
                        重连
                      </n-button>
                      <n-button size="small" quaternary :loading="refreshing" @click="refreshSnapshot">
                        手动刷新
                      </n-button>
                    </div>
                  </div>
                </n-alert>
              </div>
              <KeepAlive>
                <component :is="views[page]" class="view" />
              </KeepAlive>
            </template>
          </main>
        </div>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
  NSpin,
  dateZhCN,
  zhCN,
} from "naive-ui";
import { connectEvents, loadSnapshot, store } from "./store";
import { events } from "./api";
import { themeOverrides } from "./ui/theme";
import type { IconName } from "./ui/icons";
import AppIcon from "./components/AppIcon.vue";
import TranslateView from "./views/TranslateView.vue";
import DownloadView from "./views/DownloadView.vue";
import TasksView from "./views/TasksView.vue";
import DevView from "./views/DevView.vue";
import DbView from "./views/DbView.vue";
import AiView from "./views/AiView.vue";
import SettingsView from "./views/SettingsView.vue";

type PageKey = "translate" | "download" | "tasks" | "dev" | "db" | "ai" | "settings";

const page = ref<PageKey>("translate");

// 静态导入视图并按页面缓存：桌面端资源在本地，切换零等待，避免异步分包首切白屏
const views = {
  translate: TranslateView,
  download: DownloadView,
  tasks: TasksView,
  dev: DevView,
  db: DbView,
  ai: AiView,
  settings: SettingsView,
};

interface NavItem {
  key: PageKey;
  label: string;
  icon: IconName;
  /** 软徽标：数量多但非紧急（如启用中的任务数） */
  soft?: boolean;
}

const NAV_GROUPS: Array<{ label: string; items: NavItem[] }> = [
  {
    label: "常用",
    items: [
      { key: "translate", label: "翻译", icon: "translate" },
      { key: "download", label: "下载", icon: "download" },
      { key: "tasks", label: "定时任务", icon: "clock", soft: true },
    ],
  },
  {
    label: "工具",
    items: [
      { key: "dev", label: "开发工具", icon: "code" },
      { key: "db", label: "数据库", icon: "database" },
      { key: "ai", label: "AI 工具", icon: "sparkles" },
    ],
  },
  {
    label: "系统",
    items: [{ key: "settings", label: "设置", icon: "settings" }],
  },
];

const activeJobs = computed(
  () => store.jobs.filter((job) => job.status === "queued" || job.status === "running").length,
);
const enabledTasks = computed(() => store.tasks.filter((task) => task.enabled).length);

function badgeOf(key: PageKey): string {
  if (key === "download" && activeJobs.value) return String(activeJobs.value);
  if (key === "tasks" && enabledTasks.value) return String(enabledTasks.value);
  return "";
}

let unlisteners: Array<() => void> = [];
const reconnecting = ref(false);
const refreshing = ref(false);

function dismissWarning(index: number): void {
  store.warnings.splice(index, 1);
}

function disposeEvents(): void {
  for (const off of unlisteners) off();
  unlisteners = [];
}

async function attachEvents(): Promise<void> {
  disposeEvents();
  unlisteners = await connectEvents();
  try {
    const off = await events.onNavigate((target) => {
      if (!isPage(target.page)) return;
      store.pendingTaskId = target.taskId ?? null;
      page.value = target.page;
    });
    unlisteners.push(off);
  } catch {
    // Navigation is a convenience; the event banner already covers a broken bridge.
  }
}

function isPage(value: string): value is PageKey {
  return (
    value === "translate" ||
    value === "download" ||
    value === "tasks" ||
    value === "dev" ||
    value === "db" ||
    value === "ai" ||
    value === "settings"
  );
}

async function reconnectEvents(): Promise<void> {
  if (reconnecting.value) return;
  reconnecting.value = true;
  try {
    await attachEvents();
    await loadSnapshot();
  } finally {
    reconnecting.value = false;
  }
}

async function refreshSnapshot(): Promise<void> {
  if (refreshing.value) return;
  refreshing.value = true;
  try {
    await loadSnapshot();
  } finally {
    refreshing.value = false;
  }
}

watch(page, () => {
  if (!store.eventsOk) void loadSnapshot();
});

onMounted(async () => {
  await attachEvents();
  await loadSnapshot();
});

onBeforeUnmount(() => {
  disposeEvents();
});
</script>

<style scoped>
.alert-wrap {
  margin: 20px 0 0;
  padding: 0 32px;
}

.alert-wrap :deep(.n-alert + .n-alert) {
  margin-top: 8px;
}

.event-alert {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.event-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

@media (max-width: 900px) {
  .alert-wrap {
    padding: 0 16px;
  }
}
</style>
