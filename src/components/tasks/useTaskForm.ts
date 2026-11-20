/** 任务表单状态与提交逻辑，供 TaskFormModal 与分区子组件共用。 */

import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from "vue";
import { useDialog, useMessage } from "naive-ui";
import { api } from "../../api";
import { store } from "../../store";
import type { ScheduleKind, Task, TaskInput, TaskPayload, TaskType } from "../../types";
import { emptyPayload, errorText } from "./task-utils";
import { copyTaskName } from "../../utils/taskName";
import { enableCommandsWithConflict } from "../../utils/securityConfig";
import { cronSummary } from "../../utils/cron";
import {
  applyCronToRepeat,
  composeCron,
  defaultRepeatState,
  defaultRunAt,
  type RepeatState,
} from "./task-cron";

export interface TaskFormState {
  name: string;
  type: TaskType;
  schedule: ScheduleKind;
  cron: string;
  runAt: string;
  enabled: boolean;
  payload: TaskPayload;
}

interface TaskFormProps {
  show: boolean;
  task: Task | null;
  template?: Task | null;
  allowCommands: boolean;
}

type TaskFormEmits = {
  (e: "update:show", value: boolean): void;
  (e: "saved"): void;
};

export function useTaskForm(props: TaskFormProps, emit: TaskFormEmits) {
  const message = useMessage();
  const dialog = useDialog();

  const form = reactive<TaskFormState>({
    name: "",
    type: "command",
    schedule: "cron",
    cron: "0 17 * * *",
    runAt: "",
    enabled: true,
    payload: emptyPayload(),
  });
  const errors = reactive<Record<string, string>>({});
  const apiError = ref("");
  const submitting = ref(false);

  const repeat = reactive<RepeatState>(defaultRepeatState());
  const preview = ref<string[]>([]);
  const previewError = ref("");
  const previewing = ref(false);

  let previewTimer: ReturnType<typeof setTimeout> | null = null;
  let previewSeq = 0;
  let submitSeq = 0;
  let initialSnapshot = "";

  const isEdit = computed(() => Boolean(props.task));
  const isWindows = computed(() => !store.platform || /win/i.test(store.platform));
  const summary = computed(() => cronSummary(form.cron));
  const cronValidation = computed(() =>
    form.schedule === "cron" && repeat.mode === "custom"
      ? errors.cron || previewError.value
      : "",
  );

  const shellOptions = computed(() =>
    isWindows.value
      ? [
          { label: "cmd", value: "cmd" },
          { label: "PowerShell", value: "powershell" },
        ]
      : [{ label: "sh", value: "sh" }],
  );

  watch(
    () => props.show,
    (visible) => {
      if (visible) init();
      else clearPreviewTimer();
    },
  );

  watch(
    () => form.type,
    () => {
      clearErrors();
      ensureShell();
    },
  );

  watch(
    () => form.cron,
    () => {
      errors.cron = "";
      schedulePreview();
    },
  );

  watch(() => form.schedule, schedulePreview);

  watch(
    () => [
      repeat.mode,
      repeat.time,
      repeat.weekdays,
      repeat.day,
      repeat.interval,
      repeat.minute,
    ],
    () => {
      if (repeat.mode === "custom") return;
      form.cron = composeCron(repeat);
    },
    { deep: true },
  );

  function clearErrors(): void {
    for (const key of Object.keys(errors)) delete errors[key];
  }

  function init(): void {
    submitSeq += 1;
    const task = props.task;
    const template = task ? null : props.template ?? null;
    const source = task ?? template;
    form.name = task ? task.name : template ? copyTaskName(template.name) : "";
    form.type = source?.type ?? "command";
    form.schedule = source?.schedule ?? "cron";
    form.cron = source?.cron || "0 17 * * *";
    form.runAt = source?.runAt || defaultRunAt();
    form.enabled = source?.enabled ?? true;
    form.payload = { ...emptyPayload(), ...(source?.payload ?? {}) };
    Object.assign(repeat, defaultRepeatState());
    applyCronToRepeat(repeat, form.cron);
    ensureShell();
    clearErrors();
    apiError.value = "";
    submitting.value = false;
    schedulePreview();
    // 等重复规则字段归一后再记录初始快照。
    void nextTick(() => {
      initialSnapshot = snapshotForm();
    });
  }

  function snapshotForm(): string {
    return JSON.stringify(form);
  }

  function isDirty(): boolean {
    return snapshotForm() !== initialSnapshot;
  }

  function ensureShell(): void {
    if (form.type !== "command") return;
    if (!shellOptions.value.some((item) => item.value === form.payload.shell)) {
      form.payload.shell = shellOptions.value[0]?.value ?? "sh";
    }
  }

  function clearPreviewTimer(): void {
    if (previewTimer) {
      clearTimeout(previewTimer);
      previewTimer = null;
    }
    previewSeq += 1;
    previewing.value = false;
  }

  function schedulePreview(): void {
    clearPreviewTimer();
    if (form.schedule !== "cron") {
      preview.value = [];
      previewError.value = "";
      return;
    }
    const expr = form.cron.trim();
    if (!expr) {
      preview.value = [];
      previewError.value = "cron 表达式不能为空";
      return;
    }
    preview.value = [];
    previewError.value = "";
    previewing.value = true;
    const seq = previewSeq;
    previewTimer = setTimeout(async () => {
      try {
        const list = await api.cronPreview(expr);
        if (seq !== previewSeq) return;
        preview.value = list.slice(0, 3);
        previewError.value = "";
      } catch (err) {
        if (seq !== previewSeq) return;
        preview.value = [];
        previewError.value = errorText(err);
      } finally {
        if (seq === previewSeq) previewing.value = false;
      }
    }, 300);
  }

  function validate(): boolean {
    const next: Record<string, string> = {};
    if (!form.name.trim()) next.name = "请填写任务名称";
    if (form.schedule === "cron") {
      if (repeat.mode === "weekly" && !repeat.weekdays.length) {
        next.cron = "请至少选择一个星期";
      } else if (!form.cron.trim()) {
        next.cron = "请选择重复方式";
      } else if (previewError.value) {
        next.cron = previewError.value;
      }
    } else {
      const runAt = form.runAt.trim();
      if (!runAt) {
        next.runAt = "请选择执行时间";
      } else if (!/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(runAt)) {
        next.runAt = "时间格式应为 YYYY-MM-DD HH:MM:SS";
      } else {
        const when = new Date(runAt.replace(" ", "T"));
        if (Number.isNaN(when.getTime())) next.runAt = "执行时间无效";
        else if (form.enabled && when.getTime() <= Date.now()) {
          next.runAt = "执行时间必须晚于当前时间";
        }
      }
    }

    if (form.type === "command") {
      if (!form.payload.command.trim()) next.command = "请填写要执行的命令";
      const timeout = Number(form.payload.timeoutSeconds);
      if (!Number.isFinite(timeout) || timeout < 1 || timeout > 86400) {
        next.timeout = "超时需在 1 - 86400 秒之间";
      }
    }

    clearErrors();
    Object.assign(errors, next);
    return Object.keys(next).length === 0;
  }

  function buildInput(): TaskInput {
    const p = form.payload;
    return {
      name: form.name.trim(),
      type: form.type,
      cron: form.schedule === "cron" ? form.cron.trim() : "",
      schedule: form.schedule,
      runAt: form.schedule === "once" ? form.runAt.trim() : "",
      enabled: form.enabled,
      payload: {
        url: p.url.trim(),
        quality: p.quality || "best",
        text: p.text,
        direction: p.direction || "auto",
        saveFile: p.saveFile.trim(),
        command: p.command.trim(),
        workDir: p.workDir.trim(),
        shell: p.shell || shellOptions.value[0].value,
        timeoutSeconds: Number(p.timeoutSeconds) || 600,
        env: p.env.trim(),
        message: p.message.trim(),
      },
    };
  }

  function confirmEnableCommands(): Promise<boolean> {
    return new Promise((resolve) => {
      let settled = false;
      const finish = (value: boolean) => {
        if (settled) return;
        settled = true;
        resolve(value);
      };
      dialog.warning({
        title: "开启命令执行",
        content:
          "该任务会以当前用户权限执行终端命令。需要在设置中开启「允许执行命令」才能保存并运行。",
        positiveText: "开启并保存",
        negativeText: "取消",
        onPositiveClick: async () => {
          try {
            const enabled = await enableCommandsWithConflict(dialog);
            if (!enabled) {
              apiError.value = "未开启命令执行，任务未保存。";
              finish(false);
              return false;
            }
            message.success("已开启命令执行");
            finish(true);
          } catch (err) {
            apiError.value = `开启命令执行失败：${errorText(err)}`;
            finish(false);
            return false;
          }
        },
        onNegativeClick: () => finish(false),
        onClose: () => finish(false),
      });
    });
  }

  async function save(): Promise<void> {
    if (submitting.value) return;
    apiError.value = "";
    if (props.task && props.task.running) {
      apiError.value = "任务正在运行，暂不能编辑。";
      return;
    }
    if (!validate()) return;
    if (form.type === "command" && !props.allowCommands) {
      const enabled = await confirmEnableCommands();
      if (!enabled) return;
    }

    const session = ++submitSeq;
    submitting.value = true;
    try {
      const input = buildInput();
      if (props.task) await api.taskUpdate(props.task.id, input);
      else await api.taskCreate(input);
      if (session !== submitSeq) return;
      message.success(props.task ? "任务已更新" : "任务已创建");
      emit("update:show", false);
      emit("saved");
    } catch (err) {
      if (session === submitSeq) apiError.value = errorText(err);
    } finally {
      if (session === submitSeq) submitting.value = false;
    }
  }

  function requestClose(): void {
    if (submitting.value) return;
    if (!isDirty()) {
      emit("update:show", false);
      return;
    }
    dialog.warning({
      title: "放弃未保存的修改？",
      content: "关闭后本次编辑的内容会丢失。",
      positiveText: "放弃修改",
      negativeText: "继续编辑",
      onPositiveClick: () => {
        emit("update:show", false);
      },
    });
  }

  function onShowChange(value: boolean): void {
    if (value) emit("update:show", true);
    else requestClose();
  }

  function close(): void {
    requestClose();
  }

  onBeforeUnmount(clearPreviewTimer);

  return {
    form,
    errors,
    apiError,
    submitting,
    repeat,
    preview,
    previewError,
    previewing,
    isEdit,
    summary,
    cronValidation,
    shellOptions,
    save,
    close,
    onShowChange,
  };
}
