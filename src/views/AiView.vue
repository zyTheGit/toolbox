<template>
  <div class="page">
    <PageHead
      title="AI 工具"
      subtitle="多模型并行对比调试；凭据加密保存在本机，请求只发送到你配置的服务商。"
    />

    <n-tabs v-model:value="tab" type="segment" :theme-overrides="tabsTheme">
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

// 分段轨道比页面底色稍深，避免和背景连成一片；选中项用主色文字 + 加粗强化当前位置
const tabsTheme = {
  colorSegment: "#e3e8f1",
  tabColorSegment: "#ffffff",
  tabTextColorSegment: "#5b6475",
  tabTextColorHoverSegment: "#1c2333",
  tabTextColorActiveSegment: "#4f63e7",
  // segment 模式选中项的字重走这个变量，tabFontWeightActive 在该模式下不生效
  fontWeightStrong: "600",
};
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

:deep(.n-tabs-rail) {
  border: 1px solid #d3dbe7;
  box-shadow: inset 0 1px 2px rgba(28, 35, 51, 0.06);
}

:deep(.n-tabs-capsule) {
  box-shadow: 0 1px 4px rgba(28, 35, 51, 0.16);
}

:deep(.n-tabs-tab) {
  font-size: 14px;
  font-weight: 500;
}
</style>
