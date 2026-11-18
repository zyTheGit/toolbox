export type ErrorCode =
  | "NETWORK"
  | "TOOL"
  | "PATH"
  | "PERMISSION"
  | "TIMEOUT"
  | "PARAM"
  | "DATA"
  | "CONFLICT";

export interface AppError {
  code: ErrorCode | "";
  message: string;
  action: string;
}

const ACTIONS: Record<ErrorCode, string> = {
  NETWORK: "检查网络或代理后重试",
  TOOL: "到「设置 → 下载与工具」安装或更新工具",
  PATH: "更换保存目录后重试",
  PERMISSION: "检查目录权限，或更换保存目录",
  TIMEOUT: "稍后重试，或改用更低的画质",
  PARAM: "检查参数后重试",
  DATA: "到「设置 → 备份」恢复数据",
  CONFLICT: "刷新后重试",
};

const PATTERN = /^\[([A-Z_]+)\]\s*/;

export function errorText(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object" && "message" in err) {
    const message = (err as { message?: unknown }).message;
    if (typeof message === "string") return message;
  }
  return "操作失败，请稍后重试";
}

export function parseError(err: unknown): AppError {
  const raw = errorText(err).trim();
  const match = raw.match(PATTERN);
  const code = (match?.[1] ?? "") as ErrorCode | "";
  const message = match ? raw.slice(match[0].length) : raw;
  return {
    code,
    message,
    action: code && code in ACTIONS ? ACTIONS[code as ErrorCode] : "",
  };
}

/// One-line display text: strips the `[CODE]` prefix and appends the
/// actionable next step, so toasts don't leak internal tags.
export function describeError(err: unknown): string {
  const { message, action } = parseError(err);
  return action ? `${message}（${action}）` : message;
}