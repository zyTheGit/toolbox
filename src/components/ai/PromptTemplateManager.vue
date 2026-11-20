<template>
  <div class="templates">
    <div v-for="(template, index) in templates" :key="template.id" class="template">
      <div class="template-head">
        <n-input
          :value="template.name"
          size="small"
          maxlength="40"
          placeholder="模板名称"
          class="name-input"
          @update:value="patch(index, 'name', $event)"
        />
        <div class="vars">
          <span v-for="name in variables(template.content)" :key="name" class="tag">{{ name }}</span>
        </div>
        <n-button quaternary circle size="small" type="error" title="删除模板" @click="remove(index)">
          <template #icon><AppIcon name="trash" :size="16" /></template>
        </n-button>
      </div>
      <n-input
        :value="template.content"
        type="textarea"
        size="small"
        :autosize="{ minRows: 3, maxRows: 8 }"
        placeholder="使用 {{name}} 作为变量占位符"
        @update:value="patch(index, 'content', $event)"
      />
    </div>

    <p v-if="!templates.length" class="hint">
      还没有模板，新建后可在调试台选择；模板中的 &#123;&#123;变量&#125;&#125; 会在发送前要求填写。
    </p>

    <div>
      <n-button size="small" dashed @click="add">
        <template #icon><AppIcon name="plus" :size="16" /></template>
        新建模板
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton, NInput } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import { defaultTemplate, templateVariables } from "../../utils/llm";
import type { LlmTemplate } from "../../types";

const props = defineProps<{ templates: LlmTemplate[] }>();
const emit = defineEmits<{ "update:templates": [templates: LlmTemplate[]] }>();

function commit(next: LlmTemplate[]): void {
  emit("update:templates", next);
}

function patch<K extends keyof LlmTemplate>(index: number, key: K, value: LlmTemplate[K]): void {
  commit(props.templates.map((item, i) => (i === index ? { ...item, [key]: value } : item)));
}

function add(): void {
  commit([...props.templates, defaultTemplate()]);
}

function remove(index: number): void {
  commit(props.templates.filter((_, i) => i !== index));
}

function variables(content: string): string[] {
  return templateVariables(content);
}
</script>

<style scoped>
.templates {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.template {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  background: var(--surface);
}
.template-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.name-input {
  max-width: 220px;
}
.vars {
  display: flex;
  flex: 1;
  gap: 4px;
  flex-wrap: wrap;
}
</style>
