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
              <div class="brand-mark">T</div>
              <div>
                <div class="brand-name">Toolbox</div>
                <div class="brand-sub">桌面工具箱</div>
              </div>
            </div>

            <nav class="nav">
              <button
                v-for="item in navItems"
                :key="item.key"
                class="nav-item"
                :class="{ active: page === item.key }"
                :aria-current="page === item.key ? 'page' : undefined"
                @click="page = item.key"
              >
                <span class="nav-icon" v-html="item.icon"></span>
                <span>{{ item.label }}</span>
              </button>
            </nav>

            <div class="sidebar-foot">
              <div class="tool-row" :aria-label="toolLabel('yt-dlp', store.tools?.ytdlp.ok)">
                <span>yt-dlp</span>
                <span class="tool-state">
                  <span class="dot" :class="{ ok: store.tools?.ytdlp.ok }"></span>
                  {{ store.tools?.ytdlp.ok ? "就绪" : "缺失" }}
                </span>
              </div>
              <div class="tool-row" :aria-label="toolLabel('ffmpeg', store.tools?.ffmpeg.ok)">
                <span>ffmpeg</span>
                <span class="tool-state">
                  <span class="dot" :class="{ ok: store.tools?.ffmpeg.ok }"></span>
                  {{ store.tools?.ffmpeg.ok ? "就绪" : "缺失" }}
                </span>
              </div>
              <div class="version">v{{ store.version || "0.1.0" }}</div>
            </div>
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
              <div v-show="page === 'translate'" class="view"><TranslateView /></div>
              <div v-show="page === 'download'" class="view"><DownloadView /></div>
              <div v-show="page === 'tasks'" class="view"><TasksView /></div>
              <div v-show="page === 'settings'" class="view"><SettingsView /></div>
            </template>
          </main>
        </div>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch, defineAsyncComponent } from "vue";
import {
  NAlert,
  NButton,
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
  NSpin,
  dateZhCN,
  zhCN,
  type GlobalThemeOverrides,
} from "naive-ui";
import { connectEvents, loadSnapshot, store } from "./store";
import { events } from "./api";
// 视图异步分包：首屏只加载当前视图，减小主 chunk 体积。
const TranslateView = defineAsyncComponent(() => import("./views/TranslateView.vue"));
const DownloadView = defineAsyncComponent(() => import("./views/DownloadView.vue"));
const TasksView = defineAsyncComponent(() => import("./views/TasksView.vue"));
const SettingsView = defineAsyncComponent(() => import("./views/SettingsView.vue"));

type PageKey = "translate" | "download" | "tasks" | "settings";

const page = ref<PageKey>("translate");

const navItems: Array<{ key: PageKey; label: string; icon: string }> = [
  {
    key: "translate",
    label: "翻译",
    icon: '<svg viewBox="0 0 24 24"><path d="M4 5h10"/><path d="M9 3v2"/><path d="M6 5c.6 3.6 2.8 6.4 6 8"/><path d="M11 5c-.6 3.4-2.6 6.2-5.5 8"/><path d="M4 19l4.5-10L13 19"/><path d="M6 15.5h5"/><path d="M16 8h5"/><path d="M18.5 6v2"/><path d="M15.5 8c.4 2.4 1.8 4.4 3.9 5.7"/><path d="M20 8c-.4 2.4-1.8 4.4-3.9 5.7"/></svg>',
  },
  {
    key: "download",
    label: "m3u8 下载",
    icon: '<svg viewBox="0 0 24 24"><path d="M12 4v10"/><path d="M8 10l4 4 4-4"/><path d="M4 18h16"/><path d="M4 20h16"/></svg>',
  },
  {
    key: "tasks",
    label: "定时任务",
    icon: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8"/><path d="M12 8v4l3 2"/><path d="M4 4l1.5 3"/><path d="M20 4l-1.5 3"/></svg>',
  },
  {
    key: "settings",
    label: "设置",
    icon: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.5-2-3.5-2.3 1a7 7 0 0 0-2-1.2L14.2 3h-4l-.4 2.6a7 7 0 0 0-2 1.2l-2.3-1-2 3.5 2 1.5A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.5 2 3.5 2.3-1a7 7 0 0 0 2 1.2l.4 2.6h4l.4-2.6a7 7 0 0 0 2-1.2l2.3 1 2-3.5-2-1.5c.1-.4.1-.8.1-1.2z"/></svg>',
  },
];

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: "#4f63e7",
    primaryColorHover: "#6678f0",
    primaryColorPressed: "#3f51d1",
    primaryColorSuppl: "#6678f0",
    borderRadius: "8px",
    fontFamily: '"Segoe UI", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif',
  },
};

let unlisteners: Array<() => void> = [];
const reconnecting = ref(false);
const refreshing = ref(false);

function dismissWarning(index: number): void {
  store.warnings.splice(index, 1);
}

function toolLabel(name: string, ok: boolean | undefined): string {
  return `${name} ${ok ? "已就绪" : "缺失"}`;
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
      if (isPage(target)) page.value = target;
    });
    unlisteners.push(off);
  } catch {
    // Navigation is a convenience; the event banner already covers a broken bridge.
  }
}

function isPage(value: string): value is PageKey {
  return value === "translate" || value === "download" || value === "tasks" || value === "settings";
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
  max-width: 1080px;
  margin: 20px auto 0;
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
</style>
