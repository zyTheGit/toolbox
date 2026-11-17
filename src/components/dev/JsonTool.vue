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

      <div ref="editorRef" class="editor-wrap">
        <n-input
          v-model:value="input"
          class="json-input"
          type="textarea"
          :autosize="false"
          placeholder='{"name":"Toolbox","tags":["翻译","下载"],"version":1}'
          spellcheck="false"
        />
      </div>
    </div>

    <n-alert v-if="error" type="error" :bordered="false" class="result-block">
      <div class="error-head">
        <span>{{ error.title }}</span>
        <n-button v-if="error.snippet" size="tiny" text type="primary" @click="locateError">
          定位到出错位置
        </n-button>
      </div>
      <pre v-if="error.snippet" class="mono error-snippet">{{ error.snippet }}</pre>
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
import { jsonToTs } from "../../utils/json-to-ts";
import { parseJsonWithLocation } from "../../utils/json-parse";

const message = useMessage();

interface ErrorView {
  /** 已带上行列的错误描述 */
  title: string;
  /** 出错行与指示符 */
  snippet: string;
  /** 0 基偏移，用于选中出错字符 */
  position: number;
}

const input = ref("");
const result = ref("");
const resultLabel = ref("");
const error = ref<ErrorView | null>(null);
const indent = ref<number | string>(2);
const editorRef = ref<HTMLElement | null>(null);

/** 解析失败标记：区分「JSON 根值就是 null」与「解析失败」两种情况 */
const PARSE_FAILED = Symbol("parse-failed");

const indentOptions = [
  { label: "2 空格", value: 2 as number | string },
  { label: "4 空格", value: 4 as number | string },
  { label: "Tab", value: "\t" as number | string },
];

function parseInput(): unknown {
  error.value = null;
  result.value = "";
  resultLabel.value = "";
  const text = input.value;
  if (!text.trim()) {
    error.value = { title: "请先粘贴 JSON 内容。", snippet: "", position: 0 };
    return PARSE_FAILED;
  }
  const parsed = parseJsonWithLocation(text);
  if (!parsed.ok) {
    const { line, column, message: reason, snippet, position } = parsed.error;
    error.value = { title: `第 ${line} 行第 ${column} 列：${reason}`, snippet, position };
    return PARSE_FAILED;
  }
  return parsed.value;
}

/** 选中输入框中出错的字符，快速定位 */
function locateError(): void {
  const position = error.value?.position;
  const el = editorRef.value?.querySelector("textarea");
  if (position === undefined || !el) return;
  el.focus();
  el.setSelectionRange(position, Math.min(position + 1, el.value.length));
}

function formatJson(): void {
  const data = parseInput();
  if (data === PARSE_FAILED) return;
  result.value = JSON.stringify(data, null, indent.value);
  resultLabel.value = "格式化结果";
}

function minifyJson(): void {
  const data = parseInput();
  if (data === PARSE_FAILED) return;
  result.value = JSON.stringify(data);
  resultLabel.value = "压缩结果";
}

function genTs(): void {
  const data = parseInput();
  if (data === PARSE_FAILED) return;
  result.value = jsonToTs(data);
  resultLabel.value = "TypeScript 类型定义";
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
  error.value = null;
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

.editor-wrap {
  display: contents;
}

.error-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  line-height: 1.6;
}

.error-snippet {
  margin: 8px 0 0;
  padding: 8px 10px;
  overflow-x: auto;
  max-height: 160px;
  overflow-y: auto;
  font-size: 12.5px;
  line-height: 1.6;
  white-space: pre;
  background: var(--card, #fff);
  border: 1px solid var(--border);
  border-radius: 6px;
}
</style>
