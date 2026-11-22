import type { AppConfig, ProviderId } from "../types";
import { DEFAULT_SHORTCUTS } from "./shortcut";

const PROVIDER_ORDER: ProviderId[] = ["microsoft", "google", "openai", "deepl", "baidu", "youdao"];

function clamp(value: unknown, min: number, max: number, fallback: number): number {
  const number = Number(value);
  return Number.isFinite(number) ? Math.min(max, Math.max(min, Math.round(number))) : fallback;
}

/** 保存前规整设置段的数值与回退顺序，避免把越界值写进配置。 */
export function normalizeSettingsDraft(draft: AppConfig): AppConfig {
  const payload = JSON.parse(JSON.stringify(draft)) as AppConfig;
  payload.translate.provider = payload.translate.provider || "microsoft";
  payload.translate.fallback = PROVIDER_ORDER.filter(
    (provider) =>
      provider !== payload.translate.provider && payload.translate.fallback.includes(provider),
  );
  payload.history.maxEntries = clamp(payload.history.maxEntries, 1, 2000, 200);
  payload.downloadConcurrency = clamp(payload.downloadConcurrency, 1, 5, 2);
  payload.fragmentThreads = clamp(payload.fragmentThreads, 1, 16, 4);
  const delay = Number(payload.startup.delaySeconds);
  payload.startup.delaySeconds = Number.isFinite(delay) ? delay : 0;
  const ttsRate = Number(payload.translate.ttsRate);
  payload.translate.ttsRate = Number.isFinite(ttsRate) ? Math.min(2, Math.max(0.5, ttsRate)) : 1;
  if (!payload.shortcuts.translateSubmit.trim()) {
    payload.shortcuts.translateSubmit = DEFAULT_SHORTCUTS.translateSubmit;
  }
  if (!payload.shortcuts.dbRun.trim()) {
    payload.shortcuts.dbRun = DEFAULT_SHORTCUTS.dbRun;
  }
  return payload;
}
