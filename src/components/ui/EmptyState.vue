<script setup lang="ts">
defineProps<{ title: string; description?: string }>();
</script>

<template>
  <div class="flex flex-col items-center justify-center gap-2 py-14 text-center">
    <div class="mb-1 grid h-11 w-11 place-items-center rounded-xl border border-white/[0.07] bg-white/[0.02]">
      <slot name="icon" />
    </div>
    <p class="text-sm font-medium text-foreground/80">{{ title }}</p>
    <p v-if="description" class="max-w-sm text-xs leading-relaxed text-muted-foreground">{{ description }}</p>
    <div class="mt-2"><slot /></div>
  </div>
</template>
