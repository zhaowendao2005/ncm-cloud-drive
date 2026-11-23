<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/lib/utils";

const props = withDefaults(defineProps<{ value?: number; class?: string; tone?: "primary" | "success" | "muted" }>(), {
  value: 0,
  tone: "primary",
});
const pct = computed(() => Math.min(100, Math.max(0, props.value)));
const color = computed(
  () => ({ primary: "bg-primary", success: "bg-emerald-500", muted: "bg-white/30" }[props.tone])
);
</script>

<template>
  <div :class="cn('h-1.5 w-full overflow-hidden rounded-full bg-white/[0.06]', $props.class)">
    <div :class="cn('h-full rounded-full transition-[width] duration-300 ease-out', color)" :style="{ width: pct + '%' }" />
  </div>
</template>
