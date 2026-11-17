<template>
  <div class="page">
    <PageHead title="m3u8 下载" subtitle="调用 yt-dlp 下载，支持批量链接、并发任务与实时进度。">
      <n-button size="small" @click="openDownloadDir">打开下载目录</n-button>
    </PageHead>

    <DownloadForm />
    <JobList />
  </div>
</template>

<script setup lang="ts">
import { NButton, useMessage } from "naive-ui";
import PageHead from "../components/PageHead.vue";
import DownloadForm from "../components/download/DownloadForm.vue";
import JobList from "../components/download/JobList.vue";
import { api } from "../api";
import { store } from "../store";
import { describeError } from "../utils/errors";

const message = useMessage();

function errorText(err: unknown): string {
  return describeError(err);
}

async function openDownloadDir(): Promise<void> {
  try {
    const path = await api.openFolder(store.config?.downloadDir);
    message.success(`已打开 ${path}`);
  } catch (err) {
    message.error(errorText(err));
  }
}
</script>