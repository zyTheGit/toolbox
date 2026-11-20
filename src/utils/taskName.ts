/** 复制任务时在原名后追加「（副本）」 */
export function copyTaskName(name: string): string {
  const base = name.trim() || "未命名任务";
  return `${base}（副本）`;
}
