<template>
  <div class="coder-grid">
    <!-- 时间戳 -->
    <section class="card">
      <div class="tool-head">
        <span class="tool-title">时间戳</span>
        <span class="hint">当前：{{ nowStamp }} · {{ nowText }}</span>
      </div>
      <n-input
        v-model:value="stampInput"
        size="small"
        placeholder="时间戳（秒/毫秒自动识别）或 YYYY-MM-DD HH:mm:ss"
        @update:value="convertStamp"
      />
      <div v-if="stampResult" class="mono result-line">{{ stampResult }}</div>
    </section>

    <!-- UUID -->
    <section class="card">
      <div class="tool-head">
        <span class="tool-title">UUID</span>
        <n-button size="tiny" text type="primary" @click="genUuids">生成 5 个</n-button>
      </div>
      <div v-if="uuids.length" class="uuid-list">
        <div v-for="item in uuids" :key="item" class="uuid-row" @click="copy(item)">
          <span class="mono">{{ item }}</span>
        </div>
      </div>
      <div v-else class="hint">点击「生成 5 个」，单击条目复制。</div>
    </section>

    <!-- Base64 -->
    <section class="card">
      <div class="tool-head">
        <span class="tool-title">Base64</span>
        <div class="actions">
          <n-button size="tiny" text type="primary" @click="runBase64('encode')">编码</n-button>
          <n-button size="tiny" text type="primary" @click="runBase64('decode')">解码</n-button>
        </div>
      </div>
      <n-input
        v-model:value="b64Input"
        type="textarea"
        :autosize="{ minRows: 2, maxRows: 5 }"
        placeholder="输入文本或 Base64，支持中文"
      />
      <div v-if="b64Result" class="mono result-line selectable">{{ b64Result }}</div>
    </section>

    <!-- URL 编码 -->
    <section class="card">
      <div class="tool-head">
        <span class="tool-title">URL 编码</span>
        <div class="actions">
          <n-button size="tiny" text type="primary" @click="runUrl('encode')">编码</n-button>
          <n-button size="tiny" text type="primary" @click="runUrl('decode')">解码</n-button>
        </div>
      </div>
      <n-input
        v-model:value="urlInput"
        type="textarea"
        :autosize="{ minRows: 2, maxRows: 5 }"
        placeholder="输入文本或已编码的 URL 片段"
      />
      <div v-if="urlResult" class="mono result-line selectable">{{ urlResult }}</div>
    </section>

    <!-- 随机密码 -->
    <PasswordTool />

    <!-- 大小写 / 命名转换 -->
    <CaseTool />
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { NButton, NInput, useMessage } from "naive-ui";
import PasswordTool from "./PasswordTool.vue";
import CaseTool from "./CaseTool.vue";

const message = useMessage();

// ---------- 时间戳 ----------
const nowStamp = ref("");
const nowText = ref("");
const stampInput = ref("");
const stampResult = ref("");
let timer: ReturnType<typeof setInterval> | null = null;

function pad2(value: number): string {
  return String(value).padStart(2, "0");
}

function formatLocal(date: Date): string {
  return (
    `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())} ` +
    `${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`
  );
}

function tickNow(): void {
  const now = new Date();
  nowStamp.value = String(Math.floor(now.getTime() / 1000));
  nowText.value = formatLocal(now);
}

function convertStamp(): void {
  const raw = stampInput.value.trim();
  stampResult.value = "";
  if (!raw) return;
  if (/^\d{10,13}$/.test(raw)) {
    const num = Number(raw);
    // 10 位按秒，其余按毫秒
    const date = new Date(raw.length === 10 ? num * 1000 : num);
    if (Number.isNaN(date.getTime())) {
      stampResult.value = "时间戳无效";
      return;
    }
    stampResult.value = `${formatLocal(date)}（${raw.length === 10 ? "秒" : "毫秒"}时间戳）`;
    return;
  }
  const parsed = new Date(raw.replace(" ", "T"));
  if (Number.isNaN(parsed.getTime())) {
    stampResult.value = "无法识别：请输入时间戳或 YYYY-MM-DD HH:mm:ss";
    return;
  }
  stampResult.value = `秒：${Math.floor(parsed.getTime() / 1000)} · 毫秒：${parsed.getTime()}`;
}

// ---------- UUID ----------
const uuids = ref<string[]>([]);

function genUuids(): void {
  uuids.value = Array.from({ length: 5 }, () => crypto.randomUUID());
}

// ---------- Base64 ----------
const b64Input = ref("");
const b64Result = ref("");

function runBase64(mode: "encode" | "decode"): void {
  const text = b64Input.value;
  if (!text.trim()) {
    message.warning("请先输入内容");
    return;
  }
  try {
    if (mode === "encode") {
      const bytes = new TextEncoder().encode(text);
      let binary = "";
      bytes.forEach((byte) => {
        binary += String.fromCharCode(byte);
      });
      b64Result.value = btoa(binary);
    } else {
      const binary = atob(text.trim());
      const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
      b64Result.value = new TextDecoder().decode(bytes);
    }
  } catch {
    b64Result.value = "";
    message.error(mode === "decode" ? "不是有效的 Base64 内容" : "编码失败");
  }
}

// ---------- URL 编码 ----------
const urlInput = ref("");
const urlResult = ref("");

function runUrl(mode: "encode" | "decode"): void {
  const text = urlInput.value;
  if (!text.trim()) {
    message.warning("请先输入内容");
    return;
  }
  try {
    urlResult.value =
      mode === "encode" ? encodeURIComponent(text) : decodeURIComponent(text);
  } catch {
    urlResult.value = "";
    message.error(mode === "decode" ? "不是有效的 URL 编码内容" : "编码失败");
  }
}

function copy(text: string): void {
  void navigator.clipboard
    .writeText(text)
    .then(() => message.success("已复制"))
    .catch(() => message.error("复制失败"));
}

onMounted(() => {
  tickNow();
  timer = setInterval(tickNow, 1000);
});

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});
</script>

<style scoped>
.coder-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  align-items: start;
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

.actions {
  display: flex;
  gap: 4px;
}

.result-line {
  font-size: 13px;
  line-height: 1.6;
  padding: 8px 10px;
  background: var(--card, #fff);
  border: 1px solid var(--border);
  border-radius: 6px;
  word-break: break-all;
}

.uuid-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.uuid-row {
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  word-break: break-all;
}

.uuid-row:hover {
  background: var(--primary-hover-bg, rgba(79, 99, 231, 0.08));
}

@media (max-width: 900px) {
  .coder-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
