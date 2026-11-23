<script setup lang="ts">
import { cn } from "@/lib/utils";
defineProps<{ class?: string }>();
</script>

<template>
  <div :class="cn('rounded-xl border border-white/[0.06] bg-card/60 backdrop-blur-sm', $props.class)">
    <slot />
  </div>
</template>
