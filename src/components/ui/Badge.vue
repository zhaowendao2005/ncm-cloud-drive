<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/lib/utils";
const props = withDefaults(defineProps<{ variant?: "default" | "success" | "warn" | "muted" | "danger"; class?: string }>(), {
  variant: "default",
});
const cls = computed(
  () =>
    ({
      default: "bg-primary/12 text-primary border-primary/20",
      success: "bg-emerald-500/12 text-emerald-400 border-emerald-500/20",
      warn: "bg-amber-400/12 text-amber-300 border-amber-400/20",
      muted: "bg-white/5 text-muted-foreground border-white/10",
      danger: "bg-destructive/15 text-red-400 border-destructive/25",
    }[props.variant])
);
</script>

<template>
  <span :class="cn('inline-flex items-center rounded-md border px-1.5 py-0.5 text-[11px] font-medium leading-4', cls, $props.class)">
    <slot />
  </span>
</template>
