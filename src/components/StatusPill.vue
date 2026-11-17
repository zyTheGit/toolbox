<template>
  <n-tag size="small" :type="meta.type" :bordered="false">{{ label ?? meta.label }}</n-tag>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NTag } from "naive-ui";

const props = defineProps<{ status: string; label?: string }>();

type TagType = "default" | "error" | "info" | "success" | "warning";

const MAP: Record<string, { label: string; type: TagType }> = {
  "": { label: "未运行", type: "default" },
  queued: { label: "排队中", type: "info" },
  running: { label: "运行中", type: "info" },
  completed: { label: "已完成", type: "success" },
  success: { label: "成功", type: "success" },
  failed: { label: "失败", type: "error" },
  canceled: { label: "已取消", type: "default" },
  interrupted: { label: "已中断", type: "error" },
  skipped: { label: "已跳过", type: "warning" },
};

const meta = computed(() => MAP[props.status] ?? { label: props.status, type: "default" as TagType });
</script>
