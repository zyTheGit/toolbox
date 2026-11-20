/** 下载任务卡的纯展示逻辑：状态、阶段标签、失败建议与时间文案。 */

import type { Job } from "../../types";
import { parseError, type AppError } from "../../utils/errors";

export function isActiveJob(job: Job): boolean {
  return job.status === "queued" || job.status === "running";
}

const QUALITY_LABEL: Record<string, string> = {
  best: "最佳画质",
  "1080": "1080p 及以下",
  "720": "720p 及以下",
  "480": "480p 及以下",
  audio: "仅音频",
};

const SUBTITLE_LABEL: Record<string, string> = {
  zh: "中文字幕",
  "zh-mt": "中文机翻",
  en: "英文字幕",
  both: "中英字幕",
};

/** 任务卡左上角画质 / 格式标签。 */
export function jobQuality(job: Job): string {
  if (job.subtitleOnly) return "仅字幕";
  return QUALITY_LABEL[job.quality] ?? job.quality;
}

export function jobSubtitle(job: Job): string {
  return SUBTITLE_LABEL[job.subtitles] ?? "";
}

const FAILURE_RULES: Array<[RegExp, string]> = [
  [
    /429|too many requests|rate.?limit|限流/i,
    "站点限流（YouTube 字幕常见）：等待几分钟后重试，或在高级选项里选择 Cookie 来源",
  ],
  [/yt-dlp|ffmpeg|not found|未找到|找不到/, "到「设置 → 下载与工具」安装或更新工具"],
  [/permission|denied|拒绝访问|只读|read-only|权限/, "检查目录权限，或更换保存目录"],
  [/directory|folder|no such file|目录|路径/, "更换保存目录后重试"],
  [/timeout|timed out|超时/, "稍后重试，或改用更低的画质"],
  [/network|connection|resolve|dns|tls|ssl|网络|连接/, "检查网络或代理后重试"],
];

/** 失败 / 中断任务的错误信息与下一步建议；其他状态返回 null。 */
export function jobFailure(job: Job): AppError | null {
  if (job.status !== "failed" && job.status !== "interrupted") return null;
  if (!job.error) return null;
  const parsed = parseError(job.error);
  if (parsed.action) return parsed;
  const guessed = FAILURE_RULES.find(([pattern]) => pattern.test(parsed.message))?.[1] ?? "";
  return { ...parsed, action: guessed };
}

export function jobPercent(job: Job): number {
  const value = Number(job.progress?.percent);
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(100, Math.round(value)));
}

/** 已下载 / 总大小；缺少数据时返回空串。 */
export function jobSize(job: Job): string {
  const downloaded = job.progress?.downloaded ?? "";
  const total = job.progress?.total ?? "";
  if (downloaded && total) return `${downloaded} / ${total}`;
  return downloaded || total;
}

const DAY_MS = 24 * 60 * 60 * 1000;

/** 历史任务时间：今天 / 昨天用相对说法，更早的显示日期。 */
export function formatJobTime(iso: string): string {
  if (!iso) return "";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const pad = (value: number) => String(value).padStart(2, "0");
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  const today = new Date();
  const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime();
  const startOfDate = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  if (startOfDate === startOfToday) return `今天 ${time}`;
  if (startOfDate === startOfToday - DAY_MS) return `昨天 ${time}`;
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${time}`;
}
