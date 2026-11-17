<template>
  <div class="json-tool">
    <div class="card">
      <div class="tool-row">
        <span class="hint">粘贴 JSON 后选择操作，全部本地处理。</span>
        <div class="actions">
          <n-select
            v-model:value="indent"
            class="indent-select"
            :options="indentOptions"
            size="small"
          />
          <n-button size="small" secondary @click="formatJson">格式化</n-button>
          <n-button size="small" secondary @click="minifyJson">压缩</n-button>
          <n-button size="small" secondary @click="genTs">TS 类型</n-button>
          <n-button size="small" quaternary @click="copyResult">复制结果</n-button>
          <n-button size="small" quaternary @click="clearAll">清空</n-button>
        </div>
      </div>

      <n-input
        v-model:value="input"
        class="json-input"
        type="textarea"
        :autosize="false"
        placeholder='{"name":"Toolbox","tags":["翻译","下载"],"version":1}'
        spellcheck="false"
      />
    </div>

    <n-alert v-if="error" type="error" :bordered="false" class="result-block">
      {{ error }}
    </n-alert>

    <div v-else-if="result" class="card result-card">
      <div class="tool-row">
        <span class="hint">{{ resultLabel }}</span>
        <n-button size="small" text type="primary" @click="copyResult">复制</n-button>
      </div>
      <pre class="mono result-body">{{ result }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NAlert, NButton, NInput, NSelect, useMessage } from "naive-ui";

const message = useMessage();

const input = ref("");
const result = ref("");
const resultLabel = ref("");
const error = ref("");
const indent = ref<number | string>(2);

const indentOptions = [
  { label: "2 空格", value: 2 as number | string },
  { label: "4 空格", value: 4 as number | string },
  { label: "Tab", value: "\t" as number | string },
];

function parseInput(): unknown | null {
  error.value = "";
  result.value = "";
  resultLabel.value = "";
  const text = input.value.trim();
  if (!text) {
    error.value = "请先粘贴 JSON 内容。";
    return null;
  }
  try {
    return JSON.parse(text) as unknown;
  } catch (err) {
    error.value = `JSON 解析失败：${(err as Error).message}`;
    return null;
  }
}

function formatJson(): void {
  const data = parseInput();
  if (data === null) return;
  result.value = JSON.stringify(data, null, indent.value);
  resultLabel.value = "格式化结果";
}

function minifyJson(): void {
  const data = parseInput();
  if (data === null) return;
  result.value = JSON.stringify(data);
  resultLabel.value = "压缩结果";
}

function genTs(): void {
  const data = parseInput();
  if (data === null) return;
  const interfaces = new Map<string, string>();
  const rootType = inferType(data, "Root", interfaces);
  const blocks = [...interfaces.values()];
  const output = blocks.length ? blocks.join("\n\n") : "";
  if (Array.isArray(data)) {
    result.value = output ? `${output}\n\ntype Root = ${rootType}` : `type Root = ${rootType}`;
  } else if (output) {
    result.value = `${output}\n\ntype Root = ${rootType}`;
  } else {
    result.value = `type Root = ${rootType}`;
  }
  resultLabel.value = "TypeScript 类型定义";
}

/** Pascal 化命名：root_user_addr → RootUserAddr */
function pascal(name: string): string {
  return name
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

/** 递归推断 JSON 值对应的 TS 类型，对象与数组元素收集为 interface。 */
function inferType(value: unknown, name: string, interfaces: Map<string, string>): string {
  if (value === null) return "null";
  if (Array.isArray(value)) {
    if (!value.length) return "unknown[]";
    const elemTypes = [...new Set(value.map((item, i) => inferType(item, `${name}${i}`, interfaces)))];
    return `${elemTypes.join(" | ")}[]`;
  }
  if (typeof value === "object") {
    const key = pascal(name);
    const lines = Object.entries(value as Record<string, unknown>).map(([prop, child]) => {
      const safeProp = /^[A-Za-z_$][\w$]*$/.test(prop) ? prop : JSON.stringify(prop);
      return `  ${safeProp}: ${inferType(child, `${name}_${prop}`, interfaces)};`;
    });
    const body = lines.length ? lines.join("\n") : "  [key: string]: unknown;";
    interfaces.set(key, `interface ${key} {\n${body}\n}`);
    return key;
  }
  return typeof value;
}

function copyResult(): void {
  const text = result.value || input.value;
  if (!text.trim()) {
    message.warning("没有可复制的内容");
    return;
  }
  void navigator.clipboard
    .writeText(text)
    .then(() => message.success("已复制"))
    .catch(() => message.error("复制失败"));
}

function clearAll(): void {
  input.value = "";
  result.value = "";
  error.value = "";
  resultLabel.value = "";
}
</script>

<style scoped>
.json-tool {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.card {
  background: var(--panel-soft);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tool-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.indent-select {
  width: 96px;
}

.json-input {
  flex: 1;
  min-height: 200px;
}

.json-input :deep(.n-input-wrapper),
.json-input :deep(.n-input__textarea),
.json-input :deep(textarea) {
  height: 100%;
  min-height: 200px;
}

.json-input :deep(textarea) {
  font-family: Consolas, "JetBrains Mono", monospace;
  resize: none;
}

.result-card .result-body {
  margin: 0;
  max-height: 320px;
  overflow: auto;
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-all;
}

.result-block {
  margin: 0;
}
</style>
