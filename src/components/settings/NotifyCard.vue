<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="bell" :size="16" />通知与快捷翻译</h3>
    </div>

    <SettingsRow label="下载完成通知" desc="仅在主窗口隐藏或最小化时提醒，点击通知回到下载页">
      <n-switch v-model:value="draft.notify.download" aria-label="下载完成与失败通知" />
    </SettingsRow>

    <SettingsRow label="终端任务结果通知" desc="仅对终端任务生效，提醒任务保持原有通知">
      <n-switch v-model:value="draft.notify.task" aria-label="终端任务完成与失败通知" />
    </SettingsRow>

    <SettingsRow
      label="全局快捷翻译"
      desc="默认关闭；触发时只把剪贴板文本载入翻译页供预览，不会自动翻译或上传"
    >
      <n-switch v-model:value="draft.quickTranslate.enabled" aria-label="启用全局快捷翻译" />
      <n-input
        v-model:value="draft.quickTranslate.shortcut"
        class="w-shortcut mono"
        aria-label="快捷翻译快捷键"
        :disabled="!draft.quickTranslate.enabled"
        placeholder="CommandOrControl+Shift+T"
      />
    </SettingsRow>

    <p class="hint shortcut-hint">快捷键被其他程序占用时，保存会失败并提示，修改后可重试。</p>
  </section>
</template>

<script setup lang="ts">
import { NInput, NSwitch } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
import type { AppConfig } from "../../types";

defineProps<{ draft: AppConfig }>();
</script>

<style scoped>
.w-shortcut {
  width: 150px;
}

.shortcut-hint {
  margin: 10px 0 0;
}

/* Naive 输入框内部才是真正的 input，等宽字体要下探一层才生效 */
.mono :deep(.n-input__input-el) {
  font-family: var(--font-mono);
  font-size: var(--fs-12);
}

@media (max-width: 760px) {
  .w-shortcut {
    width: 100%;
  }
}
</style>
