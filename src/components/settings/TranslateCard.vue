<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="translate" :size="16" />翻译</h3>
    </div>

    <SettingsRow label="默认服务" desc="翻译页优先使用的服务商">
      <n-select
        class="w-ctrl"
        aria-label="默认翻译服务"
        :value="primaryProvider"
        :options="providerOptions"
        @update:value="setPrimary"
      />
    </SettingsRow>

    <SettingsRow
      label="失败自动回退"
      desc="主服务失败时按勾选顺序回退；未配置 Key 的服务自动跳过"
    >
      <div class="fallback-list">
        <div v-for="item in fallbackCandidates" :key="item.id" class="check-row">
          <n-checkbox
            :checked="isFallback(item.id)"
            @update:checked="(checked) => toggleFallback(item.id, Boolean(checked))"
          >
            {{ item.name }}
          </n-checkbox>
          <span v-if="item.requiresKey && !item.ready" class="tag">未配置 Key</span>
        </div>
        <span v-if="!fallbackCandidates.length" class="hint">没有可选的回退服务。</span>
      </div>
    </SettingsRow>

    <SettingsRow label="朗读倍率" desc="译文与原文的朗读速度">
      <n-select
        class="w-num"
        aria-label="朗读倍率"
        :value="draft.translate.ttsRate"
        :options="ttsRateOptions"
        @update:value="setTtsRate"
      />
    </SettingsRow>

    <SettingsRow label="保存翻译历史" desc="关闭后不再记录新的翻译">
      <n-switch v-model:value="draft.history.enabled" aria-label="保存翻译历史" />
      <span class="hint">最多保留</span>
      <n-input-number
        class="w-sm"
        aria-label="最多保留历史条数"
        :value="maxEntries"
        :min="1"
        :max="2000"
        :precision="0"
        :disabled="!draft.history.enabled"
        @update:value="setMaxEntries"
      />
      <span class="hint">条 · 收藏不计入</span>
    </SettingsRow>

    <n-collapse class="provider-config">
      <n-collapse-item title="各服务参数与 Key" name="provider-config">
        <div class="form-grid">
          <ConfigField
            v-model="draft.translate.openai.baseUrl"
            label="OpenAI · 接口地址"
            placeholder="https://api.openai.com/v1"
          />
          <SecretField
            :model-value="draft.translate.openai.apiKey"
            label="OpenAI · API Key"
            :stored="secretStored('openai')"
            :cleared="cleared.openai"
            @update:model-value="emit('set-secret', 'openai', $event)"
            @clear="emit('clear-secret', 'openai')"
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
            :cleared="cleared.deepl"
            @update:model-value="emit('set-secret', 'deepl', $event)"
            @clear="emit('clear-secret', 'deepl')"
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
            :cleared="cleared.baidu"
            @update:model-value="emit('set-secret', 'baidu', $event)"
            @clear="emit('clear-secret', 'baidu')"
          />
          <ConfigField v-model="draft.translate.youdao.appKey" label="有道 · App Key" />
          <SecretField
            :model-value="draft.translate.youdao.appSecret"
            label="有道 · App Secret"
            :stored="secretStored('youdao')"
            :cleared="cleared.youdao"
            @update:model-value="emit('set-secret', 'youdao', $event)"
            @clear="emit('clear-secret', 'youdao')"
          />
        </div>
        <SecretProtectionNotice />
      </n-collapse-item>
    </n-collapse>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NCheckbox, NCollapse, NCollapseItem, NInputNumber, NSelect, NSwitch } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import SettingsRow from "./SettingsRow.vue";
import ConfigField from "./ConfigField.vue";
import SecretField from "./SecretField.vue";
import SecretProtectionNotice from "./SecretProtectionNotice.vue";
import { store } from "../../store";
import type { AppConfig, ProviderId, ProviderInfo } from "../../types";
import type { SecretKey } from "./secret";

const props = defineProps<{
  draft: AppConfig;
  /** 当前页面标记为「保存后清除」的密钥 */
  cleared: Record<SecretKey, boolean>;
}>();

const emit = defineEmits<{
  "set-secret": [key: SecretKey, value: string];
  "clear-secret": [key: SecretKey];
}>();

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

// 读取后端返回的密钥存在状态，供 SecretField 展示「已保存」
function secretStored(key: SecretKey): boolean {
  return store.secrets[key];
}

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

const primaryProvider = computed<ProviderId>(() => props.draft.translate.provider ?? "microsoft");

const fallbackCandidates = computed(() =>
  providers.value.filter((item) => item.id !== primaryProvider.value),
);

function setPrimary(value: ProviderId | null): void {
  if (!value) return;
  props.draft.translate.provider = value;
  props.draft.translate.fallback = props.draft.translate.fallback.filter((id) => id !== value);
}

function isFallback(id: ProviderId): boolean {
  return props.draft.translate.fallback.includes(id);
}

function toggleFallback(id: ProviderId, checked: boolean): void {
  const selected = new Set(props.draft.translate.fallback);
  if (checked) selected.add(id);
  else selected.delete(id);
  props.draft.translate.fallback = PROVIDER_ORDER.filter(
    (provider) => provider !== props.draft.translate.provider && selected.has(provider),
  );
}

function setTtsRate(value: number | null): void {
  if (value == null) return;
  props.draft.translate.ttsRate = value;
}

const maxEntries = computed<number | null>(() => props.draft.history.maxEntries ?? null);

function setMaxEntries(value: number | null): void {
  if (value === null || !Number.isFinite(value)) return;
  props.draft.history.maxEntries = value;
}
</script>

<style scoped>
.w-ctrl {
  width: 220px;
}

.w-num {
  width: 220px;
}

.w-sm {
  width: 90px;
}

.fallback-list {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 6px 16px;
  max-width: 460px;
}

.check-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.provider-config {
  margin-top: 12px;
}

@media (max-width: 760px) {
  .w-ctrl,
  .w-num,
  .w-sm {
    width: 100%;
  }

  .fallback-list {
    justify-content: flex-start;
    max-width: none;
  }
}
</style>
