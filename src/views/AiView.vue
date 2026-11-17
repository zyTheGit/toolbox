<template>
  <div class="page">
    <PageHead
      title="AI 工具"
      subtitle="多模型并行对比调试；凭据加密保存在本机，请求只发送到你配置的服务商。"
    />

    <n-tabs v-model:value="tab" type="segment">
      <n-tab-pane name="llm" tab="LLM 调试台">
        <LlmConsole />
      </n-tab-pane>
      <n-tab-pane name="keys" tab="API Key 管理器">
        <KeyManager />
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NTabPane, NTabs } from "naive-ui";
import PageHead from "../components/PageHead.vue";
import LlmConsole from "../components/ai/LlmConsole.vue";
import KeyManager from "../components/ai/KeyManager.vue";

const tab = ref("llm");
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
</style>
