<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { ArrowDownToLine, Trash2, Search } from "lucide-vue-next";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import EmptyState from "@/components/ui/EmptyState.vue";
import { useUploadStore } from "@/stores/upload";

const upload = useUploadStore();
const filter = ref<"all" | "info" | "warn" | "error" | "success">("all");
const query = ref("");
const autoScroll = ref(true);
const box = ref<HTMLElement | null>(null);

const levels = [
  { key: "all" as const, label: "全部" },
  { key: "info" as const, label: "信息" },
  { key: "success" as const, label: "成功" },
  { key: "warn" as const, label: "警告" },
  { key: "error" as const, label: "错误" },
];

const shown = computed(() =>
  upload.logs.filter(
    (l) => (filter.value === "all" || l.level === filter.value) && (!query.value || l.message.toLowerCase().includes(query.value.toLowerCase()))
  )
);

const color = (lvl: string) => ({ info: "text-sky-400/80", success: "text-emerald-400", warn: "text-amber-300", error: "text-red-400" }[lvl] || "text-muted-foreground");
const dot = (lvl: string) => ({ info: "bg-sky-400", success: "bg-emerald-400", warn: "bg-amber-300", error: "bg-red-400" }[lvl] || "bg-white/30");
const ts = (t: number) => new Date(t).toTimeString().slice(0, 8);

async function scrollToEnd() {
  await nextTick();
  if (autoScroll.value && box.value) box.value.scrollTop = box.value.scrollHeight;
}

watch(() => upload.logs.length, scrollToEnd);
onMounted(scrollToEnd);
onUnmounted(() => {});

function detectScroll() {
  if (!box.value) return;
  const nearBottom = box.value.scrollHeight - box.value.scrollTop - box.value.clientHeight < 48;
  if (!nearBottom) autoScroll.value = false;
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <header class="drag-region flex h-16 shrink-0 items-center justify-between px-8">
      <div>
        <h1 class="text-[15px] font-semibold tracking-tight">运行日志</h1>
        <p class="text-[11.5px] text-muted-foreground">实时上传进度与接口调用记录</p>
      </div>
      <div class="no-drag flex items-center gap-2">
        <button
          :class="['flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-[11.5px] transition-colors', autoScroll ? 'border-primary/30 bg-primary/10 text-primary' : 'border-white/10 text-muted-foreground hover:text-foreground']"
          @click="autoScroll = !autoScroll"
        >
          <ArrowDownToLine class="h-3.5 w-3.5" /> 自动滚动
        </button>
        <Button variant="ghost" size="sm" @click="upload.logs = []"><Trash2 class="h-3.5 w-3.5" /> 清空</Button>
      </div>
    </header>

    <div class="flex shrink-0 items-center gap-3 px-8 pb-3">
      <div class="flex items-center gap-1 rounded-lg border border-white/[0.07] bg-white/[0.02] p-0.5">
        <button
          v-for="l in levels"
          :key="l.key"
          :class="['rounded-md px-2.5 py-1 text-[11.5px] transition-colors', filter === l.key ? 'bg-white/[0.08] text-foreground' : 'text-muted-foreground hover:text-foreground/80']"
          @click="filter = l.key"
        >{{ l.label }}</button>
      </div>
      <div class="relative ml-auto w-56">
        <Search class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground/60" />
        <input v-model="query" placeholder="搜索日志…" class="h-8 w-full rounded-lg border border-white/[0.07] bg-white/[0.02] pl-8 pr-2 text-[12px] outline-none placeholder:text-muted-foreground/50 focus:border-white/15" />
      </div>
    </div>

    <div class="flex-1 overflow-hidden px-8 pb-8">
      <Card class="flex h-full flex-col overflow-hidden bg-black/40">
        <div ref="box" class="flex-1 overflow-y-auto px-4 py-3 font-mono text-[11.5px] leading-[1.75]" @scroll="detectScroll">
          <EmptyState v-if="!shown.length" title="暂无日志" description="开始上传后，这里会实时输出每一块的进度与接口响应。" />
          <div v-for="(l, i) in shown" :key="i" class="flex items-start gap-2.5 py-[1px] hover:bg-white/[0.02]">
            <span class="num shrink-0 text-muted-foreground/45">{{ ts(l.time) }}</span>
            <span :class="['mt-[7px] h-1.5 w-1.5 shrink-0 rounded-full', dot(l.level)]" />
            <span v-if="l.scope" class="shrink-0 text-muted-foreground/60">[{{ l.scope }}]</span>
            <span :class="['min-w-0 break-all', color(l.level)]">{{ l.message }}</span>
          </div>
        </div>
      </Card>
    </div>
  </div>
</template>
