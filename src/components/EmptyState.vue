<template>
  <div class="empty">
    <div v-if="$slots.icon" class="empty-icon"><slot name="icon" /></div>
    <div class="empty-title">{{ title }}</div>
    <div v-if="description" class="empty-desc">{{ description }}</div>
    <div v-if="$slots.default" class="empty-actions"><slot /></div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ title: string; description?: string }>();
</script>

<style scoped>
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 44px 20px;
  color: var(--muted);
}

.empty-icon {
  margin-bottom: 12px;
  color: #b9c0cd;
}

.empty-title {
  font-size: 14px;
  font-weight: 600;
  color: #3c4456;
}

.empty-desc {
  margin-top: 6px;
  font-size: 12px;
  max-width: 420px;
  line-height: 1.7;
}

.empty-actions {
  margin-top: 16px;
}
</style>
