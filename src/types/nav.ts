/** 通知点击后的跳转目标；taskId 存在时定位到对应任务。 */
export interface NavTarget {
  page: string;
  taskId?: string | null;
}
