<template>
  <div class="empty">
    <AppIcon :name="icon" />
    <b>{{ title }}</b>
    <p v-if="description">{{ description }}</p>
    <slot />
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
import type { IconName } from "../ui/icons";

withDefaults(defineProps<{ title: string; description?: string; icon?: IconName }>(), {
  icon: "inbox",
});
</script>
