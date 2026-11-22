<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="bell" :size="16" />通知</h3>
    </div>

    <SettingsRow
      label="下载事件通知"
      desc="总开关。窗口隐藏或最小化时发送系统通知，应用在前台时显示应用内提示"
    >
      <n-switch v-model:value="draft.notify.download" aria-label="下载事件通知总开关" />
    </SettingsRow>

    <SettingsRow label="任务完成" desc="单个任务下载完成时提醒一次">
      <n-switch
        v-model:value="draft.notify.events.success"
        :disabled="!draft.notify.download"
        aria-label="下载完成提醒"
      />
    </SettingsRow>

    <SettingsRow label="任务失败" desc="提醒并附带最关键的一行失败原因">
      <n-switch
        v-model:value="draft.notify.events.failed"
        :disabled="!draft.notify.download"
        aria-label="下载失败提醒"
      />
    </SettingsRow>

    <SettingsRow label="全部完成汇总" desc="队列全部结束时汇总本轮成功与失败的数量">
      <n-switch
        v-model:value="draft.notify.events.allDone"
        :disabled="!draft.notify.download"
        aria-label="下载队列全部完成提醒"
      />
    </SettingsRow>

    <SettingsRow label="提示音" desc="应用在前台时播放内置提示音；窗口隐藏时由系统通知发声">
      <n-switch
        v-model:value="draft.notify.sound"
        :disabled="!draft.notify.download"
        aria-label="下载事件提示音"
      />
    </SettingsRow>

    <SettingsRow label="终端任务结果通知" desc="仅对终端任务生效，提醒任务保持原有通知">
      <n-switch v-model:value="draft.notify.task" aria-label="终端任务完成与失败通知" />
    </SettingsRow>
  </section>
</template>

<script setup lang="ts">
import { NSwitch } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
import type { AppConfig } from "../../types";

defineProps<{ draft: AppConfig }>();
</script>
