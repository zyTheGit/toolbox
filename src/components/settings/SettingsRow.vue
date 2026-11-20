<template>
  <div class="set-row">
    <div class="set-text">
      <div class="lbl">{{ label }}</div>
      <div v-if="desc" class="desc">{{ desc }}</div>
    </div>
    <div class="ctl"><slot /></div>
  </div>
</template>

<script setup lang="ts">
// 设置页统一的「左说明 + 右控件」行，窄屏自动改为上下排列
defineProps<{ label: string; desc?: string }>();
</script>

<style scoped>
.set-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}

.set-row:last-child {
  border-bottom: 0;
}

.set-text {
  min-width: 0;
}

.lbl {
  font-size: var(--fs-13);
  font-weight: 500;
}

.desc {
  margin-top: 1px;
  font-size: var(--fs-12);
  color: var(--muted);
}

.ctl {
  flex: none;
  display: flex;
  align-items: center;
  gap: 8px;
}

@media (max-width: 760px) {
  .set-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .ctl {
    width: 100%;
    flex-wrap: wrap;
  }
}
</style>
