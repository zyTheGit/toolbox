<template>
  <div class="page">
    <PageHead
      title="下载"
      icon="download"
      subtitle="调用 yt-dlp 下载视频、音频与字幕，支持 m3u8、YouTube、X、B 站链接或分享文本。"
    >
      <n-button :loading="openingDir" :disabled="openingDir" @click="openDownloadDir">
        <template #icon><AppIcon name="folder-open" :size="16" /></template>
        打开下载目录
      </n-button>
    </PageHead>

    <DownloadForm />
    <JobList />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NButton, useMessage } from "naive-ui";
import AppIcon from "../components/AppIcon.vue";
import PageHead from "../components/PageHead.vue";
import DownloadForm from "../components/download/DownloadForm.vue";
import JobList from "../components/download/JobList.vue";
import { api } from "../api";
import { store } from "../store";
import { describeError } from "../utils/errors";

const message = useMessage();
const openingDir = ref(false);

async function openDownloadDir(): Promise<void> {
  if (openingDir.value) return;
  openingDir.value = true;
  try {
    const path = await api.openFolder(store.config?.downloadDir);
    message.success(`已打开 ${path}`);
  } catch (err) {
    message.error(describeError(err));
  } finally {
    openingDir.value = false;
  }
}
</script>
