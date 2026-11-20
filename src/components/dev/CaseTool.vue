<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="wand" :size="16" />大小写 / 命名转换</h3>
    </div>

    <textarea
      v-model="source"
      class="textarea mono case-in"
      spellcheck="false"
      placeholder="输入文本，如 desktop toolbox app"
    ></textarea>

    <div class="case-grid">
      <button
        v-for="item in CASE_ACTIONS"
        :key="item.key"
        class="btn btn-sm"
        :class="activeKey === item.key ? 'btn-primary' : 'btn-outline'"
        @click="convert(item.key)"
      >
        {{ item.label }}
      </button>
    </div>

    <div class="case-out">
      <span class="mono case-text" :class="{ placeholder: !output }">{{ output || "—" }}</span>
      <button
        class="btn btn-ghost btn-sm btn-icon"
        :class="{ loading: copying }"
        :disabled="copying || !output"
        title="复制结果"
        @click="copy"
      >
        <AppIcon name="copy" :size="16" />
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { CASE_ACTIONS, convertCase, type CaseKey } from "../../utils/text-case";
import { copyText } from "../../utils/clipboard";

const message = useMessage();

const source = ref("");
const output = ref("");
const activeKey = ref<CaseKey | "">("");
const copying = ref(false);

function convert(key: CaseKey): void {
  if (!source.value.trim()) {
    message.warning("请先输入文本");
    output.value = "";
    activeKey.value = "";
    return;
  }
  output.value = convertCase(source.value, key);
  activeKey.value = key;
}

async function copy(): Promise<void> {
  if (!output.value || copying.value) return;
  copying.value = true;
  try {
    await copyText(output.value);
    message.success("已复制");
  } catch {
    message.error("复制失败");
  } finally {
    copying.value = false;
  }
}
</script>

<style scoped>
.card {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 卡片本身已有 gap，抵消全局卡片标题的下外边距 */
.card-head {
  margin-bottom: 0;
}

.case-in {
  min-height: 72px;
  font-family: var(--font-mono);
}

.case-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.case-out {
  display: flex;
  align-items: center;
  gap: 8px;
}

.case-text {
  flex: 1;
  font-size: var(--fs-13);
  line-height: 1.6;
  word-break: break-all;
}

.case-text.placeholder {
  color: var(--muted);
}
</style>
