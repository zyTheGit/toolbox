import { reactive } from "vue";
import { api, events } from "./api";
import type {
  AppConfig,
  AppSnapshot,
  AutostartStatus,
  Job,
  ProviderInfo,
  SecretEntryView,
  SecretStatus,
  Task,
  ToolsStatus,
  ToolProgress,
} from "./types";

interface StoreState {
  ready: boolean;
  error: string;
  version: string;
  platform: string;
  config: AppConfig | null;
  tools: ToolsStatus | null;
  tasks: Task[];
  jobs: Job[];
  providers: ProviderInfo[];
  autostart: AutostartStatus | null;
  tasksPaused: boolean;
  secrets: SecretStatus;
  secretEntries: SecretEntryView[];
  secretEntriesLoaded: boolean;
  warnings: string[];
  configRevision: number;
  toolProgress: Record<string, ToolProgress>;
  eventsOk: boolean;
  eventsError: string;
}

export const store = reactive<StoreState>({
  ready: false,
  error: "",
  version: "",
  platform: "",
  config: null,
  tools: null,
  tasks: [],
  jobs: [],
  providers: [],
  autostart: null,
  tasksPaused: false,
  secrets: { openai: false, deepl: false, baidu: false, youdao: false },
  secretEntries: [],
  secretEntriesLoaded: false,
  warnings: [],
  configRevision: 0,
  toolProgress: {},
  eventsOk: false,
  eventsError: "",
});

export function applySnapshot(snapshot: AppSnapshot): void {
  store.version = snapshot.version;
  store.platform = snapshot.platform;
  store.config = snapshot.config;
  store.tools = snapshot.tools;
  store.tasks = snapshot.tasks ?? [];
  store.jobs = snapshot.jobs ?? [];
  store.providers = snapshot.providers ?? [];
  store.autostart = snapshot.autostart;
  store.tasksPaused = snapshot.tasksPaused;
  store.secrets = snapshot.secrets;
  store.warnings = snapshot.warnings ?? [];
  store.configRevision = snapshot.configRevision ?? 0;
  store.ready = true;
  store.error = "";
}

export async function loadSnapshot(): Promise<void> {
  try {
    applySnapshot(await api.snapshot());
  } catch (err) {
    store.error = err instanceof Error ? err.message : String(err);
    store.ready = true;
  }
}

export async function loadSecretEntries(force = false): Promise<SecretEntryView[]> {
  if (store.secretEntriesLoaded && !force) return store.secretEntries;
  store.secretEntries = await api.secretsList();
  store.secretEntriesLoaded = true;
  return store.secretEntries;
}

function upsertJob(job: Job): void {
  const index = store.jobs.findIndex((item) => item.id === job.id);
  if (index >= 0) store.jobs[index] = job;
  else store.jobs.unshift(job);
}

export async function connectEvents(): Promise<Array<() => void>> {
  const registrations: Array<() => Promise<() => void>> = [
    () => events.onJobUpdated(upsertJob),
    () =>
      events.onJobRemoved((id) => {
        store.jobs = store.jobs.filter((item) => item.id !== id);
      }),
    () =>
      events.onTasksUpdated((tasks) => {
        store.tasks = tasks;
      }),
    () =>
      events.onTasksPaused((paused) => {
        store.tasksPaused = paused;
      }),
    () =>
      events.onToolProgress((progress) => {
        store.toolProgress[progress.tool] = progress;
      }),
    () =>
      events.onTaskRunUpdated(() => {
        // Task views refresh their own run history on demand.
      }),
  ];

  const unlisteners: Array<() => void> = [];
  try {
    for (const register of registrations) {
      unlisteners.push(await register());
    }
    store.eventsOk = true;
    store.eventsError = "";
    return unlisteners.map((fn) => () => void fn());
  } catch (err) {
    for (const off of unlisteners) off();
    store.eventsOk = false;
    store.eventsError = err instanceof Error ? err.message : String(err);
    return [];
  }
}

export function toolReady(tool: "ytdlp" | "ffmpeg"): boolean {
  return Boolean(store.tools?.[tool]?.ok);
}
