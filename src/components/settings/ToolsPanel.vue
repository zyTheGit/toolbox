<template>
  <div class="tools">
    <div v-for="row in rows" :key="row.key" class="tool-item">
      <div class="tool-row">
        <span class="lbl grow">{{ row.label }}</span>
        <span class="mono muted ellipsis tool-ver" :title="toolTitle(row.status)">
          {{ toolVersion(row.status) }}
        </span>
        <span class="pill" :class="pillClass(row.status)">{{ pillText(row.status) }}</span>
        <n-button
          v-if="!row.status?.ok"
          size="small"
          type="primary"
          :loading="installing[row.key]"
          :disabled="installing[row.key]"
          @click="install(row.key, row.label)"
        >
          <template #icon><AppIcon name="download" :size="16" /></template>
          一键下载
        </n-button>
      </div>
      <div v-if="installing[row.key]" class="install-line">
        <div class="progress thin">
          <i :style="{ width: `${installPercent(row.key)}%` }" />
        </div>
        <span class="hint">{{ phaseText(row.key) }}</span>
      </div>
    </div>
  </div>

  <div class="tools-foot">
    <p class="hint">
      工具按需下载到数据目录 bin/，安装前进行 SHA256 校验，校验失败自动终止并保留旧版本。
    </p>
    <n-button size="small" quaternary :loading="detecting" :disabled="detecting" @click="refreshTools">
      <template #icon><AppIcon name="refresh" :size="16" /></template>
      重新检测
    </n-button>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { NButton, useMessage } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { api } from "../../api";
import { store } from "../../store";
import { describeError } from "../../utils/errors";
import type { ToolProgress, ToolStatus } from "../../types";
import type { ToolKey } from "./secret";

const message = useMessage();

const detecting = ref(false);
const installing = reactive<Record<ToolKey, boolean>>({ ytdlp: false, ffmpeg: false });

const rows = computed(() => [
  { key: "ytdlp" as ToolKey, label: "yt-dlp", status: store.tools?.ytdlp },
  { key: "ffmpeg" as ToolKey, label: "ffmpeg", status: store.tools?.ffmpeg },
]);

function progressOf(key: ToolKey): ToolProgress | null {
  if (key === "ytdlp") return store.toolProgress["ytdlp"] ?? store.toolProgress["yt-dlp"] ?? null;
  return store.toolProgress["ffmpeg"] ?? null;
}

function shortVersion(version: string): string {
  const match = version.match(/\d+(?:\.\d+){1,3}/);
  if (match) return match[0];
  return version.trim().split(/\s+/)[0] ?? "";
}

function toolVersion(status: ToolStatus | undefined): string {
  if (!status) return "—";
  if (!status.ok) return status.path || "—";
  return shortVersion(status.version) || status.path;
}

function toolTitle(status: ToolStatus | undefined): string {
  if (!status) return "尚未检测";
  return [status.version, status.path].filter(Boolean).join("\n");
}

function pillText(status: ToolStatus | undefined): string {
  if (!status) return "未检测";
  return status.ok ? "就绪" : "缺失";
}

function pillClass(status: ToolStatus | undefined): string {
  if (!status) return "pill-idle";
  return status.ok ? "pill-ok" : "pill-err";
}

function installPercent(key: ToolKey): number {
  const pct = progressOf(key)?.pct;
  if (pct == null || !Number.isFinite(pct)) return 100;
  return Math.max(0, Math.min(100, Math.round(pct)));
}

function phaseText(key: ToolKey): string {
  const progress = progressOf(key);
  if (!progress) return "正在安装…";
  if (progress.phase === "extract") return "正在解压…";
  if (progress.phase === "done") return "安装完成，SHA256 校验通过";
  const pct = progress.pct;
  return pct == null ? "正在下载…" : `正在下载 ${Math.round(pct)}%`;
}

async function refreshTools(): Promise<void> {
  if (detecting.value) return;
  detecting.value = true;
  try {
    store.tools = await api.getTools(true);
    message.success("工具状态已更新");
  } catch (err) {
    message.error(`检测失败：${describeError(err)}`);
  } finally {
    detecting.value = false;
  }
}

async function install(key: ToolKey, name: string): Promise<void> {
  if (installing[key]) return;
  installing[key] = true;
  try {
    const result = await api.installTool(key);
    message.success(`${name} 安装完成：${result.version || result.path}`);
    try {
      store.tools = await api.getTools(true);
    } catch {
      store.tools = result.tools;
    }
  } catch (err) {
    message.error(`${name} 安装失败：${describeError(err)}`);
  } finally {
    installing[key] = false;
  }
}
</script>

<style scoped>
.tools {
  display: flex;
  flex-direction: column;
  margin-top: 12px;
}

.tool-item {
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}

.tool-item:last-child {
  border-bottom: 0;
}

.tool-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.lbl {
  font-size: var(--fs-13);
  font-weight: 500;
}

.tool-ver {
  max-width: 42%;
}

.install-line {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
}

.install-line .progress {
  flex: 1;
  min-width: 0;
}

.install-line .hint {
  flex: none;
}

.tools-foot {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-top: 8px;
}

.tools-foot .hint {
  margin: 0;
}

@media (max-width: 760px) {
  .tool-row {
    flex-wrap: wrap;
    gap: 8px 12px;
  }

  .tool-ver {
    max-width: 100%;
  }

  .install-line {
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }

  .tools-foot {
    flex-direction: column;
  }
}
</style>
