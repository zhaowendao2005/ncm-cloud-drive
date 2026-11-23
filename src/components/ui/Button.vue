<script setup lang="ts">
import { computed } from "vue";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 disabled:pointer-events-none disabled:opacity-50 active:scale-[0.985]",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow-[0_1px_0_0_rgba(255,255,255,0.12)_inset] hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80 border border-white/5",
        outline: "border border-white/10 bg-transparent hover:bg-white/5 text-foreground",
        ghost: "hover:bg-white/5 text-muted-foreground hover:text-foreground",
        destructive: "bg-destructive/90 text-destructive-foreground hover:bg-destructive",
      },
      size: {
        default: "h-9 px-4",
        sm: "h-8 px-3 text-[13px]",
        lg: "h-11 px-6",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  }
);

type Variants = VariantProps<typeof buttonVariants>;
const props = withDefaults(defineProps<{ variant?: Variants["variant"]; size?: Variants["size"]; class?: string }>(), {
  variant: "default",
  size: "default",
});
const classes = computed(() => cn(buttonVariants({ variant: props.variant, size: props.size }), props.class));
</script>

<template>
  <button :class="classes"><slot /></button>
</template>
