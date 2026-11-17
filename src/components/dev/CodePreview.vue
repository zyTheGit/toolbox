<template>
  <div class="preview">
    <div class="preview-bar">
      <span class="lang">{{ language === "python" ? "Python" : "TypeScript" }}</span>
      <n-button size="tiny" quaternary @click="copy">复制</n-button>
    </div>
    <pre class="code"><span
      v-for="(token, index) in tokens"
      :key="index"
      :class="`tok-${token.kind}`"
    >{{ token.text }}</span></pre>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, useMessage } from "naive-ui";
import { highlightCode } from "../../utils/code-highlight";
import { copyText } from "../../utils/clipboard";

const props = defineProps<{
  code: string;
  language: "ts" | "python";
}>();

const message = useMessage();
const tokens = computed(() => highlightCode(props.code, props.language));

async function copy(): Promise<void> {
  try {
    await copyText(props.code);
    message.success("已复制");
  } catch (e) {
    message.error(`复制失败: ${String(e)}`);
  }
}
</script>

<style scoped>
.preview {
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  background: var(--panel-soft);
}
.preview-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 12px;
  color: var(--muted);
}
.lang {
  font-weight: 500;
}
.code {
  margin: 0;
  padding: 12px;
  overflow: auto;
  max-height: 340px;
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre;
  color: var(--text);
}
.tok-string {
  color: #1f9d6b;
}
.tok-comment {
  color: #9aa1b2;
  font-style: italic;
}
.tok-keyword {
  color: #4f63e7;
  font-weight: 600;
}
.tok-number {
  color: #b7791f;
}
</style>
