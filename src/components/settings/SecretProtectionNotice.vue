<template>
  <p class="hint">
    <n-tag v-if="label" size="tiny" :bordered="false" :type="tagType">{{ label }}</n-tag>
    {{ detail }}
  </p>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NTag } from "naive-ui";
import { store } from "../../store";
import type { SecretProtection } from "../../types";

// 如实展示平台保护级别：macOS / Linux 目前只靠文件权限，内容等同明文。
const protection = computed<SecretProtection | "">(() => store.secrets.protection ?? "");

const label = computed(() => {
  switch (protection.value) {
    case "dpapi":
      return "Windows DPAPI 加密";
    case "file-permissions":
      return "仅文件权限保护";
    case "unknown":
      return "保护级别未知";
    default:
      return "";
  }
});

const tagType = computed<"success" | "warning">(() =>
  protection.value === "dpapi" ? "success" : "warning",
);

const detail = computed(() => {
  switch (protection.value) {
    case "dpapi":
      return "由 Windows DPAPI 绑定当前用户；Key 不写入 config.json，留空表示保持原值。";
    case "file-permissions":
      return "macOS / Linux 当前仅靠文件权限（数据目录 0700、secrets.bin 0600），内容等同明文，请勿共享或同步数据目录；Key 不写入 config.json。";
    case "unknown":
      return "无法确认当前平台保护级别，请勿共享数据目录；Key 不写入 config.json。";
    default:
      return "Key 保存在本机密钥文件，不写入 config.json；留空表示保持原值。";
  }
});
</script>
