<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="key" :size="16" />随机密码</h3>
      <span class="pill" :class="strength.cls">{{ strength.text }}</span>
    </div>

    <div class="spread">
      <span class="hint">长度</span>
      <span class="mono len-value">{{ length }}</span>
    </div>
    <input
      v-model.number="length"
      class="range"
      type="range"
      min="8"
      max="64"
      aria-label="密码长度"
    />

    <div class="checks">
      <label class="check"><input v-model="sets.lower" type="checkbox" />小写</label>
      <label class="check"><input v-model="sets.upper" type="checkbox" />大写</label>
      <label class="check"><input v-model="sets.digit" type="checkbox" />数字</label>
      <label class="check"><input v-model="sets.symbol" type="checkbox" />符号</label>
      <label class="check"><input v-model="excludeAmbiguous" type="checkbox" />排除易混淆</label>
    </div>

    <div class="pwd-out">
      <span class="mono pwd-text" :class="{ placeholder: !password }">
        {{ password || emptyHint }}
      </span>
      <button
        class="btn btn-ghost btn-sm btn-icon"
        :class="{ loading: copying }"
        :disabled="copying || !password"
        title="复制"
        @click="copy"
      >
        <AppIcon name="copy" :size="16" />
      </button>
    </div>

    <button class="btn btn-primary btn-sm pwd-gen" @click="generate">
      <AppIcon name="refresh" :size="16" />重新生成
    </button>
  </section>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watchEffect } from "vue";
import { useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { copyText } from "../../utils/clipboard";

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
const copying = ref(false);

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
  if (!size) return { text: "—", cls: "pill-idle" };
  let score = 0;
  if (size >= 12) score += 1;
  if (size >= 16) score += 1;
  if (classes >= 3) score += 1;
  if (classes >= 4) score += 1;
  if (score >= 4) return { text: "强", cls: "pill-ok" };
  if (score >= 2) return { text: "中", cls: "pill-warn" };
  return { text: "弱", cls: "pill-err" };
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

async function copy(): Promise<void> {
  if (!password.value || copying.value) return;
  copying.value = true;
  try {
    await copyText(password.value);
    message.success("密码已复制");
  } catch {
    message.error("复制失败");
  } finally {
    copying.value = false;
  }
}

// 长度或字符集变化时立即刷新，避免界面上留着旧密码
watchEffect(() => generate());
</script>

<style scoped>
.card {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 卡片本身已有 gap，抵消全局卡片标题的下外边距 */
.card-head {
  margin-bottom: 0;
}

.len-value {
  font-variant-numeric: tabular-nums;
}

.checks {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  margin-top: 2px;
}

.pwd-out {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
}

.pwd-text {
  flex: 1;
  font-size: var(--fs-14);
  line-height: 1.6;
  letter-spacing: 0.4px;
  word-break: break-all;
}

.pwd-text.placeholder {
  font-size: var(--fs-13);
  letter-spacing: 0;
  color: var(--muted);
}

.pwd-gen {
  align-self: flex-start;
}
</style>
