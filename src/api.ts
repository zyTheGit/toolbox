import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppConfig,
  AppSnapshot,
  AutostartStatus,
  ConfigResult,
  DbHistoryEntry,
  DbOpenInput,
  DbQueryResult,
  DbSchema,
  DbSessionInfo,
  DownloadRequest,
  HistoryPage,
  HistoryQuery,
  ImportOutcome,
  Job,
  JobRetryRequest,
  LlmRequest,
  LlmResponse,
  LlmUsageSummary,
  SecretEntryInput,
  SecretEntryView,
  SecretPatch,
  Task,
  TaskInput,
  TaskRun,
  ToolInstallResult,
  ToolsStatus,
  ToolProgress,
  TranslateRequest,
  TranslateResult,
} from "./types";
import type { UpdateCheck } from "./types/update";
import type { NavTarget } from "./types/nav";

export const api = {
  snapshot: () => invoke<AppSnapshot>("get_app_snapshot"),
  saveConfig: (
    config: AppConfig,
    secretPatch?: SecretPatch | null,
    section: "settings" | "llm" | "security" | "all" = "all",
    baseRevision?: number | null,
  ) =>
    invoke<ConfigResult>("save_config", {
      config,
      secretPatch: secretPatch ?? null,
      section,
      baseRevision: baseRevision ?? null,
    }),
  getTools: (force = false) => invoke<ToolsStatus>("get_tools", { force }),
  installTool: (tool: string) => invoke<ToolInstallResult>("install_tool", { tool }),
  openFolder: (path?: string) => invoke<string>("open_folder", { path: path ?? null }),
  pickFolder: () => invoke<string | null>("pick_folder"),
  translate: (request: TranslateRequest) => invoke<TranslateResult>("translate", { request }),
  historyQuery: (query: HistoryQuery) => invoke<HistoryPage>("history_query", { query }),
  historyDelete: (id: string) => invoke<void>("history_delete", { id }),
  historyClear: () => invoke<void>("history_clear"),
  historySetFavorite: (id: string, favorite: boolean) =>
    invoke<void>("history_set_favorite", { id, favorite }),
  downloadEnqueue: (request: DownloadRequest) => invoke<Job[]>("download_enqueue", { request }),
  jobCancel: (id: string) => invoke<void>("job_cancel", { id }),
  jobRetry: (id: string, request?: JobRetryRequest | null) =>
    invoke<Job>("job_retry", { id, request: request ?? null }),
  jobRemove: (id: string) => invoke<void>("job_remove", { id }),
  jobOpen: (id: string) => invoke<string>("job_open", { id }),
  tasksList: () => invoke<Task[]>("tasks_list"),
  taskCreate: (input: TaskInput) => invoke<Task>("task_create", { input }),
  taskUpdate: (id: string, input: TaskInput) => invoke<Task>("task_update", { id, input }),
  taskDelete: (id: string) => invoke<void>("task_delete", { id }),
  taskRunNow: (id: string) => invoke<TaskRun>("task_run_now", { id }),
  taskCancel: (id: string) => invoke<void>("task_cancel", { id }),
  taskRuns: (taskId: string) => invoke<TaskRun[]>("task_runs", { taskId }),
  tasksSetPaused: (paused: boolean) => invoke<boolean>("tasks_set_paused", { paused }),
  cronPreview: (expr: string) => invoke<string[]>("cron_preview", { expr }),
  autostartGet: () => invoke<AutostartStatus>("autostart_get"),
  autostartSet: (enabled: boolean) => invoke<AutostartStatus>("autostart_set", { enabled }),
  getDataDir: () => invoke<string>("get_data_dir"),
  dataExport: (includeCommands = false) =>
    invoke<string>("data_export", { includeCommands }),
  dataImport: () => invoke<ImportOutcome>("data_import"),
  restartApp: () => invoke<void>("restart_app"),
  checkForUpdate: () => invoke<UpdateCheck>("check_for_update"),
  openReleasePage: (url: string) => invoke<void>("open_release_page", { url }),
  secretsList: () => invoke<SecretEntryView[]>("secrets_list"),
  secretSave: (input: SecretEntryInput) =>
    invoke<SecretEntryView[]>("secret_save", { input }),
  secretDelete: (id: string) => invoke<SecretEntryView[]>("secret_delete", { id }),
  secretReveal: (id: string) => invoke<string>("secret_reveal", { id }),
  llmRun: (request: LlmRequest) => invoke<LlmResponse[]>("llm_run", { request }),
  llmUsage: (days = 7) => invoke<LlmUsageSummary>("llm_usage", { days }),
  dbOpen: (input: DbOpenInput) => invoke<DbSessionInfo>("db_open", { input }),
  dbSchema: (id: string) => invoke<DbSchema>("db_schema", { id }),
  dbSelectDatabase: (id: string, database: string) =>
    invoke<DbSessionInfo>("db_select_database", { id, database }),
  dbQuery: (id: string, sql: string, offset: number, limit: number) =>
    invoke<DbQueryResult>("db_query", { id, sql, offset, limit }),
  dbClose: (id: string) => invoke<void>("db_close", { id }),
  dbCloseAll: () => invoke<void>("db_close_all"),
  dbHistoryList: () => invoke<DbHistoryEntry[]>("db_history_list"),
  dbHistoryReveal: (id: string) => invoke<string>("db_history_reveal", { id }),
  dbHistoryRename: (id: string, name: string) =>
    invoke<DbHistoryEntry[]>("db_history_rename", { id, name }),
  dbPickSqlite: () => invoke<string | null>("db_pick_sqlite"),
};

export const events = {
  onJobUpdated: (cb: (job: Job) => void): Promise<UnlistenFn> =>
    listen<Job>("job-updated", (e) => cb(e.payload)),
  onJobRemoved: (cb: (id: string) => void): Promise<UnlistenFn> =>
    listen<string>("job-removed", (e) => cb(e.payload)),
  onTasksUpdated: (cb: (tasks: Task[]) => void): Promise<UnlistenFn> =>
    listen<Task[]>("tasks-updated", (e) => cb(e.payload)),
  onTaskRunUpdated: (cb: (run: TaskRun) => void): Promise<UnlistenFn> =>
    listen<TaskRun>("task-run-updated", (e) => cb(e.payload)),
  onToolProgress: (cb: (progress: ToolProgress) => void): Promise<UnlistenFn> =>
    listen<ToolProgress>("tool-progress", (e) => cb(e.payload)),
  onTasksPaused: (cb: (paused: boolean) => void): Promise<UnlistenFn> =>
    listen<boolean>("tasks-paused", (e) => cb(e.payload)),
  onNavigate: (cb: (target: NavTarget) => void): Promise<UnlistenFn> =>
    listen<NavTarget>("navigate", (e) => cb(e.payload)),
  onQuickTranslate: (cb: (text: string) => void): Promise<UnlistenFn> =>
    listen<string>("quick-translate", (e) => cb(e.payload)),
};
