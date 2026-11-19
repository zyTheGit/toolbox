/** 下载任务与请求模型。 */

export type JobStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "canceled"
  | "interrupted";

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
  /** 字幕档位："" / zh / en / both。 */
  subtitles: string;
  /** 仅下载字幕，不下载视频/音频。 */
  subtitleOnly: boolean;
  status: JobStatus;
  progress: JobProgress;
  log: string[];
  error: string;
  file: string;
  createdAt: string;
  startedAt: string;
  finishedAt: string;
  /** 是否在内存中持有下载凭据（不落盘，默认 false）。 */
  hasCredentials?: boolean;
  /** 非致命提示，例如站点字幕语言与内容不符。 */
  notice?: string;
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
  /** 字幕档位："" / zh / en / both。 */
  subtitles: string;
  /** 仅下载字幕，不下载视频/音频。 */
  subtitleOnly: boolean;
}

export interface JobRetryRequest {
  referer?: string; cookiesBrowser?: string; proxy?: string; extraArgs?: string; quality?: string; threads?: number;
}
