<template>
  <n-modal
    :show="show"
    preset="card"
    :title="isEdit ? '编辑任务' : '新建任务'"
    :style="{ width: '640px', maxWidth: 'calc(100vw - 48px)' }"
    :mask-closable="false"
    :bordered="false"
    @update:show="onShowChange"
  >
    <n-alert v-if="apiError" type="error" :bordered="false" class="modal-alert">
      {{ apiError }}
    </n-alert>

    <n-form label-placement="top" :show-feedback="true" :disabled="submitting">
      <div class="form-grid">
        <n-form-item
          label="名称"
          :validation-status="errors.name ? 'error' : undefined"
          :feedback="errors.name"
        >
          <n-input v-model:value="form.name" placeholder="例如：每日备份数据库" :maxlength="60" />
        </n-form-item>
        <n-form-item label="类型">
          <n-select
            :value="form.type"
            :options="typeOptions"
            @update:value="onTypeChange"
          />
        </n-form-item>
      </div>

      <n-form-item label="触发方式">
        <n-radio-group :value="form.schedule" @update:value="onScheduleChange">
          <n-radio-button value="cron">重复执行</n-radio-button>
          <n-radio-button value="once">仅一次</n-radio-button>
        </n-radio-group>
      </n-form-item>

      <n-form-item
        v-if="form.schedule === 'cron'"
        label="重复方式"
        :validation-status="cronValidation ? 'error' : undefined"
        :feedback="cronValidation"
      >
        <div class="repeat-block">
          <div class="repeat-row">
            <n-select
              class="repeat-mode"
              :value="repeatMode"
              :options="repeatModeOptions"
              @update:value="onRepeatModeChange"
            />

            <template v-if="repeatMode === 'weekly'">
              <n-select
                class="repeat-weekdays"
                multiple
                :value="repeatWeekdays"
                :options="weekdayOptions"
                placeholder="选择星期"
                @update:value="onWeekdaysChange"
              />
            </template>

            <template v-if="repeatMode === 'monthly'">
              <n-input-number
                class="repeat-num"
                :value="repeatDay"
                :min="1"
                :max="31"
                :precision="0"
                @update:value="onRepeatDayChange"
              />
              <span class="repeat-label">号</span>
            </template>

            <template v-if="hasRepeatTime">
              <n-time-picker
                class="repeat-time"
                format="HH:mm"
                :formatted-value="repeatTime"
                @update:formatted-value="onRepeatTimeChange"
              />
            </template>

            <template v-if="repeatMode === 'interval'">
              <span class="repeat-label">每</span>
              <n-input-number
                class="repeat-num"
                :value="repeatInterval"
                :min="1"
                :max="59"
                :precision="0"
                @update:value="onIntervalChange"
              />
              <span class="repeat-label">分钟</span>
            </template>

            <template v-if="repeatMode === 'hourly'">
              <span class="repeat-label">第</span>
              <n-input-number
                class="repeat-num"
                :value="repeatMinute"
                :min="0"
                :max="59"
                :precision="0"
                @update:value="onMinuteChange"
              />
              <span class="repeat-label">分钟</span>
            </template>
          </div>

          <div
            v-if="repeatMode === 'weekly' && !repeatWeekdays.length"
            class="hint error-hint"
          >
            请至少选择一个星期。
          </div>

          <template v-if="repeatMode === 'custom'">
            <div class="hint cron-format">
              格式：<span class="mono">分 时 日 月 周</span>，共 5 段、用空格分隔。例如
              <span class="mono">30 8 * * 1</span> 表示每周一 08:30。
            </div>
            <n-input
              v-model:value="form.cron"
              class="cron-input mono"
              placeholder="0 17 * * *"
              spellcheck="false"
            />
            <div class="cron-presets">
              <button
                v-for="item in CRON_PRESETS"
                :key="item.value"
                type="button"
                class="cron-chip"
                :class="{ active: form.cron.trim() === item.value }"
                @click="form.cron = item.value"
              >
                {{ item.label }}
                <span class="mono cron-chip-expr">{{ item.value }}</span>
              </button>
            </div>
            <div class="hint cron-legend">
              <span class="mono">*</span> 任意值 ·
              <span class="mono">*/5</span> 每 5 个单位 ·
              <span class="mono">1,3,5</span> 列举 ·
              <span class="mono">1-5</span> 范围 ·
              周：<span class="mono">0</span> 为周日
            </div>
          </template>

          <div class="cron-preview">
            <div v-if="previewing" class="hint">正在校验 cron 表达式…</div>
            <div v-else-if="previewError" class="hint error-hint">{{ previewError }}</div>
            <template v-else>
              <div class="cron-meaning">
                {{ summary ? `含义：${summary}` : "表达式有效，按 cron 规则重复" }}
              </div>
              <div v-if="preview.length" class="hint">
                接下来：
                <span class="mono">{{ preview.map((time) => formatDateTime(time).slice(5)).join(" · ") }}</span>
              </div>
            </template>
          </div>
        </div>
      </n-form-item>

      <n-form-item
        v-else
        label="执行时间（年-月-日 时:分:秒）"
        :validation-status="errors.runAt ? 'error' : undefined"
        :feedback="errors.runAt"
      >
        <div class="run-at-block">
          <n-date-picker
            type="datetime"
            format="yyyy-MM-dd HH:mm:ss"
            :formatted-value="form.runAt"
            class="run-at-input"
            clearable
            @update:formatted-value="onRunAtChange"
          />
          <div class="hint">到点执行一次；应用未运行则记为错过，不补跑。</div>
        </div>
      </n-form-item>

      <template v-if="form.type === 'reminder'">
        <n-form-item label="提醒内容（可选）">
          <n-input
            v-model:value="form.payload.message"
            type="textarea"
            :autosize="{ minRows: 3, maxRows: 6 }"
            placeholder="例如：续费服务器，避免到期停机"
          />
        </n-form-item>
        <div class="hint reminder-hint">
          到点弹出系统通知，标题使用任务名称；应用需保持运行。
        </div>
      </template>

      <template v-else>
        <div class="hint command-hint">
          以当前用户权限运行、不提权；命令与参数明文保存在本机。
        </div>

        <n-form-item
          label="命令"
          :validation-status="errors.command ? 'error' : undefined"
          :feedback="errors.command"
        >
          <n-input
            v-model:value="form.payload.command"
            class="mono"
            placeholder='例如：claude --prompt "打开微信"'
            spellcheck="false"
          />
        </n-form-item>

        <n-collapse class="advanced-collapse">
          <n-collapse-item title="高级选项（工作目录 / Shell / 超时 / 环境变量）" name="command-advanced">
            <div class="form-grid">
              <n-form-item label="工作目录">
                <n-input
                  v-model:value="form.payload.workDir"
                  placeholder="~（默认），支持 ~ 与环境变量"
                  spellcheck="false"
                />
              </n-form-item>
              <n-form-item label="Shell">
                <n-select
                  :value="form.payload.shell"
                  :options="shellOptions"
                  @update:value="onShellChange"
                />
              </n-form-item>
            </div>

            <n-form-item
              label="超时（秒）"
              :validation-status="errors.timeout ? 'error' : undefined"
              :feedback="errors.timeout"
            >
              <n-input-number
                :value="timeoutValue"
                :min="1"
                :max="86400"
                :precision="0"
                class="timeout-input"
                @update:value="onTimeoutChange"
              />
            </n-form-item>

            <n-form-item label="环境变量（可选）">
              <n-input
                v-model:value="form.payload.env"
                type="textarea"
                :autosize="{ minRows: 3, maxRows: 8 }"
                class="mono"
                placeholder="KEY=VALUE，一行一个"
                spellcheck="false"
              />
            </n-form-item>
          </n-collapse-item>
        </n-collapse>
      </template>

      <n-form-item label="启用任务">
        <n-switch v-model:value="form.enabled" />
      </n-form-item>
    </n-form>

    <template #footer>
      <div class="modal-foot">
        <n-button @click="close">取消</n-button>
        <n-button
          type="primary"
          :loading="submitting"
          @click="save"
        >
          保存
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCollapse,
  NCollapseItem,
  NDatePicker,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NModal,
  NRadioButton,
  NRadioGroup,
  NSelect,
  NSwitch,
  NTimePicker,
  useDialog,
  useMessage,
} from "naive-ui";
import { api } from "../../api";
import { store } from "../../store";
import type {
  AppConfig,
  ScheduleKind,
  Task,
  TaskInput,
  TaskPayload,
  TaskType,
} from "../../types";
import { emptyPayload, errorText, formatDateTime } from "./task-utils";
import { CRON_PRESETS, cronSummary, parseWeekdays } from "../../utils/cron";

const props = defineProps<{
  show: boolean;
  task: Task | null;
  allowCommands: boolean;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "saved"): void;
}>();

const message = useMessage();
const dialog = useDialog();

const form = ref<{
  name: string;
  type: TaskType;
  schedule: ScheduleKind;
  cron: string;
  runAt: string;
  enabled: boolean;
  payload: TaskPayload;
}>({
  name: "",
  type: "command",
  schedule: "cron",
  cron: "0 17 * * *",
  runAt: "",
  enabled: true,
  payload: emptyPayload(),
});

const errors = ref<Record<string, string>>({});
const apiError = ref("");
const submitting = ref(false);

const preview = ref<string[]>([]);
const previewError = ref("");
const previewing = ref(false);
let previewTimer: ReturnType<typeof setTimeout> | null = null;
let previewSeq = 0;
let submitSeq = 0;
let initialSnapshot = "";

type RepeatMode = "daily" | "weekly" | "monthly" | "interval" | "hourly" | "custom";

interface ParsedCron {
  mode: Exclude<RepeatMode, "custom">;
  hour?: number;
  minute?: number;
  weekdays?: number[];
  day?: number;
  interval?: number;
}

const repeatMode = ref<RepeatMode>("daily");
const repeatTime = ref("17:00");
const repeatWeekdays = ref<number[]>([1]);
const repeatDay = ref(1);
const repeatInterval = ref(5);
const repeatMinute = ref(0);

const repeatModeOptions: Array<{ label: string; value: RepeatMode }> = [
  { label: "每天", value: "daily" },
  { label: "每周", value: "weekly" },
  { label: "每月", value: "monthly" },
  { label: "间隔", value: "interval" },
  { label: "每小时", value: "hourly" },
  { label: "自定义 cron", value: "custom" },
];

const weekdayOptions = [
  { label: "周一", value: 1 },
  { label: "周二", value: 2 },
  { label: "周三", value: 3 },
  { label: "周四", value: 4 },
  { label: "周五", value: 5 },
  { label: "周六", value: 6 },
  { label: "周日", value: 0 },
];

const hasRepeatTime = computed(() =>
  ["daily", "weekly", "monthly"].includes(repeatMode.value),
);

const isEdit = computed(() => Boolean(props.task));
const isWindows = computed(() => !store.platform || /win/i.test(store.platform));
const summary = computed(() => cronSummary(form.value.cron));
const cronValidation = computed(() =>
  form.value.schedule === "cron" && repeatMode.value === "custom"
    ? errors.value.cron || previewError.value
    : "",
);

const typeOptions = computed(() => [
  { label: "终端", value: "command" },
  { label: "提醒", value: "reminder" },
]);

const shellOptions = computed(() =>
  isWindows.value
    ? [
        { label: "cmd", value: "cmd" },
        { label: "PowerShell", value: "powershell" },
      ]
    : [{ label: "sh", value: "sh" }],
);

const timeoutValue = computed<number | null>(() =>
  form.value.payload.timeoutSeconds > 0 ? form.value.payload.timeoutSeconds : null,
);

watch(
  () => props.show,
  (visible) => {
    if (visible) init();
    else clearPreviewTimer();
  },
);

watch(
  () => form.value.type,
  () => {
    errors.value = {};
    ensureShell();
  },
);

watch(
  () => form.value.cron,
  () => {
    errors.value = { ...errors.value, cron: "" };
    schedulePreview();
  },
);

watch(
  [repeatMode, repeatTime, repeatWeekdays, repeatDay, repeatInterval, repeatMinute],
  () => {
    if (repeatMode.value === "custom") return;
    form.value.cron = composeCron();
  },
  { deep: true },
);

function init(): void {
  submitSeq += 1;
  const task = props.task;
  form.value.name = task?.name ?? "";
  form.value.type = task?.type ?? "command";
  form.value.schedule = task?.schedule ?? "cron";
  form.value.cron = task?.cron || "0 17 * * *";
  form.value.runAt = task?.runAt || defaultRunAt();
  form.value.enabled = task?.enabled ?? true;
  form.value.payload = { ...emptyPayload(), ...(task?.payload ?? {}) };
  ensureShell();
  applyCron(form.value.cron);
  errors.value = {};
  apiError.value = "";
  submitting.value = false;
  schedulePreview();
  // Capture after watchers normalize the repeat-mode fields.
  void nextTick(() => {
    initialSnapshot = snapshotForm();
  });
}

function snapshotForm(): string {
  return JSON.stringify(form.value);
}

function isDirty(): boolean {
  return snapshotForm() !== initialSnapshot;
}

function defaultRunAt(): string {
  return formatDateTime(new Date(Date.now() + 60 * 60 * 1000).toISOString(), true);
}

function ensureShell(): void {
  if (form.value.type !== "command") return;
  if (!shellOptions.value.some((item) => item.value === form.value.payload.shell)) {
    form.value.payload.shell = shellOptions.value[0]?.value ?? "sh";
  }
}

function onTypeChange(value: string | number | null): void {
  form.value.type = value === "reminder" ? "reminder" : "command";
}

function onScheduleChange(value: string | number | null): void {
  form.value.schedule = value === "once" ? "once" : "cron";
  errors.value = { ...errors.value, cron: "", runAt: "" };
  if (form.value.schedule === "once" && !form.value.runAt) {
    form.value.runAt = defaultRunAt();
  }
  if (form.value.schedule === "cron" && !form.value.cron) {
    form.value.cron = "0 17 * * *";
  }
  schedulePreview();
}

function onRunAtChange(value: string | null): void {
  form.value.runAt = value ?? "";
  errors.value = { ...errors.value, runAt: "" };
}

function pad2(value: number): string {
  return String(value).padStart(2, "0");
}

function timeParts(value: string): [number, number] {
  const match = /^(\d{1,2}):(\d{1,2})$/.exec(value.trim());
  if (!match) return [17, 0];
  return [Math.min(23, Number(match[1])), Math.min(59, Number(match[2]))];
}

function composeCron(): string {
  if (repeatMode.value === "custom") return form.value.cron.trim();
  const [hour, minute] = timeParts(repeatTime.value);
  switch (repeatMode.value) {
    case "daily":
      return `${minute} ${hour} * * *`;
    case "weekly": {
      if (!repeatWeekdays.value.length) return "";
      const days = [...repeatWeekdays.value].sort((a, b) => a - b).join(",");
      return `${minute} ${hour} * * ${days}`;
    }
    case "monthly":
      return `${minute} ${hour} ${repeatDay.value} * *`;
    case "interval":
      return `*/${repeatInterval.value} * * * *`;
    case "hourly":
      return `${repeatMinute.value} * * * *`;
    default:
      return "";
  }
}

function parseCron(expr: string): ParsedCron | null {
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

function applyCron(expr: string): void {
  const parsed = parseCron(expr);
  if (!parsed) {
    repeatMode.value = "custom";
    return;
  }
  repeatMode.value = parsed.mode;
  if (parsed.hour != null && parsed.minute != null) {
    repeatTime.value = `${pad2(parsed.hour)}:${pad2(parsed.minute)}`;
  }
  if (parsed.mode === "weekly") repeatWeekdays.value = parsed.weekdays ?? [1];
  if (parsed.mode === "monthly") repeatDay.value = parsed.day ?? 1;
  if (parsed.mode === "interval") repeatInterval.value = parsed.interval ?? 5;
  if (parsed.mode === "hourly") repeatMinute.value = parsed.minute ?? 0;
}

function onRepeatModeChange(value: string | number | null): void {
  repeatMode.value = (value as RepeatMode) ?? "daily";
  errors.value = { ...errors.value, cron: "" };
}

function onRepeatTimeChange(value: string | null): void {
  repeatTime.value = value || "17:00";
}

function onWeekdaysChange(value: number[] | number | string | null): void {
  repeatWeekdays.value = Array.isArray(value) ? (value as number[]) : [];
}

function onRepeatDayChange(value: number | null): void {
  repeatDay.value = value ?? 1;
}

function onIntervalChange(value: number | null): void {
  repeatInterval.value = value ?? 5;
}

function onMinuteChange(value: number | null): void {
  repeatMinute.value = value ?? 0;
}

function onShellChange(value: string | number | null): void {
  form.value.payload.shell = typeof value === "string" ? value : shellOptions.value[0].value;
}

function onTimeoutChange(value: number | null): void {
  form.value.payload.timeoutSeconds = value ?? 0;
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
  if (form.value.schedule !== "cron") {
    preview.value = [];
    previewError.value = "";
    return;
  }
  const expr = form.value.cron.trim();
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
  if (!form.value.name.trim()) next.name = "请填写任务名称";
  if (form.value.schedule === "cron") {
    if (repeatMode.value === "weekly" && !repeatWeekdays.value.length) {
      next.cron = "请至少选择一个星期";
    } else if (!form.value.cron.trim()) {
      next.cron = "请选择重复方式";
    } else if (previewError.value) {
      next.cron = previewError.value;
    }
  } else {
    const runAt = form.value.runAt.trim();
    if (!runAt) {
      next.runAt = "请选择执行时间";
    } else if (!/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(runAt)) {
      next.runAt = "时间格式应为 YYYY-MM-DD HH:MM:SS";
    } else {
      const when = new Date(runAt.replace(" ", "T"));
      if (Number.isNaN(when.getTime())) next.runAt = "执行时间无效";
      else if (form.value.enabled && when.getTime() <= Date.now()) {
        next.runAt = "执行时间必须晚于当前时间";
      }
    }
  }

  if (form.value.type === "command") {
    if (!form.value.payload.command.trim()) next.command = "请填写要执行的命令";
    const timeout = Number(form.value.payload.timeoutSeconds);
    if (!Number.isFinite(timeout) || timeout < 1 || timeout > 86400) {
      next.timeout = "超时需在 1 - 86400 秒之间";
    }
  }

  errors.value = next;
  return Object.keys(next).length === 0;
}

function buildInput(): TaskInput {
  const p = form.value.payload;
  return {
    name: form.value.name.trim(),
    type: form.value.type,
    cron: form.value.schedule === "cron" ? form.value.cron.trim() : "",
    schedule: form.value.schedule,
    runAt: form.value.schedule === "once" ? form.value.runAt.trim() : "",
    enabled: form.value.enabled,
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
          const config = JSON.parse(JSON.stringify(store.config)) as AppConfig;
          config.security.allowCommands = true;
          const result = await api.saveConfig(config);
          store.config = result.config;
          store.tools = result.tools;
          store.providers = result.providers;
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
  if (form.value.type === "command" && !props.allowCommands) {
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
</script>

<style scoped>
.modal-alert {
  margin-bottom: 14px;
}

.repeat-block {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
  min-width: 0;
}

.repeat-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.repeat-mode {
  width: 140px;
  flex: none;
}

.repeat-time {
  width: 120px;
  flex: none;
}

.repeat-weekdays {
  flex: 1;
  min-width: 200px;
}

.repeat-label {
  font-size: 13px;
  color: #4b5468;
}

.repeat-num {
  width: 90px;
}

.error-hint {
  color: var(--warning);
}

.cron-input {
  width: 100%;
}

.cron-format,
.cron-legend {
  width: 100%;
}

.cron-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  width: 100%;
}

.cron-chip {
  display: inline-flex;
  align-items: baseline;
  gap: 6px;
  padding: 4px 10px;
  font-size: 12px;
  color: var(--text);
  background: var(--panel-soft);
  border: 1px solid var(--border);
  border-radius: 999px;
  cursor: pointer;
}

.cron-chip:hover {
  color: var(--primary);
  border-color: var(--primary);
}

.cron-chip.active {
  color: var(--primary);
  border-color: var(--primary);
  background: rgba(79, 99, 231, 0.08);
}

.cron-chip-expr {
  color: var(--muted);
  font-size: 11px;
}

.cron-chip.active .cron-chip-expr {
  color: inherit;
}

.cron-preview {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.cron-meaning {
  font-size: 13px;
  color: var(--text);
}

.command-hint {
  margin-bottom: 12px;
}

.advanced-collapse {
  margin: 4px 0 12px;
}

.run-at-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}

.run-at-input {
  width: 240px;
}

.reminder-hint {
  margin-top: -4px;
  color: var(--muted);
}

.timeout-input {
  width: 180px;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>