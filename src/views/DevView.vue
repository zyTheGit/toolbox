<template>
  <div class="page">
    <PageHead title="开发工具" subtitle="前端日常小工具，全部本地处理，不上传。" />

    <n-tabs v-model:value="tool" type="segment" :theme-overrides="tabsTheme">
      <n-tab-pane name="json" tab="JSON 工具">
        <JsonTool />
      </n-tab-pane>
      <n-tab-pane name="coder" tab="编解码">
        <CoderTool />
      </n-tab-pane>
      <n-tab-pane name="regex" tab="正则测试">
        <RegexTool />
      </n-tab-pane>
      <n-tab-pane name="curl" tab="curl 转换">
        <CurlTool />
      </n-tab-pane>
      <n-tab-pane name="assist" tab="辅助工具">
        <div class="assist-grid">
          <PasswordTool />
          <CaseTool />
        </div>
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
import CurlTool from "../components/dev/CurlTool.vue";
import PasswordTool from "../components/dev/PasswordTool.vue";
import CaseTool from "../components/dev/CaseTool.vue";

const tool = ref("json");

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

.assist-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  /* 默认 stretch：两张卡片等高，避免空白露出页面底色 */
}

/* 全局 .card + .card 的上下间距会把网格里的卡片压下去，这里抵消 */
.assist-grid > .card {
  margin-top: 0;
}

@media (max-width: 900px) {
  .assist-grid {
    grid-template-columns: minmax(0, 1fr);
  }
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
