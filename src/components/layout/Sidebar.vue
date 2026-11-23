<script setup lang="ts">
import { computed } from "vue";
import { CloudUpload, ScrollText, LogIn, PanelLeftClose, PanelLeftOpen, LogOut } from "lucide-vue-next";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/stores/ui";
import { useAuthStore } from "@/stores/auth";

const ui = useUiStore();
const auth = useAuthStore();

const items = computed(() => [
  { key: "login" as const, label: "账号登录", icon: LogIn },
  { key: "workspace" as const, label: "上传工作台", icon: CloudUpload },
  { key: "logs" as const, label: "运行日志", icon: ScrollText },
]);
</script>

<template>
  <aside
    :class="cn('relative flex h-full flex-col border-r border-white/[0.06] bg-black/20 transition-[width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]', ui.collapsed ? 'w-[72px]' : 'w-[236px]')"
  >
    <!-- brand -->
    <div class="flex h-16 items-center gap-2.5 px-4">
      <div class="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-primary shadow-[0_0_20px_-4px] shadow-primary/60">
        <CloudUpload class="h-[18px] w-[18px] text-white" :stroke-width="2.1" />
      </div>
      <div v-if="!ui.collapsed" class="min-w-0 overflow-hidden">
        <p class="truncate text-[13px] font-semibold tracking-tight">云盘上传器</p>
        <p class="truncate text-[11px] text-muted-foreground">网易云 · 开放平台</p>
      </div>
    </div>

    <nav class="mt-2 flex flex-1 flex-col gap-1 px-3">
      <button
        v-for="it in items"
        :key="it.key"
        :class="
          cn(
            'group flex h-10 items-center gap-3 rounded-lg px-2.5 text-[13px] transition-colors',
            ui.page === it.key ? 'bg-white/[0.06] text-foreground' : 'text-muted-foreground hover:bg-white/[0.03] hover:text-foreground/80',
            ui.collapsed && 'justify-center px-0'
          )
        "
        :title="ui.collapsed ? it.label : undefined"
        @click="ui.page = it.key"
      >
        <component :is="it.icon" class="h-[17px] w-[17px] shrink-0" :stroke-width="ui.page === it.key ? 2.2 : 1.8" />
        <span v-if="!ui.collapsed" class="truncate">{{ it.label }}</span>
        <span v-if="!ui.collapsed && ui.page === it.key" class="ml-auto h-1.5 w-1.5 rounded-full bg-primary" />
      </button>
    </nav>

    <!-- account -->
    <div class="border-t border-white/[0.06] p-3">
      <div v-if="!ui.collapsed && auth.loggedIn" class="mb-2 flex items-center gap-2.5 rounded-lg px-1.5 py-1.5">
        <img v-if="auth.avatar" :src="auth.avatar" class="h-7 w-7 rounded-full object-cover ring-1 ring-white/10" alt="" />
        <div v-else class="grid h-7 w-7 place-items-center rounded-full bg-white/10 text-[11px] font-semibold">
          {{ auth.nickname?.[0] ?? "?" }}
        </div>
        <div class="min-w-0">
          <p class="truncate text-[12px] font-medium">{{ auth.nickname }}</p>
          <p class="truncate text-[10.5px] text-muted-foreground">已登录</p>
        </div>
        <button class="ml-auto rounded p-1 text-muted-foreground hover:bg-white/5 hover:text-foreground" title="退出登录" @click="auth.logout()">
          <LogOut class="h-3.5 w-3.5" />
        </button>
      </div>

      <button
        :class="cn('flex h-9 w-full items-center gap-2 rounded-lg px-2.5 text-[12px] text-muted-foreground transition-colors hover:bg-white/[0.04] hover:text-foreground', ui.collapsed && 'justify-center px-0')"
        @click="ui.collapsed = !ui.collapsed"
      >
        <component :is="ui.collapsed ? PanelLeftOpen : PanelLeftClose" class="h-4 w-4 shrink-0" :stroke-width="1.8" />
        <span v-if="!ui.collapsed">收起侧栏</span>
      </button>
    </div>
  </aside>
</template>
