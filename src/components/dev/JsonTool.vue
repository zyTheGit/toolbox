<template>
  <div class="dev-grid">
    <!-- 输入 -->
    <section class="card">
      <div class="card-head">
        <h3 class="card-title"><AppIcon name="braces" :size="16" />输入</h3>
        <div class="row">
          <select v-model="indent" class="select indent-select" aria-label="缩进方式">
            <option :value="2">缩进 2</option>
            <option :value="4">缩进 4</option>
            <option value="tab">缩进 Tab</option>
          </select>
          <button class="btn btn-ghost btn-sm" @click="clearAll">清空</button>
        </div>
      </div>

      <textarea
        ref="inputRef"
        v-model="input"
        class="textarea mono json-input"
        spellcheck="false"
        placeholder='{"name":"Toolbox","tags":["翻译","下载"]}'
      ></textarea>

      <div class="row json-actions">
        <button class="btn btn-primary btn-sm" @click="formatJson">格式化</button>
        <button class="btn btn-outline btn-sm" @click="minifyJson">压缩</button>
        <button class="btn btn-outline btn-sm" @click="genTs">生成 TS 类型</button>
      </div>

      <div v-if="error" class="alert alert-err json-alert" role="alert">
        <AppIcon name="alert" :size="16" />
        <div class="grow">
          <div class="error-head">
            <span>{{ error.title }}</span>
            <button
              v-if="error.snippet"
              class="btn btn-ghost btn-sm error-locate"
              @click="locateError"
            >
              定位到出错位置
            </button>
          </div>
          <pre v-if="error.snippet" class="error-snippet">{{ error.snippet }}</pre>
        </div>
      </div>
    </section>

    <!-- 结果 -->
    <section class="card">
      <div class="card-head">
        <h3 class="card-title"><AppIcon name="file" :size="16" />结果</h3>
        <div class="row">
          <span v-if="resultLabel" class="hint">{{ resultLabel }}</span>
          <button
            class="btn btn-outline btn-sm"
            :class="{ loading: copying }"
            :disabled="copying"
            @click="copyResult"
          >
            <AppIcon name="copy" :size="16" />复制结果
          </button>
        </div>
      </div>
      <pre class="term json-out">{{ display }}</pre>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { jsonToTs } from "../../utils/json-to-ts";
import { parseJsonWithLocation } from "../../utils/json-parse";
import { copyText } from "../../utils/clipboard";

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
/** 无结果时终端区的占位文案 */
const placeholder = ref("// 点击「格式化」查看结果");
const error = ref<ErrorView | null>(null);
const indent = ref<number | string>(2);
const copying = ref(false);
const inputRef = ref<HTMLTextAreaElement | null>(null);

/** 解析失败标记：区分「JSON 根值就是 null」与「解析失败」两种情况 */
const PARSE_FAILED = Symbol("parse-failed");

/** 输入框里没有结果时展示占位文案 */
const display = computed(() => result.value || placeholder.value);

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
  const el = inputRef.value;
  if (position === undefined || !el) return;
  el.focus();
  el.setSelectionRange(position, Math.min(position + 1, el.value.length));
}

function formatJson(): void {
  const data = parseInput();
  if (data === PARSE_FAILED) return;
  // 「缩进 Tab」用制表符，其余选项是空格数
  const space = indent.value === "tab" ? "\t" : indent.value;
  result.value = JSON.stringify(data, null, space);
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

async function copyResult(): Promise<void> {
  const text = result.value || input.value;
  if (!text.trim()) {
    message.warning("没有可复制的内容");
    return;
  }
  copying.value = true;
  try {
    await copyText(text);
    message.success("已复制");
  } catch {
    message.error("复制失败");
  } finally {
    copying.value = false;
  }
}

function clearAll(): void {
  input.value = "";
  result.value = "";
  error.value = null;
  resultLabel.value = "";
  placeholder.value = "// 已清空";
  inputRef.value?.focus();
}
</script>

<style scoped>
.dev-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.indent-select {
  width: 100px;
  height: var(--h-ctl-sm);
  padding: 0 26px 0 10px;
  font-size: var(--fs-13);
}

.json-input {
  min-height: 260px;
  font-family: var(--font-mono);
}

.json-actions,
.json-alert {
  margin-top: 12px;
}

/* 固定提示条内的按钮高度，避免撑高错误块 */
.error-locate {
  flex: none;
  height: 24px;
  padding: 0 8px;
}

.error-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  line-height: 1.6;
}

.error-snippet {
  margin: 8px 0 0;
  padding: 8px 10px;
  max-height: 160px;
  overflow: auto;
  font-family: var(--font-mono);
  font-size: var(--fs-12);
  line-height: 1.6;
  white-space: pre;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
}

.json-out {
  margin: 0;
  min-height: 120px;
  max-height: 320px;
  overflow: auto;
}

@media (max-width: 900px) {
  .dev-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
