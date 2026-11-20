<template>
  <pre class="term code-preview"><span
    v-for="(token, index) in tokens"
    :key="index"
    :class="`tok-${token.kind}`"
  >{{ token.text }}</span></pre>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { highlightCode } from "../../utils/code-highlight";

const props = defineProps<{
  code: string;
  language: "ts" | "python";
}>();

const tokens = computed(() => highlightCode(props.code, props.language));
</script>

<style scoped>
.code-preview {
  margin: 0;
  max-height: 300px;
  overflow: auto;
}

.tok-string {
  color: #7ee2b8;
}

.tok-comment {
  color: #7a849c;
  font-style: italic;
}

.tok-keyword {
  color: #9db1ff;
  font-weight: 600;
}

.tok-number {
  color: #ffd18a;
}
</style>
