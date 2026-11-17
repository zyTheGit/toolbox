<template>
  <section class="card">
    <div class="tool-head">
      <span class="tool-title">随机密码</span>
      <n-tag size="small" :type="strength.type" :bordered="false">{{ strength.text }}</n-tag>
    </div>

    <div class="password-row">
      <span class="mono password-text" :class="{ placeholder: !password }">
        {{ password || emptyHint }}
      </span>
      <div class="actions">
        <n-button size="tiny" text type="primary" :disabled="!password" @click="copy">复制</n-button>
        <n-button size="tiny" text type="primary" @click="generate">重新生成</n-button>
      </div>
    </div>

    <div class="length-row">
      <span class="hint">长度</span>
      <n-slider v-model:value="length" class="length-slider" :min="8" :max="64" />
      <span class="mono length-value">{{ length }}</span>
    </div>

    <div class="option-row">
      <n-checkbox v-model:checked="sets.lower">小写</n-checkbox>
      <n-checkbox v-model:checked="sets.upper">大写</n-checkbox>
      <n-checkbox v-model:checked="sets.digit">数字</n-checkbox>
      <n-checkbox v-model:checked="sets.symbol">符号</n-checkbox>
      <n-checkbox v-model:checked="excludeAmbiguous">排除易混淆</n-checkbox>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watchEffect } from "vue";
import { NButton, NCheckbox, NSlider, NTag, useMessage } from "naive-ui";

const message = useMessage();

const POOLS = {
  lower: "abcdefghijklmnopqrstuvwxyz",
  upper: "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
  digit: "0123456789",
  symbol: "!@#$%^&*()-_=+[]{}:,.?/",
} as const;

/** 易混淆字符：数字 0 / 字母 O、数字 1 / 字母 l、大写 I、竖线 */
const AMBIGUOUS = "0O1lI|";

const sets = reactive({ lower: true, upper: true, digit: true, symbol: false });
const excludeAmbiguous = ref(true);
const length = ref(16);
const password = ref("");

/** 过滤易混淆字符后的可用字符集 */
const activeSets = computed(() => {
  const keys = Object.keys(POOLS) as (keyof typeof POOLS)[];
  return keys
    .filter((key) => sets[key])
    .map((key) => filterAmbiguous(POOLS[key], excludeAmbiguous.value));
});

const emptyHint = computed(() =>
  activeSets.value.length ? "点击生成" : "至少选择一种字符类型",
);

const strength = computed(() => {
  const classes = activeSets.value.length;
  const size = password.value.length;
  if (!size) return { text: "—", type: "default" as const };
  let score = 0;
  if (size >= 12) score += 1;
  if (size >= 16) score += 1;
  if (classes >= 3) score += 1;
  if (classes >= 4) score += 1;
  if (score >= 4) return { text: "强", type: "success" as const };
  if (score >= 2) return { text: "中", type: "warning" as const };
  return { text: "弱", type: "error" as const };
});

function filterAmbiguous(pool: string, exclude: boolean): string {
  if (!exclude) return pool;
  return [...pool].filter((char) => !AMBIGUOUS.includes(char)).join("");
}

/** 拒绝采样取随机下标，避免取模带来的分布偏差 */
function randomInt(max: number): number {
  if (max <= 1) return 0;
  const limit = Math.floor(0x1_0000_0000 / max) * max;
  const buffer = new Uint32Array(1);
  for (;;) {
    crypto.getRandomValues(buffer);
    if (buffer[0] < limit) return buffer[0] % max;
  }
}

function pickOne(pool: string): string {
  return pool.charAt(randomInt(pool.length));
}

function generate(): void {
  const pools = activeSets.value.filter(Boolean);
  if (!pools.length) {
    password.value = "";
    return;
  }
  // 每种选中的字符集至少出现一次，保证复杂度
  const chars = pools.map((pool) => pickOne(pool));
  const all = pools.join("");
  while (chars.length < length.value) chars.push(pickOne(all));
  for (let i = chars.length - 1; i > 0; i -= 1) {
    const j = randomInt(i + 1);
    [chars[i], chars[j]] = [chars[j], chars[i]];
  }
  password.value = chars.slice(0, length.value).join("");
}

function copy(): void {
  if (!password.value) return;
  void navigator.clipboard
    .writeText(password.value)
    .then(() => message.success("密码已复制"))
    .catch(() => message.error("复制失败"));
}

// 长度或字符集变化时立即刷新，避免界面上留着旧密码
watchEffect(() => generate());
</script>

<style scoped>
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

.password-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.password-text {
  flex: 1;
  font-size: 14px;
  letter-spacing: 0.4px;
  line-height: 1.6;
  word-break: break-all;
}

.password-text.placeholder {
  font-size: 13px;
  letter-spacing: 0;
  color: var(--muted);
}

.actions {
  display: flex;
  flex: none;
  gap: 4px;
}

.length-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.length-slider {
  flex: 1;
}

.length-value {
  width: 24px;
  text-align: right;
}

.option-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 14px;
  /* 卡片被拉高时把选项贴到底部，避免内容挤在顶部 */
  margin-top: auto;
}
</style>
