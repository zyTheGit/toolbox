<template>
  <svg class="ic" :class="sizeClass" viewBox="0 0 24 24" aria-hidden="true">
    <use :href="`#i-${name}`" />
  </svg>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { IconName } from "../ui/icons";

const props = defineProps<{ name: IconName; size?: 16 | 18 | 20 }>();

const sizeClass = computed(() => (props.size && props.size !== 18 ? `ic-${props.size}` : ""));
</script>
