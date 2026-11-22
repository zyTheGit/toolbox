<script setup lang="ts">
// 下载事件的前台提示：消息在窗口可见时显示；提示音仅在应用前台播放，
// 窗口隐藏时由系统通知负责发声，避免重复。
import { onBeforeUnmount, onMounted } from "vue";
import { useMessage } from "naive-ui";
import { events } from "../api";
import { store } from "../store";
import { playChime, warmUpChime } from "../utils/chime";

const message = useMessage();
let off: (() => void) | null = null;
let stopWarmUp: (() => void) | null = null;

onMounted(async () => {
  stopWarmUp = warmUpChime();
  try {
    off = await events.onDownloadNotify((toast) => {
      if (toast.kind === "failed") message.error(toast.text);
      else if (toast.kind === "all-done") message.info(toast.text);
      else message.success(toast.text);
      if (store.config?.notify.sound && !document.hidden) playChime();
    });
  } catch {
    // 事件桥断开时由顶部提示条兜底；通知失败不阻断正常使用。
  }
});

onBeforeUnmount(() => {
  off?.();
  stopWarmUp?.();
});
</script>

<template>
  <span hidden />
</template>
