<template>
  <div class="form-grid">
    <n-form-item
      label="任务名称"
      :validation-status="errors.name ? 'error' : undefined"
      :feedback="errors.name"
    >
      <n-input v-model:value="form.name" placeholder="例如：每日站会提醒" :maxlength="60" />
    </n-form-item>
    <n-form-item label="类型">
      <n-select :value="form.type" :options="typeOptions" @update:value="onTypeChange" />
    </n-form-item>
  </div>

  <n-form-item label="触发方式">
    <div class="seg" role="group" aria-label="触发方式">
      <button
        type="button"
        :class="{ on: form.schedule === 'cron' }"
        :disabled="disabled"
        @click="setSchedule('cron')"
      >
        重复执行
      </button>
      <button
        type="button"
        :class="{ on: form.schedule === 'once' }"
        :disabled="disabled"
        @click="setSchedule('once')"
      >
        仅一次
      </button>
    </div>
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
          :value="repeat.mode"
          :options="repeatModeOptions"
          @update:value="onRepeatModeChange"
        />

        <template v-if="repeat.mode === 'weekly'">
          <n-select
            class="repeat-weekdays"
            multiple
            max-tag-count="responsive"
            :value="repeat.weekdays"
            :options="weekdayOptions"
            placeholder="选择星期"
            @update:value="onWeekdaysChange"
          />
        </template>

        <template v-if="repeat.mode === 'monthly'">
          <n-input-number
            class="repeat-num"
            :value="repeat.day"
            :min="1"
            :max="31"
            :precision="0"
            @update:value="onDayChange"
          />
          <span class="repeat-label">号</span>
        </template>

        <template v-if="hasTime">
          <n-time-picker
            class="repeat-time"
            format="HH:mm"
            :formatted-value="repeat.time"
            @update:formatted-value="onTimeChange"
          />
        </template>

        <template v-if="repeat.mode === 'interval'">
          <span class="repeat-label">每</span>
          <n-input-number
            class="repeat-num"
            :value="repeat.interval"
            :min="1"
            :max="59"
            :precision="0"
            @update:value="onIntervalChange"
          />
          <span class="repeat-label">分钟</span>
        </template>

        <template v-if="repeat.mode === 'hourly'">
          <span class="repeat-label">第</span>
          <n-input-number
            class="repeat-num"
            :value="repeat.minute"
            :min="0"
            :max="59"
            :precision="0"
            @update:value="onMinuteChange"
          />
          <span class="repeat-label">分钟</span>
        </template>
      </div>

      <div v-if="repeat.mode === 'weekly' && !repeat.weekdays.length" class="hint error-hint">
        请至少选择一个星期。
      </div>

      <template v-if="repeat.mode === 'custom'">
        <div class="hint">
          格式：<span class="mono">分 时 日 月 周</span>，共 5 段、用空格分隔。例如
          <span class="mono">30 8 * * 1</span> 表示每周一 08:30。
        </div>
        <n-input
          v-model:value="form.cron"
          class="mono"
          placeholder="0 17 * * *"
          spellcheck="false"
        />
        <div class="cron-presets">
          <button
            v-for="item in CRON_PRESETS"
            :key="item.value"
            type="button"
            class="chip"
            :class="{ on: form.cron.trim() === item.value }"
            :disabled="disabled"
            @click="form.cron = item.value"
          >
            {{ item.label }}
            <span class="mono preset-expr">{{ item.value }}</span>
          </button>
        </div>
        <div class="hint">
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
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NDatePicker, NFormItem, NInput, NInputNumber, NSelect, NTimePicker } from "naive-ui";
import type { ScheduleKind } from "../../types";
import { CRON_PRESETS } from "../../utils/cron";
import { formatDateTime } from "./task-utils";
import { defaultRunAt, type RepeatMode, type RepeatState } from "./task-cron";
import type { TaskFormState } from "./useTaskForm";

const props = defineProps<{
  form: TaskFormState;
  errors: Record<string, string>;
  repeat: RepeatState;
  preview: string[];
  previewError: string;
  previewing: boolean;
  summary: string | null;
  cronValidation: string;
  disabled?: boolean;
}>();

const typeOptions = [
  { label: "终端", value: "command" },
  { label: "提醒", value: "reminder" },
];

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

const hasTime = computed(() => ["daily", "weekly", "monthly"].includes(props.repeat.mode));

function setSchedule(value: ScheduleKind): void {
  props.form.schedule = value;
  delete props.errors.cron;
  delete props.errors.runAt;
  if (value === "once" && !props.form.runAt) props.form.runAt = defaultRunAt();
  if (value === "cron" && !props.form.cron) props.form.cron = "0 17 * * *";
}

function onTypeChange(value: string | number | null): void {
  props.form.type = value === "reminder" ? "reminder" : "command";
}

function onRepeatModeChange(value: string | number | null): void {
  props.repeat.mode = (value as RepeatMode) ?? "daily";
  delete props.errors.cron;
}

function onWeekdaysChange(value: number[] | number | string | null): void {
  props.repeat.weekdays = Array.isArray(value) ? (value as number[]) : [];
}

function onDayChange(value: number | null): void {
  props.repeat.day = value ?? 1;
}

function onIntervalChange(value: number | null): void {
  props.repeat.interval = value ?? 5;
}

function onMinuteChange(value: number | null): void {
  props.repeat.minute = value ?? 0;
}

function onTimeChange(value: string | null): void {
  props.repeat.time = value || "17:00";
}

function onRunAtChange(value: string | null): void {
  props.form.runAt = value ?? "";
  delete props.errors.runAt;
}
</script>

<style scoped>
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
  /* 多选星期选中多个标签后控件会变高，顶部对齐避免与相邻输入框错位 */
  align-items: flex-start;
  gap: 10px;
  width: 100%;
}

.repeat-mode {
  width: 140px;
  flex: none;
}

.repeat-time {
  width: 130px;
  flex: none;
}

.repeat-weekdays {
  flex: 1;
  min-width: 200px;
}

.repeat-label {
  font-size: 13px;
  color: var(--text-2);
  line-height: var(--h-ctl);
}

.repeat-num {
  width: 90px;
}

.error-hint {
  color: var(--warning);
}

.cron-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  width: 100%;
}

.preset-expr {
  color: var(--faint);
}

.chip.on .preset-expr {
  color: inherit;
}

.cron-preview {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
  /* 预留「含义 + 接下来」两行高度，避免校验结果返回时弹窗高度跳动 */
  min-height: 45px;
}

.cron-meaning {
  font-size: 13px;
  color: var(--text);
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

.seg button:disabled,
.chip:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
</style>
