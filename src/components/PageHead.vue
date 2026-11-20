<template>
  <header class="page-head">
    <div>
      <h1>
        <AppIcon v-if="icon" :name="icon" :size="20" />
        {{ title }}
        <span v-if="tag" class="beta-tag">{{ tag }}</span>
      </h1>
      <p v-if="subtitle" class="sub">{{ subtitle }}</p>
    </div>
    <div class="actions"><slot /></div>
  </header>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
import type { IconName } from "../ui/icons";

defineProps<{
  title: string;
  subtitle?: string;
  icon?: IconName;
  /** 实验性功能徽标，如「实验」 */
  tag?: string;
}>();
</script>
