<template>
  <div class="page">
    <PageHead
      title="开发工具"
      subtitle="前端日常小工具：JSON / 编解码 / 正则，全部本地处理，不上传。"
    />

    <n-tabs v-model:value="tool" type="segment" animated>
      <n-tab-pane name="json" tab="JSON 工具">
        <JsonTool />
      </n-tab-pane>
      <n-tab-pane name="coder" tab="编解码">
        <CoderTool />
      </n-tab-pane>
      <n-tab-pane name="regex" tab="正则测试">
        <RegexTool />
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NTabPane, NTabs } from "naive-ui";
import PageHead from "../components/PageHead.vue";
import JsonTool from "../components/dev/JsonTool.vue";
import CoderTool from "../components/dev/CoderTool.vue";
import RegexTool from "../components/dev/RegexTool.vue";

const tool = ref("json");
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

:deep(.n-tabs-tab) {
  font-size: 14px;
}
</style>
