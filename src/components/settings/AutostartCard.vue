<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="power" :size="16" />启动与后台</h3>
    </div>

    <SettingsRow
      label="开机自启"
      :desc="supported ? '登录系统后自动启动 Toolbox' : '当前平台不支持开机自启'"
    >
      <n-switch
        aria-label="开机自启"
        :value="enabled"
        :loading="busy"
        :disabled="!supported || busy"
        @update:value="toggle"
      />
    </SettingsRow>

    <div v-if="mismatch" class="alert alert-warn mismatch">
      <AppIcon name="alert" :size="16" />
      <div class="mismatch-body">
        <span>启动项路径与当前程序不一致，可能移动过程序。</span>
        <n-button size="tiny" :loading="repairing" :disabled="repairing" @click="repair">
          修复
        </n-button>
      </div>
    </div>

    <SettingsRow
      v-if="supported"
      label="自启时仅驻留托盘"
      desc="开机自启时不弹出主窗口，仅驻留托盘"
    >
      <n-switch v-model:value="startup.startMinimized" aria-label="自启时仅驻留托盘" />
    </SettingsRow>

    <SettingsRow v-if="supported" label="启动延迟" desc="避免与开机高峰争抢资源">
      <n-select
        class="w-delay"
        aria-label="启动延迟"
        :value="startup.delaySeconds"
        :options="delayOptions"
        @update:value="setDelay"
      />
    </SettingsRow>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NButton, NSelect, NSwitch, useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
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
  if (value == null) return;
  props.startup.delaySeconds = value;
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
    message.error(`设置开机自启失败：${describeError(err)}`);
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
    if (
      next.enabled &&
      next.path &&
      next.expectedPath &&
      normalizePath(next.path) !== normalizePath(next.expectedPath)
    ) {
      message.warning("启动项路径仍不一致，请手动检查注册项");
    } else {
      message.success("已修复开机自启路径");
    }
  } catch (err) {
    message.error(`修复失败：${describeError(err)}`);
  } finally {
    repairing.value = false;
  }
}
</script>

<style scoped>
.mismatch {
  margin-top: 12px;
}

.mismatch-body {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex: 1;
  min-width: 0;
}

.w-delay {
  width: 220px;
}

@media (max-width: 760px) {
  .mismatch-body {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .w-delay {
    width: 100%;
  }
}
</style>
