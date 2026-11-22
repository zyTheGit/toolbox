import type { ShortcutsConfig } from "./shortcuts";

export type ProviderId =
  | "microsoft"
  | "google"
  | "openai"
  | "deepl"
  | "baidu"
  | "youdao";

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

/** 下载事件的细分开关；总开关仍是 NotifyConfig.download。 */
export interface NotifyEvents {
  success: boolean;
  failed: boolean;
  allDone: boolean;
}

export interface NotifyConfig {
  download: boolean;
  task: boolean;
  events: NotifyEvents;
  sound: boolean;
}

export interface QuickTranslateConfig {
  enabled: boolean;
  shortcut: string;
}

export interface LlmProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  keyEntryId: string;
  temperature: number;
}

export interface LlmTemplate {
  id: string;
  name: string;
  content: string;
}

export interface LlmConfig {
  providers: LlmProviderConfig[];
  templates: LlmTemplate[];
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
  shortcuts: ShortcutsConfig;
  llm: LlmConfig;
}
