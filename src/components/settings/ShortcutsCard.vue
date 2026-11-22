<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="key" :size="16" />快捷键</h3>
    </div>

    <SettingsRow
      label="全局快捷翻译"
      desc="默认关闭；触发时切到翻译页并载入剪贴板文本，只预览，不会自动翻译或上传"
    >
      <n-switch v-model:value="draft.quickTranslate.enabled" aria-label="启用全局快捷翻译" />
      <ShortcutInput
        v-model="draft.quickTranslate.shortcut"
        label="快捷翻译"
        disabled-hint="先启用全局快捷翻译"
        :disabled="!draft.quickTranslate.enabled"
      />
    </SettingsRow>

    <SettingsRow label="翻译页提交" desc="在原文输入框中按下该组合键，直接发起翻译">
      <ShortcutInput v-model="draft.shortcuts.translateSubmit" label="翻译页提交" />
    </SettingsRow>

    <SettingsRow label="SQL 编辑器执行" desc="在 SQL 编辑器内按下该组合键，执行当前语句">
      <ShortcutInput v-model="draft.shortcuts.dbRun" label="SQL 编辑器执行" />
    </SettingsRow>

    <p class="hint shortcut-hint">
      点击快捷键框后直接按下组合键（Esc 取消，退格清空）。全局快捷键若被其他程序占用，保存会失败并提示，原快捷键继续生效。
      输入框内的 Enter 换行、重命名时的 Enter / Esc 属于编辑行为，不在此配置。
    </p>
  </section>
</template>

<script setup lang="ts">
import { NSwitch } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
import ShortcutInput from "./ShortcutInput.vue";
import type { AppConfig } from "../../types";

defineProps<{ draft: AppConfig }>();
</script>

<style scoped>
.shortcut-hint {
  margin: 10px 0 0;
}
</style>
