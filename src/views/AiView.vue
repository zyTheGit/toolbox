<template>
  <div class="page">
    <PageHead
      title="AI 工具"
      subtitle="多模型并行对比调试；凭据保存在本机，不会上传到任何服务器。"
      icon="sparkles"
      tag="实验"
    >
      <span class="pill" :class="dirty ? 'pill-warn' : 'pill-ok'">
        {{ dirty ? "AI 配置未保存" : "AI 配置已保存" }}
      </span>
      <n-button size="small" :disabled="!dirty || saving" :loading="saving" @click="save">
        <template #icon><AppIcon name="save" :size="16" /></template>
        保存 AI 配置
      </n-button>
    </PageHead>

    <div class="seg-wrap">
      <n-tabs v-model:value="tab" type="segment" size="small">
        <n-tab-pane name="llm">
          <template #tab>
            <span class="tab-label"><AppIcon name="zap" :size="16" />LLM 调试台</span>
          </template>
          <div class="tab-body"><LlmConsole /></div>
        </n-tab-pane>
        <n-tab-pane name="keys">
          <template #tab>
            <span class="tab-label"><AppIcon name="key" :size="16" />API Key 管理器</span>
          </template>
          <div class="tab-body"><KeyManager /></div>
        </n-tab-pane>
      </n-tabs>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NButton, NTabPane, NTabs } from "naive-ui";
import AppIcon from "../components/AppIcon.vue";
import PageHead from "../components/PageHead.vue";
import LlmConsole from "../components/ai/LlmConsole.vue";
import KeyManager from "../components/ai/KeyManager.vue";
import { useLlmConfigSave } from "../composables/useLlmConfigSave";
import { llmDirty, llmDraft } from "../composables/useLlmDraft";

const tab = ref("llm");

// 保存与状态提示放在页头，调试台里的模型配置、模板改动共用这份草稿
const dirty = llmDirty;
const { saving, save } = useLlmConfigSave(llmDraft);
</script>

<style scoped>
.tab-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.tab-body {
  /* 分段轨道下方与卡片之间留 16px（Naive 分段 tabs 自带 8px 内边距） */
  margin-top: 8px;
}
</style>
