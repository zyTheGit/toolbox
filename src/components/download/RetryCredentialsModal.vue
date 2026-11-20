<template>
  <n-modal
    :show="show"
    preset="card"
    title="重新填写下载凭据"
    :style="{ width: '560px', maxWidth: '92vw' }"
    :mask-closable="!submitting"
    :closable="!submitting"
    @update:show="emit('update:show', $event)"
  >
    <div class="stack">
      <div class="alert alert-warn">
        <AppIcon name="key" :size="16" />
        <span>
          代理 / Cookie / Referer / 附加参数只保存在内存中，应用重启后已丢失，需要重新填写。
          凭据不会写入 jobs.json。
        </span>
      </div>

      <div class="form-grid">
        <div class="field">
          <label>Referer</label>
          <n-input
            v-model:value="draft.referer"
            aria-label="Referer"
            placeholder="部分站点需要来源页"
            :disabled="submitting"
          />
        </div>
        <div class="field">
          <label>Cookie 来源</label>
          <n-select
            v-model:value="draft.cookiesBrowser"
            aria-label="Cookie 来源"
            :options="cookieOptions"
            :disabled="submitting"
          />
        </div>
        <div class="field span-2">
          <label>代理</label>
          <n-input
            v-model:value="draft.proxy"
            aria-label="代理"
            placeholder="留空则使用设置中的代理"
            :disabled="submitting"
          />
        </div>
        <div class="field span-2">
          <label>附加参数</label>
          <n-input
            v-model:value="draft.extraArgs"
            class="mono"
            aria-label="附加参数"
            placeholder="例如 --add-header &quot;Authorization: Bearer ...&quot;"
            :disabled="submitting"
          />
        </div>
      </div>

      <div v-if="error" class="alert alert-err">
        <AppIcon name="alert" :size="16" />
        <span>{{ error }}</span>
      </div>
    </div>

    <template #footer>
      <div class="modal-foot">
        <n-button quaternary :disabled="submitting" @click="emit('update:show', false)">取消</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">
          <template #icon><AppIcon name="refresh" :size="16" /></template>
          重试
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { reactive, watch } from "vue";
import { NButton, NInput, NModal, NSelect } from "naive-ui";
import AppIcon from "../AppIcon.vue";
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
