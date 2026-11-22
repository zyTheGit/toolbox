export * from "./types/download";
export * from "./types/config";
export * from "./types/shortcuts";
import type { Job } from "./types/download";
import type { AppConfig, ProviderId, ToolsStatus } from "./types/config";

export type Direction = "auto" | "zh2en" | "en2zh";
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

export interface LlmRequest {
  providerIds: string[];
  system: string;
  prompt: string;
  temperature?: number | null;
  maxTokens?: number | null;
}

export interface LlmResponse {
  providerId: string;
  providerName: string;
  model: string;
  content: string;
  error: string;
  latencyMs: number;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

export interface LlmUsageDay {
  date: string;
  calls: number;
  promptTokens: number;
  completionTokens: number;
}

export interface LlmUsageModel {
  model: string;
  providerName: string;
  calls: number;
  promptTokens: number;
  completionTokens: number;
}

export interface LlmUsageSummary {
  days: LlmUsageDay[];
  models: LlmUsageModel[];
  totalCalls: number;
  totalPromptTokens: number;
  totalCompletionTokens: number;
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

export type DbKind = "sqlite" | "postgres" | "mysql";

export interface DbColumn {
  name: string;
  dataType: string;
  nullable: boolean;
  primaryKey: boolean;
}

export interface DbTable {
  name: string;
  schema: string;
  kind: "table" | "view";
  columns: DbColumn[];
}

export interface DbSchema {
  tables: DbTable[];
}

export interface DbOpenInput {
  kind: DbKind;
  sqlitePath?: string | null;
  connectionString?: string | null;
}

export interface DbDatabase {
  name: string;
  tableCount: number;
}

export interface DbSessionInfo {
  id: string;
  kind: DbKind;
  label: string;
  schema: DbSchema;
  /** MySQL 可切换的数据库；其它类型为空。 */
  databases: DbDatabase[];
}

export interface DbHistoryEntry {
  id: string;
  kind: DbKind;
  /** 自定义记录名称，默认为脱敏连接目标。 */
  name: string;
  /** 脱敏后的连接目标，不含密码。 */
  label: string;
  updatedAt: string;
}

export interface DbQueryResult {
  columns: string[];
  rows: Array<Array<string | number | boolean | null>>;
  offset: number;
  limit: number;
  hasMore: boolean;
  elapsedMs: number;
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
  /** 任务版本号，用于并发写入的期望版本校验（0 表示尚未启用）。 */
  revision?: number;
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

/** 密钥平台保护级别：Windows=DPAPI，Unix=仅文件权限（等同明文），未知=unknown。 */
export type SecretProtection = "dpapi" | "file-permissions" | "unknown";
export interface SecretStatus {
  openai: boolean;
  deepl: boolean;
  baidu: boolean;
  youdao: boolean;
  protection?: SecretProtection;
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
  /** 配置版本号，用于保存前的期望版本校验（0 表示尚未启用）。 */
  configRevision?: number;
  /** 备份恢复后的首次提示，启动时读取一次后由后端删除。 */
  restoreNotice?: string | null;
}
export interface ImportOutcome {
  restoredConfig: boolean; restoredHistory: number; restoredTasks: number; restoredJobs: number;
  disabledCommandTasks: number; backupPath: string; restartRequired: boolean;
}
export interface ConfigResult {
  config: AppConfig;
  tools: ToolsStatus;
  providers: ProviderInfo[];
  secrets: SecretStatus;
  /** 保存成功后的配置版本号，前端用它作为下一次保存的基准。 */
  configRevision: number;
}
