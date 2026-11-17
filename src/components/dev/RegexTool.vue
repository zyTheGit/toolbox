<template>
  <div class="regex-tool">
    <div class="card">
      <div class="pattern-row">
        <span class="regex-prefix mono">/</span>
        <n-input
          v-model:value="pattern"
          class="pattern-input mono"
          size="small"
          placeholder="\d{4}-\d{2}-\d{2}"
          spellcheck="false"
        />
        <span class="regex-prefix mono">/</span>
        <n-input
          v-model:value="flags"
          class="flags-input mono"
          size="small"
          placeholder="gim"
          spellcheck="false"
        />
        <n-select
          class="preset-select"
          size="small"
          :value="presetValue"
          :options="presetOptions"
          placeholder="常用预设"
          clearable
          @update:value="applyPreset"
        />
      </div>

      <n-alert v-if="regexError" type="error" :bordered="false" class="regex-error">
        正则无效：{{ regexError }}
      </n-alert>

      <n-input
        v-model:value="text"
        type="textarea"
        :autosize="false"
        class="test-text"
        placeholder="输入测试文本，匹配结果实时更新"
        spellcheck="false"
      />

      <div class="hint-row">
        <span class="hint">
          命中 {{ matches.length }} 处{{ groupNames ? " · 含命名分组" : "" }}，高亮部分为匹配结果
        </span>
        <n-button size="small" text type="primary" @click="copyMatches">复制命中项</n-button>
      </div>
    </div>

    <div v-if="segments.length" class="card">
      <div class="highlight mono">
        <template v-for="(segment, index) in segments" :key="index">
          <mark v-if="segment.hit" class="hit">{{ segment.text }}</mark>
          <template v-else>{{ segment.text }}</template>
        </template>
      </div>
    </div>

    <div v-if="matches.length" class="card">
      <div
        v-for="(match, index) in matches"
        :key="index"
        class="match-item"
      >
        <span class="match-index">#{{ index + 1 }}</span>
        <span class="mono match-text selectable">{{ match.text }}</span>
        <span class="hint">位置 {{ match.index }} · 长度 {{ match.text.length }}</span>
        <span v-if="match.groups" class="mono match-groups">{{ match.groups }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NAlert, NButton, NInput, NSelect, useMessage } from "naive-ui";

interface MatchInfo {
  text: string;
  index: number;
  groups: string;
}

interface Segment {
  text: string;
  hit: boolean;
}

const message = useMessage();

const pattern = ref("");
const flags = ref("g");
const text = ref("");
const regexError = ref("");

const presetOptions = [
  { label: "手机号", value: "1[3-9]\\d{9}" },
  { label: "邮箱", value: "[\\w.+-]+@[\\w-]+\\.[\\w.]+" },
  { label: "URL", value: "https?://[^\\s]+" },
  { label: "日期 YYYY-MM-DD", value: "\\d{4}-\\d{2}-\\d{2}" },
  { label: "IPv4", value: "(\\d{1,3}\\.){3}\\d{1,3}" },
  { label: "十六进制色值", value: "#[0-9a-fA-F]{3,8}" },
];

const presetValue = computed(
  () => presetOptions.find((item) => item.value === pattern.value)?.value ?? null,
);

const groupNames = computed(() => /<\w+>/.test(pattern.value));

/** 解析正则，无效时记录错误信息。 */
function tryBuild(): RegExp | null {
  regexError.value = "";
  const source = pattern.value.trim();
  if (!source || !text.value) return null;
  let cleanedFlags = flags.value.replace(/[^gimsuy]/g, "");
  try {
    return new RegExp(source, cleanedFlags);
  } catch (err) {
    regexError.value = (err as Error).message;
    return null;
  }
}

const matches = computed<MatchInfo[]>(() => {
  const regex = tryBuild();
  if (!regex) return [];
  const found: MatchInfo[] = [];
  try {
    if (regex.global) {
      for (const match of text.value.matchAll(regex)) {
        found.push(toInfo(match));
        if (found.length > 500) break;
      }
    } else {
      const match = regex.exec(text.value);
      if (match) found.push(toInfo(match));
    }
  } catch {
    return found;
  }
  return found;
});

function toInfo(match: RegExpMatchArray): MatchInfo {
  const named = match.groups ?? {};
  const namedText = Object.entries(named)
    .map(([key, value]) => `${key}=${value ?? ""}`)
    .join(" ");
  const numbered = match
    .slice(1)
    .filter((value) => value !== undefined)
    .join(" · ");
  const groups = [namedText, numbered].filter(Boolean).join(" · ");
  return { text: match[0], index: match.index ?? 0, groups };
}

/** 按命中位置把文本切成普通/命中交替片段，用于高亮渲染。 */
const segments = computed<Segment[]>(() => {
  const list = matches.value;
  if (!list.length) return [];
  const parts: Segment[] = [];
  let cursor = 0;
  for (const match of list) {
    if (match.index < cursor) continue;
    if (match.index > cursor) {
      parts.push({ text: text.value.slice(cursor, match.index), hit: false });
    }
    parts.push({ text: match.text, hit: true });
    cursor = match.index + match.text.length;
  }
  if (cursor < text.value.length) {
    parts.push({ text: text.value.slice(cursor), hit: false });
  }
  return parts;
});

function applyPreset(value: string | number | null): void {
  if (typeof value === "string") {
    pattern.value = value;
    if (!flags.value.includes("g")) flags.value = `g${flags.value}`;
  }
}

function copyMatches(): void {
  if (!matches.value.length) {
    message.warning("没有可复制的命中项");
    return;
  }
  const payload = matches.value.map((match) => match.text).join("\n");
  void navigator.clipboard
    .writeText(payload)
    .then(() => message.success(`已复制 ${matches.value.length} 个命中项`))
    .catch(() => message.error("复制失败"));
}
</script>

<style scoped>
.regex-tool {
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

.pattern-row {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.regex-prefix {
  font-size: 15px;
  color: var(--muted, #6b7280);
}

.pattern-input {
  flex: 1;
  min-width: 200px;
}

.flags-input {
  width: 64px;
}

.preset-select {
  width: 150px;
}

.regex-error {
  margin: 0;
}

.test-text {
  min-height: 140px;
}

.test-text :deep(.n-input-wrapper),
.test-text :deep(.n-input__textarea),
.test-text :deep(textarea) {
  height: 100%;
  min-height: 140px;
}

.test-text :deep(textarea) {
  font-family: Consolas, "JetBrains Mono", monospace;
  resize: none;
}

.hint-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.highlight {
  font-size: 13px;
  line-height: 1.9;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}

.hit {
  background: rgba(255, 213, 79, 0.5);
  border-radius: 3px;
  padding: 0 1px;
  color: inherit;
}

.match-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}

.match-item:last-child {
  border-bottom: none;
}

.match-index {
  color: var(--muted, #6b7280);
  flex: none;
}

.match-text {
  word-break: break-all;
}

.match-groups {
  color: var(--muted, #6b7280);
  word-break: break-all;
}
</style>
