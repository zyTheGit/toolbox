/** 任务表单的重复规则状态与 cron 组合/解析（纯函数，便于拆分与测试）。 */

import { parseWeekdays } from "../../utils/cron";
import { formatDateTime } from "./task-utils";

export type RepeatMode = "daily" | "weekly" | "monthly" | "interval" | "hourly" | "custom";

export interface RepeatState {
  mode: RepeatMode;
  time: string;
  weekdays: number[];
  day: number;
  interval: number;
  minute: number;
}

export interface ParsedCron {
  mode: Exclude<RepeatMode, "custom">;
  hour?: number;
  minute?: number;
  weekdays?: number[];
  day?: number;
  interval?: number;
}

export function defaultRepeatState(): RepeatState {
  return { mode: "daily", time: "17:00", weekdays: [1], day: 1, interval: 5, minute: 0 };
}

/** 默认执行时间：一小时后，精确到秒。 */
export function defaultRunAt(): string {
  return formatDateTime(new Date(Date.now() + 60 * 60 * 1000).toISOString(), true);
}

export function pad2(value: number): string {
  return String(value).padStart(2, "0");
}

function timeParts(value: string): [number, number] {
  const match = /^(\d{1,2}):(\d{1,2})$/.exec(value.trim());
  if (!match) return [17, 0];
  return [Math.min(23, Number(match[1])), Math.min(59, Number(match[2]))];
}

export function composeCron(repeat: RepeatState): string {
  if (repeat.mode === "custom") return "";
  const [hour, minute] = timeParts(repeat.time);
  switch (repeat.mode) {
    case "daily":
      return `${minute} ${hour} * * *`;
    case "weekly": {
      if (!repeat.weekdays.length) return "";
      const days = [...repeat.weekdays].sort((a, b) => a - b).join(",");
      return `${minute} ${hour} * * ${days}`;
    }
    case "monthly":
      return `${minute} ${hour} ${repeat.day} * *`;
    case "interval":
      return `*/${repeat.interval} * * * *`;
    case "hourly":
      return `${repeat.minute} * * * *`;
    default:
      return "";
  }
}

export function parseCron(expr: string): ParsedCron | null {
  const parts = String(expr ?? "").trim().split(/\s+/);
  if (parts.length !== 5) return null;
  const [min, hour, dom, mon, dow] = parts;
  if (mon !== "*") return null;

  const step = /^\*\/(\d+)$/.exec(min);
  if (step && hour === "*" && dom === "*" && dow === "*") {
    const value = Number(step[1]);
    return value >= 1 && value <= 59 ? { mode: "interval", interval: value } : null;
  }

  if (!/^\d+$/.test(min)) return null;
  const minute = Number(min);
  if (minute > 59) return null;

  if (hour === "*" && dom === "*" && dow === "*") {
    return { mode: "hourly", minute };
  }
  if (!/^\d+$/.test(hour)) return null;
  const h = Number(hour);
  if (h > 23) return null;

  if (dom === "*" && dow === "*") return { mode: "daily", hour: h, minute };

  if (dom === "*" && /^[0-9,-]+$/.test(dow)) {
    const days = parseWeekdays(dow);
    return days ? { mode: "weekly", hour: h, minute, weekdays: days } : null;
  }

  if (/^\d+$/.test(dom) && dow === "*") {
    const day = Number(dom);
    if (day < 1 || day > 31) return null;
    return { mode: "monthly", hour: h, minute, day };
  }

  return null;
}

/** 按 cron 表达式回填重复规则；无法识别时切到自定义模式。 */
export function applyCronToRepeat(repeat: RepeatState, expr: string): void {
  const parsed = parseCron(expr);
  if (!parsed) {
    repeat.mode = "custom";
    return;
  }
  repeat.mode = parsed.mode;
  if (parsed.hour != null && parsed.minute != null) {
    repeat.time = `${pad2(parsed.hour)}:${pad2(parsed.minute)}`;
  }
  if (parsed.mode === "weekly") repeat.weekdays = parsed.weekdays ?? [1];
  if (parsed.mode === "monthly") repeat.day = parsed.day ?? 1;
  if (parsed.mode === "interval") repeat.interval = parsed.interval ?? 5;
  if (parsed.mode === "hourly") repeat.minute = parsed.minute ?? 0;
}
