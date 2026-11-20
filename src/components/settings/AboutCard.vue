<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="info" :size="16" />关于</h3>
    </div>

    <div class="about">
      <div class="kv"><b>版本</b><span>v{{ version }} · {{ platform }}</span></div>
      <div class="kv"><b>数据目录</b><span class="mono">{{ dataDir || "…" }}</span></div>
    </div>

    <SettingsRow label="新版本" :desc="updateDesc">
      <span v-if="pending" class="update-found">发现 v{{ pending.latestVersion }}</span>
      <n-button
        v-if="pending"
        size="small"
        type="primary"
        :loading="opening"
        :disabled="opening"
        @click="openDownload"
      >
        打开下载页
      </n-button>
      <n-button v-if="pending" size="small" quaternary :disabled="opening" @click="skip">
        跳过此版本
      </n-button>
      <n-button v-if="showUndoSkip" size="small" quaternary @click="undoSkip">撤销跳过</n-button>
      <n-button
        size="small"
        :loading="checking"
        :disabled="checking || opening"
        @click="check"
      >
        {{ failed ? "重新检查" : "检查更新" }}
      </n-button>
    </SettingsRow>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NButton } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
import { useUpdateCheck } from "../../composables/useUpdateCheck";

defineProps<{
  version: string;
  platform: string;
  dataDir: string;
}>();

const { state, pendingUpdate, showUndoSkip, runUpdateCheck, openDownloadPage, skipUpdateVersion, clearSkippedVersion } =
  useUpdateCheck();

const opening = ref(false);
const pending = pendingUpdate;
const checking = computed(() => state.checking);
const failed = computed(() => Boolean(state.error) && !state.result);

const updateDesc = computed(() => {
  const result = state.result;
  if (state.error && !result) return `检查失败：${state.error}`;
  if (state.checking) return "正在检查…";
  if (pending.value) {
    return pending.value.publishedAt
      ? `线上最新 v${pending.value.latestVersion}，发布于 ${pending.value.publishedAt}，下载由浏览器完成。`
      : `线上最新 v${pending.value.latestVersion}，下载由浏览器完成。`;
  }
  if (result?.available) return `已跳过 v${result.latestVersion}，需要时可在「撤销跳过」后重新提示。`;
  if (result?.currentNewer) {
    return `当前 v${result.currentVersion} 高于线上最新 v${result.latestVersion}，请确认版本号是否标注有误。`;
  }
  if (result) return `已是最新版本 v${result.latestVersion}。`;
  return "启动时会自动检查一次，也可手动检查。";
});

async function check(): Promise<void> {
  if (state.checking) return;
  await runUpdateCheck();
}

async function openDownload(): Promise<void> {
  if (opening.value) return;
  opening.value = true;
  try {
    await openDownloadPage();
  } finally {
    opening.value = false;
  }
}

function skip(): void {
  skipUpdateVersion();
}

function undoSkip(): void {
  clearSkippedVersion();
}
</script>

<style scoped>
.about {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border);
}

.kv b {
  min-width: 56px;
}

.update-found {
  font-size: var(--fs-13);
  font-weight: 600;
  color: var(--primary);
}
</style>
