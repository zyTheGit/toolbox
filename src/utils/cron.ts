/** cron 表达式的预设、周字段解析与人类可读描述。纯函数，便于单测。 */

export const CRON_PRESETS: Array<{ label: string; value: string }> = [
  { label: "每分钟", value: "* * * * *" },
  { label: "每 5 分钟", value: "*/5 * * * *" },
  { label: "每小时整点", value: "0 * * * *" },
  { label: "每天 09:00", value: "0 9 * * *" },
  { label: "每天 17:00", value: "0 17 * * *" },
  { label: "工作日 09:00", value: "0 9 * * 1-5" },
  { label: "每周一 09:00", value: "0 9 * * 1" },
  { label: "每月 1 日 03:00", value: "0 3 1 * *" },
];

/** 解析周的列举与范围写法：1,3,5 / 1-5，7 归一为周日 0 */
export function parseWeekdays(spec: string): number[] | null {
  const days = new Set<number>();
  for (const part of spec.split(",")) {
    const range = /^(\d)-(\d)$/.exec(part.trim());
    if (range) {
      const start = Number(range[1]);
      const end = Number(range[2]);
      if (start > end || end > 7) return null;
      for (let day = start; day <= end; day += 1) days.add(day % 7);
      continue;
    }
    if (!/^\d$/.test(part.trim())) return null;
    const day = Number(part);
    if (day > 7) return null;
    days.add(day % 7);
  }
  return days.size ? [...days].sort((a, b) => a - b) : null;
}

const WEEKDAYS = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];

/** 把常见 cron 表达式转成中文描述；无法识别时返回 null */
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

  if (dom === "*" && /^[0-9,-]+$/.test(dow)) {
    const days = parseWeekdays(dow);
    if (!days) return null;
    const isWorkdays = days.length === 5 && days.join(",") === "1,2,3,4,5";
    if (isWorkdays) return `工作日 ${time}`;
    return `每${days.map((day) => WEEKDAYS[day]).join("、")} ${time}`;
  }

  if (/^\d+$/.test(dom) && dow === "*") {
    const day = Number(dom);
    if (day < 1 || day > 31) return null;
    return `每月 ${day} 日 ${time}`;
  }

  return null;
}
