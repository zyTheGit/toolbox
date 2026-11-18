<template>
  <div class="page">
    <PageHead title="设置" subtitle="下载、翻译、启动与命令执行。" />

    <EmptyState
      v-if="!hasConfig"
      title="配置尚未加载"
      description="设置依赖后端返回的配置，请稍后重试。"
    >
      <n-button :loading="retrying" @click="retryLoad">重试</n-button>
    </EmptyState>

    <template v-else>
      <section class="card">
        <h2 class="section-title">下载</h2>
        <div class="stack">
          <div class="field">
            <span class="field-label">保存目录</span>
            <div class="row">
              <n-input v-model:value="draft.downloadDir" class="grow" aria-label="保存目录" placeholder="留空使用默认目录" />
              <n-button @click="browse">浏览</n-button>
              <n-button quaternary @click="openDownloadDir">打开</n-button>
            </div>
          </div>

          <div class="grid">
            <div class="field">
              <span class="field-label">同时下载</span>
              <div class="row">
                <n-input-number
                  class="num-input"
                  aria-label="同时下载任务数"
                  :value="draft.downloadConcurrency"
                  :min="1"
                  :max="5"
                  :precision="0"
                  @update:value="setConcurrency"
                />
                <span class="muted">个任务并行</span>
              </div>
            </div>
            <div class="field">
              <span class="field-label">分片线程</span>
              <div class="row">
                <n-input-number
                  class="num-input"
                  aria-label="分片线程数"
                  :value="draft.fragmentThreads"
                  :min="1"
                  :max="16"
                  :precision="0"
                  @update:value="setFragmentThreads"
                />
                <span class="muted">单文件并发分片</span>
              </div>
            </div>
            <div class="field">
              <span class="field-label">全局代理</span>
              <n-input v-model:value="draft.proxy" aria-label="全局代理" placeholder="http://127.0.0.1:7890" />
            </div>
            <div class="field">
              <span class="field-label">工具下载镜像</span>
              <n-input v-model:value="draft.toolDownloadMirror" aria-label="工具下载镜像" placeholder="https://ghproxy.net/" />
            </div>
          </div>

          <div class="tools-block">
            <div class="tools-bar">
              <span class="field-label">工具</span>
              <n-button size="small" quaternary :loading="detecting" :disabled="detecting" @click="refreshTools">
                重新检测
              </n-button>
            </div>
            <div class="tool-list">
              <div v-for="row in toolRows" :key="row.key" class="tool-item">
                <div class="tool-line">
                  <span class="tool-name">{{ row.label }}</span>
                  <n-tag size="small" :bordered="false" :type="row.status?.ok ? 'success' : 'error'">
                    {{ row.status?.ok ? "就绪" : "缺失" }}
                  </n-tag>
                  <span
                    class="mono muted tool-meta ellipsis"
                    :title="toolTitle(row.status)"
                    :aria-label="toolTitle(row.status)"
                  >
                    {{ toolVersion(row.status) }}
                  </span>
                  <n-button
                    v-if="!row.status?.ok"
                    size="small"
                    type="primary"
                    :loading="installing[row.key]"
                    :disabled="installing[row.key]"
                    @click="install(row.key, row.label)"
                  >
                    一键下载
                  </n-button>
                </div>
                <n-progress
                  v-if="installing[row.key]"
                  type="line"
                  :percentage="installPercent(row.key)"
                  :processing="progressOf(row.key)?.pct == null"
                  :show-indicator="false"
                  :height="4"
                  class="install-progress"
                />
              </div>
            </div>
          </div>

          <n-collapse>
            <n-collapse-item title="自定义工具路径" name="tool-paths">
              <div class="grid">
                <ConfigField
                  v-model="draft.toolPaths.ytdlp"
                  label="yt-dlp 路径"
                  placeholder="留空自动检测"
                />
                <ConfigField
                  v-model="draft.toolPaths.ffmpeg"
                  label="ffmpeg 路径"
                  placeholder="留空自动检测"
                />
              </div>
            </n-collapse-item>
          </n-collapse>
        </div>
      </section>

      <section class="card">
        <h2 class="section-title">翻译</h2>
        <div class="stack">
          <div class="grid">
            <div class="field">
              <span class="field-label">默认服务</span>
              <n-select v-model:value="primaryProvider" aria-label="默认翻译服务" :options="providerOptions" />
            </div>
            <div class="field">
              <span class="field-label">回退服务</span>
              <div v-if="fallbackCandidates.length" class="check-list">
                <div v-for="item in fallbackCandidates" :key="item.id" class="check-row">
                  <n-checkbox
                    :checked="isFallback(item.id)"
                    @update:checked="(checked) => toggleFallback(item.id, Boolean(checked))"
                  >
                    {{ item.name }}
                  </n-checkbox>
                  <n-tag v-if="item.requiresKey && !item.ready" size="tiny" :bordered="false">
                    未配置 Key
                  </n-tag>
                </div>
              </div>
              <span v-else class="hint">没有可选的回退服务。</span>
            </div>
            <div class="field">
              <span class="field-label">译文朗读倍率</span>
              <n-select
                v-model:value="draft.translate.ttsRate"
                class="rate-input"
                aria-label="译文朗读倍率"
                :options="ttsRateOptions"
              />
            </div>
          </div>
          <div class="hint">默认服务失败时按勾选顺序回退；未配置 Key 的服务自动跳过。</div>

          <n-collapse>
            <n-collapse-item title="各服务参数与 Key" name="provider-config">
              <div class="grid">
                <ConfigField
                  v-model="draft.translate.openai.baseUrl"
                  label="OpenAI · 接口地址"
                  placeholder="https://api.openai.com/v1"
                />
                <SecretField
                  :model-value="draft.translate.openai.apiKey"
                  label="OpenAI · API Key"
                  :stored="secretStored('openai')"
                  :cleared="secretCleared('openai')"
                  @update:model-value="setSecret('openai', $event)"
                  @clear="clearSecret('openai')"
                />
                <ConfigField
                  v-model="draft.translate.openai.model"
                  label="OpenAI · 模型"
                  placeholder="gpt-4o-mini"
                />
                <SecretField
                  :model-value="draft.translate.deepl.apiKey"
                  label="DeepL · API Key"
                  :stored="secretStored('deepl')"
                  :cleared="secretCleared('deepl')"
                  @update:model-value="setSecret('deepl', $event)"
                  @clear="clearSecret('deepl')"
                />
                <ConfigField
                  v-model="draft.translate.deepl.endpoint"
                  label="DeepL · 接口地址"
                  placeholder="https://api-free.deepl.com/v2/translate"
                />
                <ConfigField v-model="draft.translate.baidu.appId" label="百度 · APP ID" />
                <SecretField
                  :model-value="draft.translate.baidu.secretKey"
                  label="百度 · 密钥"
                  :stored="secretStored('baidu')"
                  :cleared="secretCleared('baidu')"
                  @update:model-value="setSecret('baidu', $event)"
                  @clear="clearSecret('baidu')"
                />
                <ConfigField v-model="draft.translate.youdao.appKey" label="有道 · App Key" />
                <SecretField
                  :model-value="draft.translate.youdao.appSecret"
                  label="有道 · App Secret"
                  :stored="secretStored('youdao')"
                  :cleared="secretCleared('youdao')"
                  @update:model-value="setSecret('youdao', $event)"
                  @clear="clearSecret('youdao')"
                />
              </div>
              <SecretProtectionNotice />
            </n-collapse-item>
          </n-collapse>

          <div class="sub-block">
            <div class="spread">
              <div class="field-label">保存翻译历史</div>
              <n-switch v-model:value="draft.history.enabled" aria-label="保存翻译历史" />
            </div>
            <div class="row">
              <span class="field-label">最多保留</span>
              <n-input-number
                class="num-input"
                aria-label="最多保留历史条数"
                :value="maxEntries"
                :min="1"
                :max="2000"
                :disabled="!draft.history.enabled"
                @update:value="setMaxEntries"
              />
              <span class="muted">条 · 收藏不计入</span>
            </div>
          </div>
        </div>
      </section>

      <AutostartCard :startup="draft.startup" />

      <section class="card">
        <h2 class="section-title">命令执行</h2>
        <div class="stack">
          <div class="spread">
            <div>
              <div class="field-label">允许运行终端任务</div>
              <div class="hint">以当前用户权限执行，不提权；命令明文保存在本机。</div>
            </div>
            <n-switch v-model:value="draft.security.allowCommands" aria-label="允许运行终端任务" />
          </div>
          <n-alert v-if="!draft.security.allowCommands" type="warning" :bordered="false">
            终端任务当前不会触发，也无法立即运行。
          </n-alert>
        </div>
      </section>

      <section class="card">
        <h2 class="section-title">通知与快捷翻译</h2>
        <div class="stack">
          <div class="spread">
            <div>
              <div class="field-label">下载完成 / 失败通知</div>
              <div class="hint">仅在主窗口隐藏或最小化时提醒，点击通知回到下载页。</div>
            </div>
            <n-switch v-model:value="draft.notify.download" aria-label="下载完成与失败通知" />
          </div>
          <div class="spread">
            <div>
              <div class="field-label">终端任务完成 / 失败通知</div>
              <div class="hint">仅对终端任务生效，提醒任务保持原有通知。</div>
            </div>
            <n-switch v-model:value="draft.notify.task" aria-label="终端任务完成与失败通知" />
          </div>

          <div class="sub-block">
            <div class="spread">
              <div>
                <div class="field-label">全局快捷翻译</div>
                <div class="hint">
                  默认关闭。触发时只把剪贴板文本载入翻译页供预览，不会自动翻译或上传，需你确认后才会请求翻译服务。
                </div>
              </div>
              <n-switch v-model:value="draft.quickTranslate.enabled" aria-label="启用全局快捷翻译" />
            </div>
            <div class="row">
              <span class="field-label">快捷键</span>
              <n-input
                v-model:value="draft.quickTranslate.shortcut"
                class="shortcut-input"
                aria-label="快捷翻译快捷键"
                :disabled="!draft.quickTranslate.enabled"
                placeholder="CommandOrControl+Shift+T"
              />
            </div>
            <div class="hint">快捷键被其他程序占用时，保存会失败并提示，修改后可重试。</div>
          </div>
        </div>
      </section>

      <BackupPanel />

      <section class="card">
        <h2 class="section-title">关于</h2>
        <div class="meta-list">
          <div class="meta-row">
            <span class="meta-label">版本</span>
            <span class="mono">v{{ versionLabel }} · {{ platformLabel }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">数据</span>
            <span class="mono path-value">{{ dataDir || "…" }}</span>
          </div>
        </div>
      </section>

      <div class="save-bar">
        <span class="hint">{{ dirty ? "有未保存的修改" : "所有修改已保存" }}</span>
        <div class="save-bar-actions">
          <n-button size="small" :disabled="!dirty || saving" @click="discard">放弃修改</n-button>
          <n-button size="small" type="primary" :loading="saving" :disabled="!dirty || saving" @click="save">
            保存设置
          </n-button>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCheckbox,
  NCollapse,
  NCollapseItem,
  NInput,
  NInputNumber,
  NProgress,
  NSelect,
  NSwitch,
  NTag,
  useDialog,
  useMessage,
} from "naive-ui";
import { api } from "../api";
import { loadSnapshot, store } from "../store";
import { describeError } from "../utils/errors";
import { saveConfigWithConflict } from "../utils/configConflict";
import { normalizeSettingsDraft } from "../utils/settingsPayload";
import { defaultLlmConfig } from "../utils/llm";
import type {
  AppConfig,
  ProviderId,
  ProviderInfo,
  SecretPatch,
  ToolProgress,
  ToolStatus,
} from "../types";
import PageHead from "../components/PageHead.vue";
import EmptyState from "../components/EmptyState.vue";
import AutostartCard from "../components/settings/AutostartCard.vue";
import BackupPanel from "../components/settings/BackupPanel.vue";
import ConfigField from "../components/settings/ConfigField.vue";
import SecretField from "../components/settings/SecretField.vue";
import SecretProtectionNotice from "../components/settings/SecretProtectionNotice.vue";

const PROVIDER_ORDER: ProviderId[] = ["microsoft", "google", "openai", "deepl", "baidu", "youdao"];

const PROVIDER_NAMES: Record<ProviderId, string> = {
  microsoft: "微软",
  google: "Google",
  openai: "OpenAI",
  deepl: "DeepL",
  baidu: "百度",
  youdao: "有道",
};

const KEY_PROVIDERS = new Set<ProviderId>(["openai", "deepl", "baidu", "youdao"]);

const TTS_RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];
const ttsRateOptions = TTS_RATES.map((value) => ({ label: `${value}x`, value }));

type ToolKey = "ytdlp" | "ffmpeg";

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
const detecting = ref(false);
const retrying = ref(false);
const installing = ref<Record<ToolKey, boolean>>({ ytdlp: false, ffmpeg: false });

type SecretKey = "openai" | "deepl" | "baidu" | "youdao";

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

function secretStored(key: SecretKey): boolean {
  return store.secrets[key];
}

function secretCleared(key: SecretKey): boolean {
  return clearedSecrets[key];
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

const providers = computed<ProviderInfo[]>(() => {
  if (store.providers.length) return store.providers;
  return PROVIDER_ORDER.map((id) => ({
    id,
    name: PROVIDER_NAMES[id],
    requiresKey: KEY_PROVIDERS.has(id),
    ready: !KEY_PROVIDERS.has(id),
  }));
});

const providerOptions = computed(() =>
  providers.value.map((item) => ({
    label: item.requiresKey && !item.ready ? `${item.name}（未配置 Key）` : item.name,
    value: item.id,
  })),
);

const primaryProvider = computed<ProviderId>({
  get: () => draft.value.translate.provider ?? "microsoft",
  set: (value) => {
    draft.value.translate.provider = value;
    draft.value.translate.fallback = draft.value.translate.fallback.filter(
      (id) => id !== draft.value.translate.provider,
    );
  },
});

const fallbackCandidates = computed(() =>
  providers.value.filter((item) => item.id !== primaryProvider.value),
);

function isFallback(id: ProviderId): boolean {
  return Boolean(draft.value.translate.fallback.includes(id));
}

function toggleFallback(id: ProviderId, checked: boolean): void {
  const selected = new Set(draft.value.translate.fallback);
  if (checked) selected.add(id);
  else selected.delete(id);
  draft.value.translate.fallback = PROVIDER_ORDER.filter(
    (provider) => provider !== draft.value.translate.provider && selected.has(provider),
  );
}

const maxEntries = computed<number | null>(() => draft.value.history.maxEntries ?? null);

function setMaxEntries(value: number | null): void {
  if (value === null || !Number.isFinite(value)) return;
  draft.value.history.maxEntries = value;
}

function setConcurrency(value: number | null): void {
  draft.value.downloadConcurrency = value ?? 1;
}

function setFragmentThreads(value: number | null): void {
  draft.value.fragmentThreads = value ?? 1;
}

const toolRows = computed(() => [
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
  if (!status) return "";
  if (!status.ok) return status.path || "";
  return shortVersion(status.version) || status.path;
}

function toolTitle(status: ToolStatus | undefined): string {
  if (!status) return "";
  return [status.version, status.path].filter(Boolean).join("\n");
}

function installPercent(key: ToolKey): number {
  const pct = progressOf(key)?.pct;
  if (pct == null || !Number.isFinite(pct)) return 100;
  return Math.max(0, Math.min(100, Math.round(pct)));
}

const platformLabel = computed(() => {
  const platform = store.platform;
  if (platform === "windows") return "Windows";
  if (platform === "macos" || platform === "darwin") return "macOS";
  if (platform === "linux") return "Linux";
  return platform || "未知";
});

const versionLabel = computed(() => store.version || "0.1.0");

async function browse(): Promise<void> {
  try {
    const picked = await api.pickFolder();
    if (picked) draft.value.downloadDir = picked;
  } catch (err) {
    message.error(`选择目录失败：${errorText(err)}`);
  }
}

async function openDownloadDir(): Promise<void> {
  try {
    const path = draft.value.downloadDir.trim() || undefined;
    await api.openFolder(path);
  } catch (err) {
    message.error(`打开目录失败：${errorText(err)}`);
  }
}

async function refreshTools(): Promise<void> {
  if (detecting.value) return;
  detecting.value = true;
  try {
    store.tools = await api.getTools(true);
    message.success("工具状态已更新");
  } catch (err) {
    message.error(`检测失败：${errorText(err)}`);
  } finally {
    detecting.value = false;
  }
}

async function install(key: ToolKey, name: string): Promise<void> {
  if (installing.value[key]) return;
  installing.value[key] = true;
  try {
    const result = await api.installTool(key);
    message.success(`${name} 安装完成：${result.version || result.path}`);
    try {
      store.tools = await api.getTools(true);
    } catch {
      store.tools = result.tools;
    }
  } catch (err) {
    message.error(`${name} 安装失败：${errorText(err)}`);
  } finally {
    installing.value[key] = false;
  }
}

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

<style scoped>
/* 底部保存栏常驻吸附，页面底部留白与吸附间距保持一致，避免滚到底时位移 */
.page {
  padding-bottom: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.field-label {
  font-size: 12px;
  font-weight: 500;
  color: #4b5468;
}

.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px 18px;
}

.num-input {
  width: 96px;
}

.rate-input {
  width: 120px;
}

.shortcut-input {
  width: 260px;
  max-width: 100%;
}

.check-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 16px;
  padding-top: 4px;
}

.check-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tools-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.tools-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.tool-list {
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  background: var(--panel-soft);
}

.tool-item {
  padding: 8px 12px;
}

.tool-item + .tool-item {
  border-top: 1px solid var(--border);
}

.tool-line {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.tool-name {
  width: 56px;
  flex: none;
  font-size: 13px;
  font-weight: 600;
}

.tool-meta {
  flex: 1;
  min-width: 0;
}

.install-progress {
  margin-top: 6px;
}

.sub-block {
  border-top: 1px solid var(--border);
  padding-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.meta-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.meta-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.meta-label {
  width: 40px;
  flex: none;
  font-size: 12px;
  color: var(--muted);
}

.path-value {
  color: #4b5468;
  word-break: break-all;
}

.save-bar {
  position: sticky;
  bottom: 12px;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 16px;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: #fff;
  box-shadow: 0 6px 20px rgba(31, 41, 71, 0.12);
}

.save-bar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

@media (max-width: 720px) {
  .grid {
    grid-template-columns: 1fr;
  }
}
</style>