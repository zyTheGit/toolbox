<template>
  <span class="pill" :class="meta.cls">{{ label ?? meta.text }}</span>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ status: string; label?: string }>();

const MAP: Record<string, { text: string; cls: string }> = {
  "": { text: "未运行", cls: "pill-idle" },
  queued: { text: "排队中", cls: "pill-idle" },
  running: { text: "运行中", cls: "pill-run" },
  completed: { text: "已完成", cls: "pill-ok" },
  success: { text: "成功", cls: "pill-ok" },
  failed: { text: "失败", cls: "pill-err" },
  canceled: { text: "已取消", cls: "pill-idle" },
  interrupted: { text: "已中断", cls: "pill-err" },
  skipped: { text: "已跳过", cls: "pill-warn" },
};

const meta = computed(() => MAP[props.status] ?? { text: props.status, cls: "pill-idle" });
</script>
