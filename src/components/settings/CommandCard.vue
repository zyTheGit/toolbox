<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="terminal" :size="16" />命令执行</h3>
    </div>

    <SettingsRow
      label="允许运行终端任务"
      desc="默认关闭；开启后定时任务可执行本机命令，以当前用户权限运行、不提权、不监听端口，命令明文保存在本机"
    >
      <n-switch v-model:value="draft.security.allowCommands" aria-label="允许运行终端任务" />
    </SettingsRow>

    <div v-if="!draft.security.allowCommands" class="alert alert-warn cmd-alert">
      <AppIcon name="alert" :size="16" />
      <span>命令执行已关闭，终端类定时任务将不会触发，也无法立即运行。开启前请确认任务内容可信。</span>
    </div>
  </section>
</template>

<script setup lang="ts">
import { NSwitch } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
import type { AppConfig } from "../../types";

defineProps<{ draft: AppConfig }>();
</script>

<style scoped>
.cmd-alert {
  margin-top: 12px;
}
</style>
