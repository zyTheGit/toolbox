<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="download" :size="16" />下载</h3>
    </div>

    <SettingsRow label="保存目录" desc="下载产物的默认保存位置">
      <n-input
        v-model:value="draft.downloadDir"
        class="w-ctrl"
        aria-label="保存目录"
        placeholder="留空使用默认目录"
      />
      <n-button
        class="icon-only"
        :loading="browsing"
        :disabled="browsing"
        title="浏览"
        aria-label="浏览保存目录"
        @click="browse"
      >
        <template #icon><AppIcon name="folder" :size="16" /></template>
      </n-button>
      <n-button
        class="icon-only"
        quaternary
        :loading="opening"
        :disabled="opening"
        title="打开目录"
        aria-label="打开下载目录"
        @click="openDownloadDir"
      >
        <template #icon><AppIcon name="folder-open" :size="16" /></template>
      </n-button>
    </SettingsRow>

    <SettingsRow label="同时下载数" desc="超出数量的任务自动排队">
      <n-select
        class="w-num"
        aria-label="同时下载任务数"
        :value="draft.downloadConcurrency"
        :options="concurrencyOptions"
        @update:value="setConcurrency"
      />
    </SettingsRow>

    <SettingsRow label="分片线程数" desc="m3u8 分片并发下载线程">
      <n-select
        class="w-num"
        aria-label="分片线程数"
        :value="draft.fragmentThreads"
        :options="threadOptions"
        @update:value="setThreads"
      />
    </SettingsRow>

    <SettingsRow label="全局代理" desc="留空不使用代理">
      <n-input
        v-model:value="draft.proxy"
        class="w-ctrl mono"
        aria-label="全局代理"
        placeholder="http://127.0.0.1:7890"
      />
    </SettingsRow>

    <SettingsRow label="工具下载镜像" desc="yt-dlp / ffmpeg 的下载来源，留空使用官方地址">
      <n-input
        v-model:value="draft.toolDownloadMirror"
        class="w-ctrl mono"
        aria-label="工具下载镜像"
        placeholder="https://ghproxy.net/"
      />
    </SettingsRow>

    <ToolsPanel />

    <n-collapse class="tool-paths">
      <n-collapse-item title="自定义工具路径" name="tool-paths">
        <div class="form-grid">
          <ConfigField
            v-model="draft.toolPaths.ytdlp"
            label="yt-dlp 路径"
            placeholder="留空使用数据目录 bin/"
          />
          <ConfigField
            v-model="draft.toolPaths.ffmpeg"
            label="ffmpeg 路径"
            placeholder="留空使用数据目录 bin/"
          />
        </div>
      </n-collapse-item>
    </n-collapse>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { NButton, NCollapse, NCollapseItem, NInput, NSelect, useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
import ToolsPanel from "./ToolsPanel.vue";
import ConfigField from "./ConfigField.vue";
import { api } from "../../api";
import { describeError } from "../../utils/errors";
import type { AppConfig } from "../../types";

const props = defineProps<{ draft: AppConfig }>();

const message = useMessage();
const browsing = ref(false);
const opening = ref(false);

const concurrencyOptions = computed(() => {
  const values = [1, 2, 3, 4, 5];
  if (!values.includes(props.draft.downloadConcurrency)) {
    values.push(props.draft.downloadConcurrency);
  }
  return values.sort((a, b) => a - b).map((value) => ({ label: String(value), value }));
});

const threadOptions = computed(() => {
  const values = [4, 8, 12, 16];
  if (!values.includes(props.draft.fragmentThreads)) {
    values.push(props.draft.fragmentThreads);
  }
  return values.sort((a, b) => a - b).map((value) => ({ label: String(value), value }));
});

function setConcurrency(value: number | null): void {
  if (value == null) return;
  props.draft.downloadConcurrency = value;
}

function setThreads(value: number | null): void {
  if (value == null) return;
  props.draft.fragmentThreads = value;
}

async function browse(): Promise<void> {
  if (browsing.value) return;
  browsing.value = true;
  try {
    const picked = await api.pickFolder();
    if (picked) props.draft.downloadDir = picked;
  } catch (err) {
    message.error(`选择目录失败：${describeError(err)}`);
  } finally {
    browsing.value = false;
  }
}

async function openDownloadDir(): Promise<void> {
  if (opening.value) return;
  opening.value = true;
  try {
    const path = props.draft.downloadDir.trim() || undefined;
    await api.openFolder(path);
  } catch (err) {
    message.error(`打开目录失败：${describeError(err)}`);
  } finally {
    opening.value = false;
  }
}
</script>

<style scoped>
.w-ctrl {
  width: 220px;
}

.w-num {
  width: 220px;
}

.icon-only {
  width: var(--h-ctl);
  padding: 0;
}

.tool-paths {
  margin-top: 12px;
}

/* Naive 输入框内部才是真正的 input，等宽字体要下探一层才生效 */
.mono :deep(.n-input__input-el) {
  font-family: var(--font-mono);
  font-size: var(--fs-12);
}

@media (max-width: 760px) {
  .w-ctrl,
  .w-num {
    width: 100%;
  }
}
</style>
