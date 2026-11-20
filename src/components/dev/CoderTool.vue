<template>
  <div class="dev-grid">
    <!-- 时间戳 -->
    <section class="card">
      <h3 class="card-title"><AppIcon name="clock" :size="16" />时间戳</h3>
      <div class="spread">
        <div class="ts-now">
          <span class="hint">当前 Unix 时间戳（秒）</span>
          <span class="hint">{{ nowText }}</span>
        </div>
        <span class="ts-live mono">{{ nowStamp }}</span>
      </div>
      <div class="row">
        <input
          v-model="stampInput"
          class="input mono grow"
          placeholder="输入时间戳，如 1763600000"
          @keyup.enter="convertStamp"
        />
        <button class="btn btn-outline btn-sm" @click="convertStamp">转换</button>
      </div>
      <p class="hint ts-out" :class="{ 'ts-err': stampError }">{{ stampOutput }}</p>
    </section>

    <!-- UUID -->
    <section class="card">
      <div class="card-head">
        <h3 class="card-title"><AppIcon name="hash" :size="16" />UUID 生成</h3>
        <button class="btn btn-primary btn-sm" @click="genUuids">生成 5 个</button>
      </div>
      <div v-if="uuids.length" class="uuid-list">
        <div v-for="item in uuids" :key="item" class="row uuid-row">
          <span class="mono grow ellipsis">{{ item }}</span>
          <button
            class="btn btn-ghost btn-sm btn-icon"
            :class="{ loading: copyingUuid === item }"
            :disabled="copyingUuid === item"
            title="复制"
            @click="copyUuid(item)"
          >
            <AppIcon name="copy" :size="16" />
          </button>
        </div>
      </div>
      <p v-else class="hint">点击生成 v4 UUID，可单独复制。</p>
    </section>

    <!-- Base64 -->
    <section class="card">
      <h3 class="card-title"><AppIcon name="lock" :size="16" />Base64</h3>
      <textarea
        v-model="b64Input"
        class="textarea mono codec-in"
        spellcheck="false"
        placeholder="输入文本或 Base64，支持中文"
      ></textarea>
      <div class="row">
        <button class="btn btn-primary btn-sm" @click="runBase64('encode')">编码</button>
        <button class="btn btn-outline btn-sm" @click="runBase64('decode')">解码</button>
        <span class="grow"></span>
        <button
          class="btn btn-ghost btn-sm btn-icon"
          :class="{ loading: copyingTarget === 'b64' }"
          :disabled="copyingTarget === 'b64' || b64Error || !b64Result"
          title="复制结果"
          @click="copyValue(b64Result, 'b64')"
        >
          <AppIcon name="copy" :size="16" />
        </button>
      </div>
      <pre class="term codec-out"><span :class="b64Error ? 'err' : ''">{{ b64Result || "// 结果" }}</span></pre>
    </section>

    <!-- URL 编码 -->
    <section class="card">
      <h3 class="card-title"><AppIcon name="link" :size="16" />URL 编码</h3>
      <input v-model="urlInput" class="input mono" placeholder="https://example.com/search?q=桌面工具箱" />
      <div class="row">
        <button class="btn btn-primary btn-sm" @click="runUrl('encode')">编码</button>
        <button class="btn btn-outline btn-sm" @click="runUrl('decode')">解码</button>
        <span class="grow"></span>
        <button
          class="btn btn-ghost btn-sm btn-icon"
          :class="{ loading: copyingTarget === 'url' }"
          :disabled="copyingTarget === 'url' || urlError || !urlResult"
          title="复制结果"
          @click="copyValue(urlResult, 'url')"
        >
          <AppIcon name="copy" :size="16" />
        </button>
      </div>
      <pre class="term codec-out"><span :class="urlError ? 'err' : ''">{{ urlResult || "// 结果" }}</span></pre>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { copyText } from "../../utils/clipboard";

const message = useMessage();

// ---------- 时间戳 ----------
const nowStamp = ref("");
const nowText = ref("");
const stampInput = ref("");
const stampOutput = ref("支持秒 / 毫秒，自动识别；也可输入 YYYY-MM-DD HH:mm:ss。");
const stampError = ref(false);
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
  stampError.value = false;
  if (!raw) {
    stampOutput.value = "支持秒 / 毫秒，自动识别；也可输入 YYYY-MM-DD HH:mm:ss。";
    return;
  }
  if (/^\d{10,13}$/.test(raw)) {
    const num = Number(raw);
    // 10 位按秒，其余按毫秒
    const date = new Date(raw.length === 10 ? num * 1000 : num);
    if (Number.isNaN(date.getTime())) {
      stampError.value = true;
      stampOutput.value = "时间戳无效";
      return;
    }
    stampOutput.value =
      `${formatLocal(date)}（${raw.length === 10 ? "秒" : "毫秒"}时间戳） · ` +
      `ISO ${date.toISOString()}`;
    return;
  }
  const parsed = new Date(raw.replace(" ", "T"));
  if (Number.isNaN(parsed.getTime())) {
    stampError.value = true;
    stampOutput.value = "无法识别：请输入时间戳或 YYYY-MM-DD HH:mm:ss";
    return;
  }
  stampOutput.value = `秒：${Math.floor(parsed.getTime() / 1000)} · 毫秒：${parsed.getTime()}`;
}

// ---------- UUID ----------
const uuids = ref<string[]>([]);
const copyingUuid = ref("");

function genUuids(): void {
  uuids.value = Array.from({ length: 5 }, () => crypto.randomUUID());
}

async function copyUuid(text: string): Promise<void> {
  if (copyingUuid.value) return;
  copyingUuid.value = text;
  try {
    await copyText(text);
    message.success("已复制 UUID");
  } catch {
    message.error("复制失败");
  } finally {
    copyingUuid.value = "";
  }
}

// ---------- Base64 ----------
const b64Input = ref("");
const b64Result = ref("");
const b64Error = ref(false);

function runBase64(mode: "encode" | "decode"): void {
  const text = b64Input.value;
  b64Error.value = false;
  if (!text.trim()) {
    b64Result.value = "";
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
    b64Result.value = mode === "decode" ? "// 解码失败：不是合法的 Base64" : "// 编码失败";
    b64Error.value = true;
  }
}

// ---------- URL 编码 ----------
const urlInput = ref("");
const urlResult = ref("");
const urlError = ref(false);

function runUrl(mode: "encode" | "decode"): void {
  const text = urlInput.value;
  urlError.value = false;
  if (!text.trim()) {
    urlResult.value = "";
    message.warning("请先输入内容");
    return;
  }
  try {
    urlResult.value = mode === "encode" ? encodeURIComponent(text) : decodeURIComponent(text);
  } catch {
    urlResult.value = mode === "decode" ? "// 解码失败：不是合法的 URL 编码" : "// 编码失败";
    urlError.value = true;
  }
}

// ---------- 复制 ----------
/** 正在复制哪张卡片的结果，避免重复点击 */
const copyingTarget = ref<"" | "b64" | "url">("");

async function copyValue(text: string, target: "b64" | "url"): Promise<void> {
  if (!text || copyingTarget.value) return;
  copyingTarget.value = target;
  try {
    await copyText(text);
    message.success("已复制");
  } catch {
    message.error("复制失败");
  } finally {
    copyingTarget.value = "";
  }
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
.dev-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.card {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 卡片本身已有 gap，抵消全局卡片标题的下外边距 */
.card-head {
  margin-bottom: 0;
}

.ts-now {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.ts-live {
  font-size: var(--fs-16);
  font-weight: 600;
  color: var(--primary);
  font-variant-numeric: tabular-nums;
}

.ts-out {
  margin: 0;
  word-break: break-all;
}

.ts-err {
  color: var(--danger);
}

.uuid-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.uuid-row {
  gap: 8px;
}

.codec-in {
  min-height: 72px;
  font-family: var(--font-mono);
}

.codec-out {
  margin: 0;
  min-height: 56px;
  max-height: 160px;
  overflow: auto;
}

@media (max-width: 900px) {
  .dev-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
