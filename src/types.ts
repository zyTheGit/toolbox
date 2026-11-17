export type ProviderId =
  | "microsoft"
  | "google"
  | "openai"
  | "deepl"
  | "baidu"
  | "youdao";

export type Direction = "auto" | "zh2en" | "en2zh";

export type JobStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "canceled"
  | "interrupted";

export type TaskType = "command" | "reminder";

export type ScheduleKind = "cron" | "once";

export type TaskStatus =
  | ""
  | "running"
  | "success"
  | "failed"
  | "canceled"
  | "interrupted"
  | "skipped";

export interface ToolStatus {
  ok: boolean;
  path: string;
  version: string;
}

export interface ToolsStatus {
  ytdlp: ToolStatus;
  ffmpeg: ToolStatus;
}

export interface ToolPaths {
  ytdlp: string;
  ffmpeg: string;
}

export interface StartupConfig {
  startMinimized: boolean;
  delaySeconds: number;
}

export interface OpenAiConfig {
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface DeepLConfig {
  apiKey: string;
  endpoint: string;
}

export interface BaiduConfig {
  appId: string;
  secretKey: string;
}

export interface YoudaoConfig {
  appKey: string;
  appSecret: string;
}

export interface TranslateConfig {
  provider: ProviderId;
  fallback: ProviderId[];
  ttsRate: number;
  openai: OpenAiConfig;
  deepl: DeepLConfig;
  baidu: BaiduConfig;
  youdao: YoudaoConfig;
}

export interface HistoryConfig {
  enabled: boolean;
  maxEntries: number;
}

export interface SecurityConfig {
  allowCommands: boolean;
}

export interface NotifyConfig {
  download: boolean;
  task: boolean;
}

export interface QuickTranslateConfig {
  enabled: boolean;
  shortcut: string;
}

export interface SecretEntryView {
  id: string;
  name: string;
  service: string;
  note: string;
  expiresOn: string;
  createdAt: string;
  updatedAt: string;
  builtin: boolean;
  masked: string;
  daysLeft: number | null;
  expired: boolean;
}

export interface SecretEntryInput {
  id: string;
  name: string;
  service: string;
  secret: string;
  note: string;
  expiresOn: string;
}

export interface AppConfig {
  schemaVersion: number;
  downloadDir: string;
  downloadConcurrency: number;
  fragmentThreads: number;
  proxy: string;
  toolDownloadMirror: string;
  toolPaths: ToolPaths;
  startup: StartupConfig;
  translate: TranslateConfig;
  history: HistoryConfig;
  security: SecurityConfig;
  notify: NotifyConfig;
  quickTranslate: QuickTranslateConfig;
}

export interface ProviderInfo {
  id: ProviderId;
  name: string;
  requiresKey: boolean;
  ready: boolean;
}

export interface AutostartStatus {
  supported: boolean;
  enabled: boolean;
  path: string;
  expectedPath: string;
}

export interface TranslateRequest {
  text: string;
  direction: Direction;
  provider?: ProviderId | null;
}

export interface TranslateResult {
  text: string;
  provider: ProviderId;
  providerName: string;
  source: string;
  target: string;
  latencyMs: number;
}

export interface HistoryEntry {
  id: string;
  text: string;
  result: string;
  source: string;
  target: string;
  direction: Direction;
  provider: ProviderId;
  providerName: string;
  favorite: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface HistoryQuery {
  keyword?: string;
  direction?: Direction | "";
  provider?: ProviderId | "";
  favoritesOnly?: boolean;
  limit?: number;
  offset?: number;
}

export interface HistoryPage {
  entries: HistoryEntry[];
  total: number;
}

export interface ToolProgress {
  tool: string;
  phase: string;
  pct: number | null;
  received: number;
  total: number;
}

export interface ToolInstallResult {
  tool: string;
  path: string;
  version: string;
  tools: ToolsStatus;
}

export interface JobProgress {
  percent: number;
  speed: string;
  eta: string;
  downloaded: string;
  total: string;
}

export interface Job {
  id: string;
  url: string;
  quality: string;
  outDir: string;
  name: string;
  threads: number;
  referer: string;
  cookiesBrowser: string;
  proxy: string;
  extraArgs: string;
  status: JobStatus;
  progress: JobProgress;
  log: string[];
  error: string;
  file: string;
  createdAt: string;
  startedAt: string;
  finishedAt: string;
}

export interface DownloadRequest {
  urls: string[];
  quality: string;
  outDir: string | null;
  fileName: string | null;
  threads: number | null;
  referer: string;
  cookiesBrowser: string;
  proxy: string;
  extraArgs: string;
}

export interface TaskPayload {
  url: string;
  quality: string;
  text: string;
  direction: Direction;
  saveFile: string;
  command: string;
  workDir: string;
  shell: string;
  timeoutSeconds: number;
  env: string;
  message: string;
}

export interface Task {
  id: string;
  name: string;
  type: TaskType;
  cron: string;
  schedule: ScheduleKind;
  runAt: string;
  enabled: boolean;
  payload: TaskPayload;
  createdAt: string;
  updatedAt: string;
  lastRunAt: string;
  lastStatus: TaskStatus;
  lastOutput: string;
  lastError: string;
  lastJobId: string;
  running: boolean;
  nextRun: string | null;
}

export interface TaskInput {
  name: string;
  type: TaskType;
  cron: string;
  schedule: ScheduleKind;
  runAt: string;
  enabled: boolean;
  payload: TaskPayload;
}

export interface TaskRun {
  id: string;
  taskId: string;
  trigger: string;
  status: TaskStatus;
  startedAt: string;
  finishedAt: string;
  durationMs: number;
  exitCode: number | null;
  output: string;
  error: string;
  jobId: string;
  command: string;
}

export interface SecretStatus {
  openai: boolean;
  deepl: boolean;
  baidu: boolean;
  youdao: boolean;
}

export interface SecretPatch {
  openaiApiKey?: string | null;
  deeplApiKey?: string | null;
  baiduSecretKey?: string | null;
  youdaoAppSecret?: string | null;
}

export interface AppSnapshot {
  version: string;
  platform: string;
  config: AppConfig;
  tools: ToolsStatus;
  tasks: Task[];
  jobs: Job[];
  providers: ProviderInfo[];
  autostart: AutostartStatus;
  tasksPaused: boolean;
  secrets: SecretStatus;
  warnings: string[];
}

export interface ConfigResult {
  config: AppConfig;
  tools: ToolsStatus;
  providers: ProviderInfo[];
  secrets: SecretStatus;
}
