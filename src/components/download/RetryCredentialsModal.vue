<template>
  <n-modal
    :show="show"
    preset="card"
    title="重新填写下载凭据"
    :style="{ width: '480px', maxWidth: '92vw' }"
    :mask-closable="!submitting"
    :closable="!submitting"
    @update:show="emit('update:show', $event)"
  >
    <div class="hint modal-hint">
      代理 / Cookie / Referer / 附加参数只保存在内存中，应用重启后已丢失，需要重新填写。
      凭据不会写入 jobs.json。
    </div>

    <div class="field">
      <span class="label">Referer</span>
      <n-input v-model:value="draft.referer" aria-label="Referer" placeholder="部分站点需要来源页" :disabled="submitting" />
    </div>
    <div class="field">
      <span class="label">Cookie 来源</span>
      <n-select v-model:value="draft.cookiesBrowser" aria-label="Cookie 来源" :options="cookieOptions" :disabled="submitting" />
    </div>
    <div class="field">
      <span class="label">代理</span>
      <n-input
        v-model:value="draft.proxy"
        aria-label="代理"
        placeholder="留空则使用设置中的代理"
        :disabled="submitting"
      />
    </div>
    <div class="field">
      <span class="label">附加参数</span>
      <n-input
        v-model:value="draft.extraArgs"
        aria-label="附加参数"
        placeholder="例如 --add-header &quot;Authorization: Bearer ...&quot;"
        :disabled="submitting"
      />
    </div>

    <div v-if="error" class="error-text modal-error">{{ error }}</div>

    <template #footer>
      <div class="modal-foot">
        <n-button quaternary :disabled="submitting" @click="emit('update:show', false)">取消</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">重试</n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { reactive, watch } from "vue";
import { NButton, NInput, NModal, NSelect } from "naive-ui";
import type { CredentialDraft } from "./sessionCredentials";

const props = defineProps<{
  show: boolean;
  initial: CredentialDraft;
  submitting?: boolean;
  error?: string;
}>();

const emit = defineEmits<{
  "update:show": [value: boolean];
  submit: [value: CredentialDraft];
}>();

const cookieOptions = [
  { label: "不使用", value: "" },
  { label: "Chrome", value: "chrome" },
  { label: "Edge", value: "edge" },
  { label: "Firefox", value: "firefox" },
];

const draft = reactive<CredentialDraft>({
  referer: "",
  cookiesBrowser: "",
  proxy: "",
  extraArgs: "",
});

// 打开时用本会话最近一次输入的凭据预填；关闭后不保留未提交的修改。
watch(
  () => props.show,
  (visible) => {
    if (!visible) return;
    draft.referer = props.initial.referer;
    draft.cookiesBrowser = props.initial.cookiesBrowser;
    draft.proxy = props.initial.proxy;
    draft.extraArgs = props.initial.extraArgs;
  },
  { immediate: true },
);

function submit(): void {
  emit("submit", {
    referer: draft.referer,
    cookiesBrowser: draft.cookiesBrowser,
    proxy: draft.proxy,
    extraArgs: draft.extraArgs,
  });
}
</script>

<style scoped>
.modal-hint {
  margin-bottom: 14px;
}

.modal-error {
  margin-top: 4px;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.label {
  font-size: 12px;
  font-weight: 600;
  color: #4a5265;
}
</style>
