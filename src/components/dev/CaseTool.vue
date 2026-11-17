<template>
  <section class="card">
    <div class="tool-head">
      <span class="tool-title">大小写 / 命名转换</span>
      <n-button size="tiny" text type="primary" :disabled="!output" @click="copy">复制结果</n-button>
    </div>

    <n-input
      v-model:value="source"
      type="textarea"
      :autosize="{ minRows: 2, maxRows: 4 }"
      placeholder="输入文本，如 user profile info"
    />

    <div class="case-actions">
      <n-button
        v-for="item in CASE_ACTIONS"
        :key="item.key"
        size="tiny"
        secondary
        @click="convert(item.key)"
      >
        {{ item.label }}
      </n-button>
    </div>

    <div v-if="output" class="result-line mono">{{ output }}</div>
  </section>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NButton, NInput, useMessage } from "naive-ui";
import { CASE_ACTIONS, convertCase, type CaseKey } from "../../utils/text-case";

const message = useMessage();

const source = ref("");
const output = ref("");

function convert(key: CaseKey): void {
  if (!source.value.trim()) {
    message.warning("请先输入文本");
    output.value = "";
    return;
  }
  output.value = convertCase(source.value, key);
}

function copy(): void {
  if (!output.value) return;
  void navigator.clipboard
    .writeText(output.value)
    .then(() => message.success("已复制"))
    .catch(() => message.error("复制失败"));
}
</script>

<style scoped>
.card {
  background: var(--panel-soft);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tool-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.tool-title {
  font-size: 14px;
  font-weight: 600;
}

.case-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.result-line {
  padding: 8px 10px;
  font-size: 13px;
  line-height: 1.6;
  word-break: break-all;
  background: var(--card, #fff);
  border: 1px solid var(--border);
  border-radius: 6px;
}
</style>
