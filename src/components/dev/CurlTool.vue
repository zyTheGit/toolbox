<template>
  <div class="tool">
    <n-tabs v-model:value="mode" type="line" size="small">
      <n-tab-pane name="parse" tab="curl → 代码">
        <div class="split">
          <div class="pane">
            <div class="pane-head">
              <span class="pane-title">curl 命令</span>
              <n-button size="tiny" quaternary @click="input = example">填入示例</n-button>
            </div>
            <n-input
              v-model:value="input"
              type="textarea"
              class="mono"
              :autosize="{ minRows: 10, maxRows: 18 }"
              placeholder="粘贴 curl 命令，支持多行反斜杠续行"
            />
            <n-alert v-if="error" type="error" :bordered="false">{{ error }}</n-alert>
            <template v-else-if="request">
              <div class="summary">
                <n-tag size="small" :bordered="false" type="info">{{ request.method }}</n-tag>
                <span class="url" :title="request.url">{{ request.url }}</span>
              </div>
              <n-alert
                v-for="warning in request.warnings"
                :key="warning"
                type="warning"
                :bordered="false"
              >
                {{ warning }}
              </n-alert>
              <div v-if="bodySummary" class="body-hint">{{ bodySummary }}</div>
            </template>
          </div>
          <div class="pane">
            <n-tabs v-model:value="output" type="segment" size="small">
              <n-tab-pane name="fetch" tab="fetch">
                <CodePreview v-if="request" :code="fetchCode" language="ts" />
              </n-tab-pane>
              <n-tab-pane name="axios" tab="axios">
                <CodePreview v-if="request" :code="axiosCode" language="ts" />
              </n-tab-pane>
              <n-tab-pane name="python" tab="Python">
                <CodePreview v-if="request" :code="pythonCode" language="python" />
              </n-tab-pane>
            </n-tabs>
            <div v-if="!request" class="empty-hint">输入合法的 curl 命令后在这里生成代码</div>
          </div>
        </div>
      </n-tab-pane>

      <n-tab-pane name="build" tab="请求 → curl">
        <div class="split">
          <div class="pane">
            <div class="build-row">
              <n-select v-model:value="method" size="small" :options="methodOptions" class="method" />
              <n-input v-model:value="url" size="small" placeholder="https://api.example.com/v1/chat" />
            </div>
            <div class="field">
              <span class="label">请求头（每行一个 Name: Value）</span>
              <n-input
                v-model:value="headers"
                type="textarea"
                class="mono"
                :autosize="{ minRows: 4, maxRows: 8 }"
                placeholder="Content-Type: application/json"
              />
            </div>
            <div class="field">
              <span class="label">请求体</span>
              <n-input
                v-model:value="body"
                type="textarea"
                class="mono"
                :autosize="{ minRows: 4, maxRows: 10 }"
                placeholder='{"model": "gpt-4o-mini"}'
              />
            </div>
          </div>
          <div class="pane">
            <CodePreview v-if="builtCurl" :code="builtCurl" language="ts" />
            <div v-else class="empty-hint">填写 URL 后在这里生成 curl</div>
          </div>
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NAlert, NButton, NInput, NSelect, NTabPane, NTabs, NTag } from "naive-ui";
import CodePreview from "./CodePreview.vue";
import {
  parseCurl,
  parseHeadersText,
  toAxios,
  toCurl,
  toFetch,
  toPython,
  type CurlRequest,
} from "../../utils/curl";

const mode = ref("parse");
const output = ref("fetch");
const input = ref("");
const method = ref("POST");
const url = ref("");
const headers = ref("Content-Type: application/json");
const body = ref("");

const example = `curl 'https://api.openai.com/v1/chat/completions' \\
  -H 'Content-Type: application/json' \\
  -H 'Authorization: Bearer sk-xxx' \\
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"你好"}]}'`;

const methodOptions = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"].map((value) => ({
  label: value,
  value,
}));

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

const bodySummary = computed(() => {
  const current = request.value;
  if (!current) return "";
  if (current.form.length) return `表单字段 ${current.form.length} 个`;
  if (!current.body) return "";
  const text = current.body.replace(/\s+/g, " ");
  return `请求体：${text.length > 100 ? `${text.slice(0, 100)}…` : text}`;
});

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
</script>

<style scoped>
.tool {
  min-height: 320px;
}
.split {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  align-items: start;
}
.pane {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}
.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.pane-title,
.label {
  font-size: 12px;
  font-weight: 600;
  color: #4b5468;
}
.summary {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.url {
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.body-hint {
  font-size: 12px;
  color: var(--muted);
  word-break: break-all;
}
.empty-hint {
  padding: 32px 8px;
  text-align: center;
  color: var(--muted);
  font-size: 13px;
  border: 1px dashed var(--border);
  border-radius: 8px;
}
.build-row {
  display: flex;
  gap: 8px;
}
.method {
  width: 110px;
  flex: none;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.mono :deep(textarea) {
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
}
@media (max-width: 1000px) {
  .split {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
