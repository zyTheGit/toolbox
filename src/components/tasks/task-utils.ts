import type { Task, TaskInput, TaskPayload, TaskType } from "../../types";
import { describeError } from "../../utils/errors";

export const TASK_TYPE_META: Record<
  TaskType,
  { label: string; tag: "default" | "info" | "success" | "warning" | "error" }
> = {
  command: { label: "终端", tag: "warning" },
  reminder: { label: "提醒", tag: "default" },
};

export const CRON_PRESETS: Array<{ label: string; value: string }> = [
  { label: "每天 17:00", value: "0 17 * * *" },
  { label: "每天 03:00", value: "0 3 * * *" },
  { label: "每小时", value: "0 * * * *" },
  { label: "每周一 09:00", value: "0 9 * * 1" },
  { label: "每 5 分钟", value: "*/5 * * * *" },
];

export function emptyPayload(): TaskPayload {
  return {
    url: "",
    quality: "best",
    text: "",
    direction: "auto",
    saveFile: "",
    command: "",
    workDir: "",
    shell: "cmd",
    timeoutSeconds: 600,
    env: "",
    message: "",
  };
}

export function taskToInput(task: Task): TaskInput {
  return {
    name: task.name,
    type: task.type,
    cron: task.cron,
    schedule: task.schedule ?? "cron",
    runAt: task.runAt ?? "",
    enabled: task.enabled,
    payload: { ...emptyPayload(), ...task.payload },
  };
}

export function errorText(err: unknown): string {
  return describeError(err);
}

export function truncate(text: string, max = 160): string {
  const value = String(text ?? "").replace(/\s+/g, " ").trim();
  return value.length > max ? `${value.slice(0, max)}…` : value;
}

export function formatDateTime(iso: string, withSeconds = false): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const pad = (value: number) => String(value).padStart(2, "0");
  const base = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
  return withSeconds ? `${base}:${pad(date.getSeconds())}` : base;
}

const WEEKDAYS = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];

export function cronSummary(expr: string): string | null {
  const parts = String(expr ?? "").trim().split(/\s+/);
  if (parts.length !== 5) return null;
  const [min, hour, dom, mon, dow] = parts;

  if (min === "*" && hour === "*" && dom === "*" && mon === "*" && dow === "*") {
    return "每分钟";
  }

  const minStep = /^\*\/(\d+)$/.exec(min);
  if (minStep && hour === "*" && dom === "*" && mon === "*" && dow === "*") {
    const step = Number(minStep[1]);
    return step > 0 && step < 60 ? `每 ${step} 分钟` : null;
  }

  if (!/^\d+$/.test(min)) return null;
  const minute = Number(min);
  if (minute > 59) return null;

  if (hour === "*" && dom === "*" && mon === "*" && dow === "*") {
    return minute === 0 ? "每小时整点" : `每小时第 ${minute} 分钟`;
  }

  const hourStep = /^\*\/(\d+)$/.exec(hour);
  if (hourStep && dom === "*" && mon === "*" && dow === "*") {
    const step = Number(hourStep[1]);
    if (step > 0 && step < 24) {
      return `每 ${step} 小时（第 ${minute} 分钟）`;
    }
  }

  if (!/^\d+$/.test(hour) || mon !== "*") return null;
  const hourNum = Number(hour);
  if (hourNum > 23) return null;
  const time = `${String(hourNum).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;

  if (dom === "*" && dow === "*") return `每天 ${time}`;

  if (dom === "*" && /^\d+(,\d+)*$/.test(dow)) {
    const days = [...new Set(dow.split(",").map((value) => Number(value) % 7))].sort(
      (a, b) => a - b,
    );
    if (days.some((day) => day > 6)) return null;
    return `每${days.map((day) => WEEKDAYS[day]).join("、")} ${time}`;
  }

  if (/^\d+$/.test(dom) && dow === "*") {
    const day = Number(dom);
    if (day < 1 || day > 31) return null;
    return `每月 ${day} 日 ${time}`;
  }

  return null;
}