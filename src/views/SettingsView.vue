<template>
  <div class="page">
    <PageHead title="设置" subtitle="下载、翻译、启动与命令执行。" icon="settings" />

    <EmptyState
      v-if="!hasConfig"
      title="配置尚未加载"
      description="设置依赖后端返回的配置，请稍后重试。"
    >
      <n-button :loading="retrying" :disabled="retrying" @click="retryLoad">重试</n-button>
    </EmptyState>

    <template v-else>
      <DownloadCard :draft="draft" />
      <TranslateCard
        :draft="draft"
        :cleared="clearedSecrets"
        @set-secret="setSecret"
        @clear-secret="clearSecret"
      />
      <AutostartCard :startup="draft.startup" />
      <CommandCard :draft="draft" />
      <NotifyCard :draft="draft" />
      <BackupPanel />
      <AboutCard :version="versionLabel" :platform="platformLabel" :data-dir="dataDir" />
      <SettingsSaveBar :dirty="dirty" :saving="saving" @discard="discard" @save="save" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { NButton, useDialog, useMessage } from "naive-ui";
import { api } from "../api";
import { loadSnapshot, store } from "../store";
import { describeError } from "../utils/errors";
import { saveConfigWithConflict } from "../utils/configConflict";
import { normalizeSettingsDraft } from "../utils/settingsPayload";
import { defaultLlmConfig } from "../utils/llm";
import type { AppConfig, SecretPatch } from "../types";
import PageHead from "../components/PageHead.vue";
import EmptyState from "../components/EmptyState.vue";
import DownloadCard from "../components/settings/DownloadCard.vue";
import TranslateCard from "../components/settings/TranslateCard.vue";
import AutostartCard from "../components/settings/AutostartCard.vue";
import CommandCard from "../components/settings/CommandCard.vue";
import NotifyCard from "../components/settings/NotifyCard.vue";
import BackupPanel from "../components/settings/BackupPanel.vue";
import AboutCard from "../components/settings/AboutCard.vue";
import SettingsSaveBar from "../components/settings/SettingsSaveBar.vue";
import type { SecretKey } from "../components/settings/secret";

const message = useMessage();
const dialog = useDialog();

function cloneConfig(config: AppConfig): AppConfig {
  return JSON.parse(JSON.stringify(config)) as AppConfig;
}

function emptyConfig(): AppConfig {
  return {
    schemaVersion: 1,
    downloadDir: "",
    downloadConcurrency: 2,
    fragmentThreads: 4,
    proxy: "",
    toolDownloadMirror: "",
    toolPaths: { ytdlp: "", ffmpeg: "" },
    startup: { startMinimized: true, delaySeconds: 10 },
    translate: {
      provider: "microsoft",
      fallback: ["google", "openai"],
      ttsRate: 1,
      openai: { baseUrl: "https://api.openai.com/v1", apiKey: "", model: "gpt-4o-mini" },
      deepl: { apiKey: "", endpoint: "https://api-free.deepl.com/v2/translate" },
      baidu: { appId: "", secretKey: "" },
      youdao: { appKey: "", appSecret: "" },
    },
    history: { enabled: true, maxEntries: 200 },
    security: { allowCommands: false },
    notify: { download: true, task: true },
    quickTranslate: { enabled: false, shortcut: "CommandOrControl+Shift+T" },
    llm: defaultLlmConfig(),
  };
}

function errorText(err: unknown): string {
  return describeError(err);
}

const draft = ref<AppConfig>(store.config ? cloneConfig(store.config) : emptyConfig());
const hasConfig = computed(() => store.config !== null);
const saving = ref(false);
const retrying = ref(false);

const clearedSecrets = reactive<Record<SecretKey, boolean>>({
  openai: false,
  deepl: false,
  baidu: false,
  youdao: false,
});

const draftRev = ref(0);
watch(
  draft,
  () => {
    draftRev.value += 1;
  },
  { deep: true },
);

function setSecret(key: SecretKey, value: string): void {
  const translate = draft.value.translate;
  if (key === "openai") translate.openai.apiKey = value;
  else if (key === "deepl") translate.deepl.apiKey = value;
  else if (key === "baidu") translate.baidu.secretKey = value;
  else translate.youdao.appSecret = value;
  clearedSecrets[key] = false;
}

function clearSecret(key: SecretKey): void {
  setSecret(key, "");
  clearedSecrets[key] = true;
}

function resetSecretState(): void {
  clearedSecrets.openai = false;
  clearedSecrets.deepl = false;
  clearedSecrets.baidu = false;
  clearedSecrets.youdao = false;
}

function blankSecretInputs(config: AppConfig): void {
  config.translate.openai.apiKey = "";
  config.translate.deepl.apiKey = "";
  config.translate.baidu.secretKey = "";
  config.translate.youdao.appSecret = "";
}

function buildSecretPatch(config: AppConfig): SecretPatch {
  const translate = config.translate;
  const patch: SecretPatch = {};
  if (clearedSecrets.openai) patch.openaiApiKey = "";
  else if (translate.openai.apiKey.trim()) patch.openaiApiKey = translate.openai.apiKey.trim();
  if (clearedSecrets.deepl) patch.deeplApiKey = "";
  else if (translate.deepl.apiKey.trim()) patch.deeplApiKey = translate.deepl.apiKey.trim();
  if (clearedSecrets.baidu) patch.baiduSecretKey = "";
  else if (translate.baidu.secretKey.trim()) patch.baiduSecretKey = translate.baidu.secretKey.trim();
  if (clearedSecrets.youdao) patch.youdaoAppSecret = "";
  else if (translate.youdao.appSecret.trim()) patch.youdaoAppSecret = translate.youdao.appSecret.trim();
  return patch;
}

function resetDraft(): void {
  draft.value = store.config ? cloneConfig(store.config) : emptyConfig();
  resetSecretState();
}

async function retryLoad(): Promise<void> {
  if (retrying.value) return;
  retrying.value = true;
  try {
    await loadSnapshot();
    resetDraft();
  } finally {
    retrying.value = false;
  }
}

// 数据目录随平台而异，展示后端解析出的实际路径。
const dataDir = ref("");

onMounted(() => {
  if (!store.config) resetDraft();
  api
    .getDataDir()
    .then((dir) => {
      dataDir.value = dir;
    })
    .catch(() => {
      dataDir.value = "";
    });
});

const dirty = computed(() => {
  if (!store.config) return false;
  return JSON.stringify(draft.value) !== JSON.stringify(store.config);
});

const platformLabel = computed(() => {
  const platform = store.platform;
  if (platform === "windows") return "Windows";
  if (platform === "macos" || platform === "darwin") return "macOS";
  if (platform === "linux") return "Linux";
  return platform || "未知";
});

const versionLabel = computed(() => store.version || "0.1.0");

function reload(): void {
  if (!dirty.value) {
    resetDraft();
    message.info("已重新载入设置");
    return;
  }
  dialog.warning({
    title: "放弃未保存的修改？",
    content: "重新载入会用已保存的配置替换页面上的编辑内容。",
    positiveText: "重新载入",
    negativeText: "取消",
    onPositiveClick: () => {
      resetDraft();
      message.info("已重新载入设置");
    },
  });
}

function discard(): void {
  reload();
}

async function save(): Promise<void> {
  if (!store.config || saving.value) return;
  const payload = normalizeSettingsDraft(draft.value);
  const rev = draftRev.value;
  const patch = buildSecretPatch(payload);

  saving.value = true;
  try {
    const result = await saveConfigWithConflict(
      dialog,
      (base) => api.saveConfig(payload, patch, "settings", base),
      store.configRevision,
      (conflict) => {
        store.config = conflict.config;
        store.configRevision = conflict.configRevision;
        resetDraft();
        message.info("已载入磁盘上的最新配置");
      },
    );
    if (!result) return;
    store.configRevision = result.configRevision;
    store.config = result.config;
    store.tools = result.tools;
    store.providers = result.providers;
    store.secrets = result.secrets;
    resetSecretState();
    if (draftRev.value === rev) {
      draft.value = cloneConfig(result.config);
    } else {
      blankSecretInputs(draft.value);
      message.success("设置已保存，保存期间的其他修改仍保留在页面");
      return;
    }
    message.success("设置已保存");
  } catch (err) {
    message.error(`保存失败：${errorText(err)}`);
  } finally {
    saving.value = false;
  }
}
</script>
