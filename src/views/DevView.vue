<template>
  <div class="page">
    <PageHead title="开发工具" subtitle="前端日常小工具，全部本地处理，不上传。" icon="code" />

    <div class="seg-wrap">
      <n-tabs
        v-model:value="tool"
        type="segment"
        size="small"
        :pane-style="{ padding: '16px 0 0' }"
      >
        <n-tab-pane name="json">
          <template #tab>
            <span class="tab-label"><AppIcon name="braces" :size="16" />JSON 工具</span>
          </template>
          <JsonTool />
        </n-tab-pane>
        <n-tab-pane name="coder">
          <template #tab>
            <span class="tab-label"><AppIcon name="hash" :size="16" />编解码</span>
          </template>
          <CoderTool />
        </n-tab-pane>
        <n-tab-pane name="regex">
          <template #tab>
            <span class="tab-label"><AppIcon name="search" :size="16" />正则测试</span>
          </template>
          <RegexTool />
        </n-tab-pane>
        <n-tab-pane name="curl">
          <template #tab>
            <span class="tab-label"><AppIcon name="terminal" :size="16" />curl 转换</span>
          </template>
          <CurlTool />
        </n-tab-pane>
        <n-tab-pane name="assist">
          <template #tab>
            <span class="tab-label"><AppIcon name="wand" :size="16" />辅助工具</span>
          </template>
          <div class="dev-grid">
            <PasswordTool />
            <CaseTool />
          </div>
        </n-tab-pane>
      </n-tabs>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NTabPane, NTabs } from "naive-ui";
import PageHead from "../components/PageHead.vue";
import AppIcon from "../components/AppIcon.vue";
import JsonTool from "../components/dev/JsonTool.vue";
import CoderTool from "../components/dev/CoderTool.vue";
import RegexTool from "../components/dev/RegexTool.vue";
import CurlTool from "../components/dev/CurlTool.vue";
import PasswordTool from "../components/dev/PasswordTool.vue";
import CaseTool from "../components/dev/CaseTool.vue";

const tool = ref("json");
</script>

<style scoped>
.tab-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

/* 辅助工具双卡网格：靠 gap 对齐顶部，卡片不叠加自身 margin */
.dev-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

@media (max-width: 900px) {
  .dev-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
