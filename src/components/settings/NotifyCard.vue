<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="bell" :size="16" />通知</h3>
    </div>

    <SettingsRow label="下载完成通知" desc="仅在主窗口隐藏或最小化时提醒，点击通知回到下载页">
      <n-switch v-model:value="draft.notify.download" aria-label="下载完成与失败通知" />
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
