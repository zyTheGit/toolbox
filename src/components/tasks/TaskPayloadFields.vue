<template>
  <template v-if="form.type === 'reminder'">
    <n-form-item label="提醒内容（可选）">
      <n-input
        v-model:value="form.payload.message"
        type="textarea"
        :autosize="{ minRows: 3, maxRows: 6 }"
        placeholder="到点后通过系统通知提醒的内容，例如：准备每日站会"
      />
    </n-form-item>
    <div class="hint reminder-hint">到点弹出系统通知，标题使用任务名称；应用需保持运行。</div>
  </template>

  <template v-else>
    <div class="hint command-hint">以当前用户权限运行、不提权；命令与参数明文保存在本机。</div>

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
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NCollapse, NCollapseItem, NFormItem, NInput, NInputNumber, NSelect } from "naive-ui";
import type { TaskFormState } from "./useTaskForm";

const props = defineProps<{
  form: TaskFormState;
  errors: Record<string, string>;
  shellOptions: Array<{ label: string; value: string }>;
}>();

const timeoutValue = computed<number | null>(() =>
  props.form.payload.timeoutSeconds > 0 ? props.form.payload.timeoutSeconds : null,
);

function onShellChange(value: string | number | null): void {
  props.form.payload.shell =
    typeof value === "string" ? value : props.shellOptions[0]?.value ?? "sh";
}

function onTimeoutChange(value: number | null): void {
  props.form.payload.timeoutSeconds = value ?? 0;
}
</script>

<style scoped>
.command-hint {
  margin-bottom: 12px;
}

.advanced-collapse {
  margin: 4px 0 12px;
}

.reminder-hint {
  margin-top: -4px;
  color: var(--muted);
}

.timeout-input {
  width: 180px;
}
</style>
