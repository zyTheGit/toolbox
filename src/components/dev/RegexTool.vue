<template>
  <div class="card regex-card">
    <div class="pattern-row">
      <span class="mono muted">/</span>
      <input
        v-model="pattern"
        class="input mono grow pattern-input"
        placeholder="1[3-9]\d{9}"
        spellcheck="false"
      />
      <span class="mono muted">/</span>
      <input
        v-model="flags"
        class="input mono flags-input"
        placeholder="gim"
        spellcheck="false"
      />
    </div>

    <div class="preset-row">
      <span class="hint">常用预设：</span>
      <button
        v-for="preset in PRESETS"
        :key="preset.label"
        class="chip"
        :class="{ on: pattern === preset.value }"
        @click="applyPreset(preset.value)"
      >
        {{ preset.label }}
      </button>
    </div>

    <div class="regex-grid">
      <div class="field">
        <label for="re-text">测试文本</label>
        <textarea
          id="re-text"
          v-model="text"
          class="textarea mono regex-text"
          spellcheck="false"
          placeholder="输入测试文本，匹配结果实时更新"
        ></textarea>
      </div>
      <div class="field">
        <label>匹配高亮 <span class="hint count">{{ countText }}</span></label>
        <div class="regex-view mono">
          <span v-if="regexError" class="danger">正则无效：{{ regexError }}</span>
          <span v-else-if="!text" class="faint">（空文本）</span>
          <template v-else-if="segments.length">
            <template v-for="(segment, index) in segments" :key="index">
              <mark v-if="segment.hit" class="match-mark">{{ segment.text }}</mark>
              <span v-else>{{ segment.text }}</span>
            </template>
          </template>
          <span v-else>{{ text }}</span>
        </div>
      </div>
    </div>

    <div class="table-wrap matches-wrap">
      <table class="tbl">
        <thead>
          <tr>
            <th>#</th>
            <th>匹配内容</th>
            <th>位置</th>
            <th>长度</th>
            <th v-if="hasGroups">分组</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="!matches.length">
            <td :colspan="hasGroups ? 5 : 4" class="no-match">没有匹配</td>
          </tr>
          <tr v-for="(match, index) in matches" :key="index">
            <td class="num">{{ index + 1 }}</td>
            <td class="mono match-cell">{{ match.text }}</td>
            <td class="num">{{ match.index }}</td>
            <td class="num">{{ match.text.length }}</td>
            <td v-if="hasGroups" class="mono faint match-cell">{{ match.groups || "—" }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="spread regex-foot">
      <span class="hint">
        支持 g i m s u y 标志{{ groupNames ? " · 已识别命名分组" : "" }}，最多展示 500 处匹配。
      </span>
      <button
        class="btn btn-outline btn-sm"
        :class="{ loading: copying }"
        :disabled="copying || !matches.length"
        @click="copyMatches"
      >
        <AppIcon name="copy" :size="16" />复制命中项
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { copyText } from "../../utils/clipboard";

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
const copying = ref(false);

const PRESETS = [
  { label: "手机号", value: "1[3-9]\\d{9}" },
  { label: "邮箱", value: "[\\w.+-]+@[\\w-]+\\.[\\w.]+" },
  { label: "URL", value: "https?://\\S+" },
  { label: "日期", value: "\\d{4}-\\d{2}-\\d{2}" },
  { label: "IPv4", value: "\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b" },
  { label: "色值", value: "#[0-9a-fA-F]{6}\\b" },
];

const groupNames = computed(() => /<\w+>/.test(pattern.value));

/** 解析正则，无效时记录错误信息。 */
function tryBuild(): RegExp | null {
  regexError.value = "";
  const source = pattern.value.trim();
  if (!source) return null;
  const cleanedFlags = flags.value.replace(/[^gimsuy]/g, "");
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

const hasGroups = computed(
  () => groupNames.value || matches.value.some((match) => Boolean(match.groups)),
);

const countText = computed(() => {
  if (regexError.value || !text.value) return "";
  return `· ${matches.value.length} 处匹配`;
});

function applyPreset(value: string): void {
  pattern.value = value;
  if (!flags.value.includes("g")) flags.value = `g${flags.value}`;
}

async function copyMatches(): Promise<void> {
  if (!matches.value.length || copying.value) {
    if (!matches.value.length) message.warning("没有可复制的命中项");
    return;
  }
  copying.value = true;
  try {
    await copyText(matches.value.map((match) => match.text).join("\n"));
    message.success(`已复制 ${matches.value.length} 个命中项`);
  } catch {
    message.error("复制失败");
  } finally {
    copying.value = false;
  }
}
</script>

<style scoped>
.regex-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.pattern-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.pattern-input {
  min-width: 200px;
}

.flags-input {
  width: 64px;
  flex: none;
}

.preset-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.regex-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.regex-text {
  min-height: 160px;
  font-family: var(--font-mono);
}

.regex-view {
  flex: 1;
  min-height: 80px;
  max-height: 240px;
  overflow: auto;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  background: var(--surface-soft);
  font-size: var(--fs-13);
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-all;
}

.count {
  margin-left: 2px;
}

.match-mark {
  background: var(--warning-soft);
  color: var(--warning);
  border-radius: 3px;
  padding: 0 1px;
  font-weight: 600;
}

.danger {
  color: var(--danger);
}

.matches-wrap {
  max-height: 300px;
  overflow: auto;
}

.matches-wrap thead th {
  position: sticky;
  top: 0;
  z-index: 1;
}

.match-cell {
  word-break: break-all;
}

.no-match {
  text-align: center;
  color: var(--muted);
}

.regex-foot {
  flex-wrap: wrap;
  gap: 10px;
}

@media (max-width: 900px) {
  .regex-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
