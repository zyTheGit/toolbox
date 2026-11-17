<template>
  <section class="card">
    <h2 class="section-title">启动与后台</h2>
    <div class="stack">
      <div class="spread">
        <div class="field-label">开机自启</div>
        <n-switch
          aria-label="开机自启"
          :value="enabled"
          :loading="busy"
          :disabled="!supported || busy"
          @update:value="toggle"
        />
      </div>

      <div v-if="!supported" class="hint">当前平台不支持开机自启。</div>

      <n-alert v-if="mismatch" type="warning" :bordered="false">
        <div>启动项路径与当前程序不一致，可能移动过程序。</div>
        <div class="repair-row">
          <n-button size="tiny" :loading="repairing" :disabled="repairing" @click="repair">
            修复
          </n-button>
        </div>
      </n-alert>

      <div v-if="supported" class="sub-block">
        <div class="spread">
          <div class="field-label">自启时仅驻留托盘</div>
          <n-switch v-model:value="startup.startMinimized" aria-label="自启时仅驻留托盘" />
        </div>
        <div class="row">
          <span class="field-label">启动延迟</span>
          <n-select
            class="delay-select"
            aria-label="启动延迟"
            :value="startup.delaySeconds"
            :options="delayOptions"
            @update:value="setDelay"
          />
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NAlert, NButton, NSelect, NSwitch, useMessage } from "naive-ui";
import { api } from "../../api";
import { store } from "../../store";
import { describeError } from "../../utils/errors";
import type { StartupConfig } from "../../types";

const props = defineProps<{ startup: StartupConfig }>();

const message = useMessage();

const enabled = ref(store.autostart?.enabled ?? false);
const busy = ref(false);
const repairing = ref(false);

const supported = computed(() => Boolean(store.autostart?.supported));

function normalizePath(value: string): string {
  return value.trim().replace(/^"+|"+$/g, "").toLowerCase();
}

const mismatch = computed(() => {
  const current = store.autostart;
  if (!current || !current.enabled) return false;
  if (!current.path || !current.expectedPath) return false;
  return normalizePath(current.path) !== normalizePath(current.expectedPath);
});

watch(
  () => store.autostart,
  (value) => {
    enabled.value = value?.enabled ?? false;
  },
  { deep: true },
);

const delayOptions = computed(() => {
  const values = [0, 10, 30];
  if (!values.includes(props.startup.delaySeconds)) values.push(props.startup.delaySeconds);
  return values.sort((a, b) => a - b).map((value) => ({
    label: value === 0 ? "不延迟" : `${value} 秒`,
    value,
  }));
});

function setDelay(value: number | null): void {
  props.startup.delaySeconds = value ?? 0;
}

function errorText(err: unknown): string {
  return describeError(err);
}

async function toggle(value: boolean): Promise<void> {
  if (!supported.value || busy.value) return;
  const previous = enabled.value;
  busy.value = true;
  enabled.value = value;
  try {
    const next = await api.autostartSet(value);
    store.autostart = next;
    enabled.value = next.enabled;
    message.success(next.enabled ? "已开启开机自启" : "已关闭开机自启");
  } catch (err) {
    enabled.value = previous;
    message.error(`设置开机自启失败：${errorText(err)}`);
  } finally {
    busy.value = false;
  }
}

async function repair(): Promise<void> {
  if (repairing.value) return;
  repairing.value = true;
  try {
    const next = await api.autostartSet(true);
    store.autostart = next;
    if (next.enabled && next.path && next.expectedPath && normalizePath(next.path) !== normalizePath(next.expectedPath)) {
      message.warning("启动项路径仍不一致，请手动检查注册项");
    } else {
      message.success("已修复开机自启路径");
    }
  } catch (err) {
    message.error(`修复失败：${errorText(err)}`);
  } finally {
    repairing.value = false;
  }
}
</script>

<style scoped>
.field-label {
  font-size: 12px;
  font-weight: 500;
  color: #4b5468;
}

.repair-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
}

.sub-block {
  border-top: 1px solid var(--border);
  padding-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.delay-select {
  width: 140px;
}
</style>
