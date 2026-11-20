<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="save" :size="16" />备份与恢复</h3>
    </div>

    <div class="row backup-actions">
      <n-button :loading="exporting" :disabled="exporting || importing" @click="exportData">
        <template #icon><AppIcon name="download" :size="16" /></template>
        导出备份
      </n-button>
      <n-button :loading="importing" :disabled="importing || exporting" @click="importData">
        <template #icon><AppIcon name="upload" :size="16" /></template>
        从备份恢复
      </n-button>
      <n-checkbox v-model:checked="includeCommands">
        包含命令与输出（可能含敏感信息）
      </n-checkbox>
    </div>

    <div v-if="pendingRestart" class="alert alert-warn restart-alert">
      <AppIcon name="alert" :size="16" />
      <div class="restart-body">
        <span>备份已恢复，请重启应用使恢复结果生效，并到「定时任务」核对命令任务。</span>
        <n-button
          size="tiny"
          type="warning"
          :loading="restarting"
          :disabled="restarting"
          @click="restart"
        >
          重启应用
        </n-button>
      </div>
    </div>

    <p class="hint backup-hint">
      备份包含配置、翻译历史、任务与下载记录；API 密钥、Cookie、代理与请求头会被剔除。
      默认不导出命令文本、环境变量与运行输出。恢复时外部命令执行会被强制关闭，
      命令任务将被禁用并标记“待确认”，需要你核对后手动启用。
    </p>
  </section>
</template>

<script setup lang="ts">
import { h, ref } from "vue";
import { NButton, NCheckbox, useDialog, useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { api } from "../../api";
import type { ImportOutcome } from "../../types";
import { describeError } from "../../utils/errors";

const message = useMessage();
const dialog = useDialog();

const exporting = ref(false);
const importing = ref(false);
const restarting = ref(false);
const includeCommands = ref(false);
const pendingRestart = ref(false);

async function exportData(): Promise<void> {
  if (exporting.value) return;
  if (includeCommands.value) {
    const confirmed = await confirmIncludeCommands();
    if (!confirmed) return;
  }
  await runExport();
}

function confirmIncludeCommands(): Promise<boolean> {
  return new Promise((resolve) => {
    dialog.warning({
      title: "导出包含命令与输出？",
      content:
        "备份将包含命令文本与运行输出，可能带有路径、地址或凭据。请只在可信环境保存和传输；API 密钥、Cookie、代理与请求头仍会被剔除。",
      positiveText: "仍要包含",
      negativeText: "取消",
      onPositiveClick: () => resolve(true),
      onNegativeClick: () => resolve(false),
      onClose: () => resolve(false),
      onMaskClick: () => resolve(false),
    });
  });
}

async function runExport(): Promise<void> {
  exporting.value = true;
  try {
    const path = await api.dataExport(includeCommands.value);
    if (!path) return;
    message.success(`备份已导出：${path}`);
  } catch (err) {
    message.error(`导出失败：${describeError(err)}`);
  } finally {
    exporting.value = false;
  }
}

function importData(): void {
  if (importing.value) return;
  dialog.warning({
    title: "从备份恢复数据？",
    content:
      "将覆盖当前配置、翻译历史、任务和下载记录。恢复前会把现有数据复制到数据目录下的 restore-backup-<时间> 文件夹；恢复后外部命令执行会被关闭，需要重启应用生效。",
    positiveText: "选择备份并恢复",
    negativeText: "取消",
    onPositiveClick: () => {
      void runImport();
    },
  });
}

async function runImport(): Promise<void> {
  importing.value = true;
  try {
    const outcome = await api.dataImport();
    if (!outcome.backupPath) return;
    pendingRestart.value = true;
    showOutcome(outcome);
  } catch (err) {
    message.error(`恢复失败：${describeError(err)}`);
  } finally {
    importing.value = false;
  }
}

function showOutcome(outcome: ImportOutcome): void {
  dialog.success({
    title: "备份已恢复",
    content: () =>
      h("div", { style: "line-height: 1.8" }, [
        h(
          "div",
          `已恢复配置、${outcome.restoredHistory} 条翻译历史、${outcome.restoredTasks} 个任务、${outcome.restoredJobs} 条下载记录。`,
        ),
        h("div", "外部命令执行已关闭，应用在重启前保持只读。"),
        outcome.disabledCommandTasks > 0
          ? h(
              "div",
              `有 ${outcome.disabledCommandTasks} 个命令任务已禁用，待你核对命令内容后手动启用。`,
            )
          : null,
        h("div", `恢复前数据快照：${outcome.backupPath}`),
      ]),
    positiveText: "重启应用",
    negativeText: "稍后重启",
    onPositiveClick: () => {
      void restart();
    },
  });
}

async function restart(): Promise<void> {
  if (restarting.value) return;
  restarting.value = true;
  try {
    await api.restartApp();
  } catch (err) {
    message.error(`重启失败：${describeError(err)}，请手动退出后重新打开应用。`);
  } finally {
    restarting.value = false;
  }
}
</script>

<style scoped>
.backup-actions {
  flex-wrap: wrap;
  gap: 10px;
}

.restart-alert {
  margin-top: 12px;
}

.restart-body {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex: 1;
  min-width: 0;
}

.backup-hint {
  margin: 10px 0 0;
}

@media (max-width: 760px) {
  .restart-body {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
}
</style>
