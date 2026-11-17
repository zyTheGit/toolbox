import type { Task, TaskInput, TaskPayload, TaskType } from "../../types";
import { describeError } from "../../utils/errors";

export const TASK_TYPE_META: Record<
  TaskType,
  { label: string; tag: "default" | "info" | "success" | "warning" | "error" }
> = {
  command: { label: "终端", tag: "warning" },
  reminder: { label: "提醒", tag: "default" },
};

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
