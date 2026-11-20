<template>
  <div class="card curl-card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="terminal" :size="16" />curl 命令</h3>
      <div class="seg">
        <button :class="{ on: mode === 'parse' }" @click="mode = 'parse'">curl → 代码</button>
        <button :class="{ on: mode === 'build' }" @click="mode = 'build'">请求 → curl</button>
      </div>
    </div>

    <!-- curl 转代码 -->
    <template v-if="mode === 'parse'">
      <textarea
        v-model="input"
        class="textarea mono curl-in"
        spellcheck="false"
        placeholder="粘贴 curl 命令，支持多行反斜杠续行"
      ></textarea>

      <div class="row curl-actions">
        <button class="btn btn-primary btn-sm" @click="convert">转换为代码</button>
        <button class="btn btn-ghost btn-sm" @click="fillExample">填入示例</button>
      </div>

      <div v-if="error" class="alert alert-err curl-alert" role="alert">
        <AppIcon name="alert" :size="16" /><span>{{ error }}</span>
      </div>
      <template v-else-if="request">
        <div class="row curl-summary">
          <span class="tag tag-primary">{{ request.method }}</span>
          <span class="mono muted ellipsis grow">{{ request.url }}</span>
          <span v-if="bodySummary" class="hint ellipsis body-summary">{{ bodySummary }}</span>
        </div>
        <div v-for="warning in request.warnings" :key="warning" class="alert alert-warn curl-alert">
          <AppIcon name="alert" :size="16" /><span>{{ warning }}</span>
        </div>
      </template>
    </template>

    <!-- 请求转 curl -->
    <template v-else>
      <div class="row build-row">
        <select v-model="method" class="select build-method" aria-label="请求方法">
          <option v-for="item in METHOD_OPTIONS" :key="item" :value="item">{{ item }}</option>
        </select>
        <input v-model="url" class="input mono grow" placeholder="https://api.example.com/v1/chat" />
      </div>
      <div class="field build-field">
        <label>请求头（每行一个 Name: Value）</label>
        <textarea
          v-model="headers"
          class="textarea mono build-textarea"
          spellcheck="false"
          placeholder="Content-Type: application/json"
        ></textarea>
      </div>
      <div class="field build-field">
        <label>请求体</label>
        <textarea
          v-model="body"
          class="textarea mono build-textarea"
          spellcheck="false"
          placeholder='{"model": "gpt-4o-mini"}'
        ></textarea>
      </div>
    </template>

    <div class="row out-head">
      <span class="hint">{{ mode === "parse" ? "输出语言" : "生成的 curl 命令" }}</span>
      <div v-if="mode === 'parse'" class="seg" role="group" aria-label="输出语言">
        <button
          v-for="item in LANGS"
          :key="item.value"
          :class="{ on: language === item.value }"
          @click="language = item.value"
        >
          {{ item.label }}
        </button>
      </div>
      <span class="grow"></span>
      <button
        class="btn btn-outline btn-sm"
        :class="{ loading: copying }"
        :disabled="copying || !resultText"
        @click="copyResult"
      >
        <AppIcon name="copy" :size="16" />复制结果
      </button>
    </div>

    <CodePreview
      v-if="mode === 'parse' ? outputCode : builtCurl"
      :code="mode === 'parse' ? outputCode : builtCurl"
      :language="mode === 'parse' ? codeLanguage : 'ts'"
    />
    <div v-else class="term curl-out">// {{ mode === "parse" ? "点击「转换为代码」查看结果" : "填写 URL 后自动生成 curl" }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import CodePreview from "./CodePreview.vue";
import { copyText } from "../../utils/clipboard";
import {
  parseCurl,
  parseHeadersText,
  toAxios,
  toCurl,
  toFetch,
  toPython,
  type CurlRequest,
} from "../../utils/curl";

type Lang = "fetch" | "axios" | "python";

const message = useMessage();

const LANGS: { value: Lang; label: string }[] = [
  { value: "fetch", label: "fetch" },
  { value: "axios", label: "axios" },
  { value: "python", label: "Python" },
];

const METHOD_OPTIONS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];

const mode = ref<"parse" | "build">("parse");
const language = ref<Lang>("fetch");
const copying = ref(false);

// ---------- curl → 代码 ----------
const input = ref("");
/** 是否已点过「转换为代码」；编辑输入后回到初始占位 */
const converted = ref(false);

const example = `curl 'https://api.openai.com/v1/chat/completions' \\
  -H 'Content-Type: application/json' \\
  -H 'Authorization: Bearer sk-xxx' \\
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"你好"}]}'`;

const parsed = computed<{ request: CurlRequest | null; error: string }>(() => {
  const text = input.value.trim();
  if (!text) return { request: null, error: "" };
  try {
    return { request: parseCurl(text), error: "" };
  } catch (e) {
    return { request: null, error: e instanceof Error ? e.message : String(e) };
  }
});

const request = computed(() => parsed.value.request);
const error = computed(() => parsed.value.error);

const fetchCode = computed(() => (request.value ? toFetch(request.value) : ""));
const axiosCode = computed(() => (request.value ? toAxios(request.value) : ""));
const pythonCode = computed(() => (request.value ? toPython(request.value) : ""));

const outputCode = computed(() => {
  if (!converted.value) return "";
  if (language.value === "axios") return axiosCode.value;
  if (language.value === "python") return pythonCode.value;
  return fetchCode.value;
});

const codeLanguage = computed<"ts" | "python">(() =>
  language.value === "python" ? "python" : "ts",
);

const bodySummary = computed(() => {
  const current = request.value;
  if (!current) return "";
  if (current.form.length) return `表单字段 ${current.form.length} 个`;
  if (!current.body) return "";
  const text = current.body.replace(/\s+/g, " ");
  return `请求体：${text.length > 100 ? `${text.slice(0, 100)}…` : text}`;
});

function convert(): void {
  if (error.value) return;
  if (!request.value) {
    message.warning("请先粘贴 curl 命令");
    return;
  }
  converted.value = true;
}

function fillExample(): void {
  input.value = example;
  converted.value = false;
}

// 输入变化后需要重新确认转换，避免展示与输入不一致的旧代码
watch(input, () => {
  converted.value = false;
});

// ---------- 请求 → curl ----------
const method = ref("POST");
const url = ref("");
const headers = ref("Content-Type: application/json");
const body = ref("");

const builtCurl = computed(() => {
  if (!url.value.trim()) return "";
  return toCurl({
    method: method.value,
    url: url.value.trim(),
    headers: parseHeadersText(headers.value),
    body: body.value,
    form: [],
    warnings: [],
  });
});

// ---------- 复制 ----------
const resultText = computed(() =>
  mode.value === "parse" ? outputCode.value : builtCurl.value,
);

async function copyResult(): Promise<void> {
  if (!resultText.value || copying.value) {
    if (!resultText.value) message.warning("暂无可复制的代码");
    return;
  }
  copying.value = true;
  try {
    await copyText(resultText.value);
    message.success("已复制");
  } catch {
    message.error("复制失败");
  } finally {
    copying.value = false;
  }
}
</script>

<style scoped>
.curl-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 卡片本身已有 gap，抵消全局卡片标题的下外边距 */
.card-head {
  margin-bottom: 0;
}

.curl-in {
  min-height: 110px;
  font-family: var(--font-mono);
}

.curl-actions {
  flex-wrap: wrap;
}

.curl-alert {
  margin: 0;
}

.curl-summary {
  flex-wrap: wrap;
  gap: 8px;
  font-size: var(--fs-13);
}

.curl-summary .body-summary {
  max-width: 45%;
}

.build-row {
  gap: 8px;
}

.build-method {
  width: 110px;
  flex: none;
}

.build-field {
  gap: 6px;
}

.build-textarea {
  min-height: 84px;
  font-family: var(--font-mono);
}

.out-head {
  flex-wrap: wrap;
  gap: 10px;
}

.curl-out {
  margin: 0;
  min-height: 110px;
  max-height: 300px;
  overflow: auto;
}

@media (max-width: 900px) {
  .card-head {
    flex-wrap: wrap;
    justify-content: flex-start;
  }
}
</style>
