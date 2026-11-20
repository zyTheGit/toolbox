<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title"><AppIcon name="info" :size="16" />关于</h3>
    </div>

    <div class="about">
      <div class="kv"><b>版本</b><span>v{{ version }} · {{ platform }}</span></div>
      <div class="kv"><b>数据目录</b><span class="mono">{{ dataDir || "…" }}</span></div>
    </div>
  </section>
</template>

<script setup lang="ts">
import AppIcon from "../AppIcon.vue";

defineProps<{
  version: string;
  platform: string;
  dataDir: string;
}>();
</script>

<style scoped>
.about {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.kv b {
  min-width: 56px;
}
</style>
