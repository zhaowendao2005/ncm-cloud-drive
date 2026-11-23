<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { FolderPlus, FilePlus2, Zap, Trash2, Music4, Gauge, HardDrive, CheckCheck, Loader2 } from "lucide-vue-next";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import Badge from "@/components/ui/Badge.vue";
import Progress from "@/components/ui/Progress.vue";
import EmptyState from "@/components/ui/EmptyState.vue";
import { useUploadStore } from "@/stores/upload";
import { formatBytes, formatDuration, formatSpeed, formatTime } from "@/lib/utils";

const upload = useUploadStore();
const dragging = ref(false);
let unlisten: (() => void) | null = null;

const stats = computed(() => [
  { label: "今日上传", value: String(upload.todayCount), sub: "个文件", icon: CheckCheck },
  { label: "累计体积", value: formatBytes(upload.totalBytes), sub: `${upload.history.length} 个文件`, icon: HardDrive },
  { label: "平均速度", value: formatSpeed(upload.avgSpeed), sub: "实测稳态", icon: Gauge },
]);

const statusBadge = (h: { fast: boolean; status: string }) =>
  h.fast ? { text: "秒传", variant: "warn" as const } : { text: h.status === "done" ? "已上传" : h.status, variant: "success" as const };

onMounted(async () => {
  await upload.refreshHistory();
  unlisten = await getCurrentWebview().onDragDropEvent((ev) => {
    if (ev.payload.type === "enter" || ev.payload.type === "over") dragging.value = true;
    else if (ev.payload.type === "drop") {
      dragging.value = false;
      if (ev.payload.paths?.length) void upload.addFiles(ev.payload.paths);
    } else dragging.value = false;
  });
});
onUnmounted(() => unlisten?.());
</script>

<template>
  <div class="relative flex h-full flex-col overflow-hidden">
    <header class="drag-region flex h-16 shrink-0 items-center justify-between px-8">
      <div>
        <h1 class="text-[15px] font-semibold tracking-tight">上传工作台</h1>
        <p class="text-[11.5px] text-muted-foreground">选择本地音乐，上传到你的网易云云盘</p>
      </div>
      <div class="no-drag flex items-center gap-2">
        <Button variant="outline" size="sm" :disabled="upload.running" @click="upload.pickFolder()">
          <FolderPlus class="h-3.5 w-3.5" /> 选择文件夹
        </Button>
        <Button size="sm" :disabled="upload.running" @click="upload.pickFiles()">
          <FilePlus2 class="h-3.5 w-3.5" /> 选择文件
        </Button>
      </div>
    </header>

    <div class="flex-1 overflow-y-auto px-8 pb-10">
      <!-- stats -->
      <div class="grid grid-cols-3 gap-3">
        <Card v-for="s in stats" :key="s.label" class="p-4">
          <div class="flex items-center justify-between">
            <span class="text-[11.5px] text-muted-foreground">{{ s.label }}</span>
            <component :is="s.icon" class="h-3.5 w-3.5 text-muted-foreground/70" :stroke-width="1.8" />
          </div>
          <p class="num mt-2 text-[22px] font-semibold leading-none tracking-tight">{{ s.value }}</p>
          <p class="mt-1.5 text-[11px] text-muted-foreground">{{ s.sub }}</p>
        </Card>
      </div>

      <!-- queue -->
      <section v-if="upload.queue.length" class="mt-7">
        <div class="mb-2.5 flex items-center justify-between">
          <h2 class="text-[12.5px] font-medium text-foreground/80">当前队列</h2>
          <button class="text-[11.5px] text-muted-foreground hover:text-foreground" @click="upload.clearFinished()">清除已完成</button>
        </div>
        <Card class="divide-y divide-white/[0.05] overflow-hidden">
          <div v-for="q in upload.queue" :key="q.file" class="px-4 py-3">
            <div class="flex items-center gap-3">
              <div class="grid h-8 w-8 shrink-0 place-items-center rounded-md bg-white/[0.04]">
                <Loader2 v-if="q.status === 'uploading' || q.status === 'hashing'" class="h-3.5 w-3.5 animate-spin text-primary" />
                <Music4 v-else class="h-3.5 w-3.5 text-muted-foreground" />
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <p class="truncate text-[12.5px]">{{ q.name }}</p>
                  <Badge :variant="q.status === 'done' ? 'success' : q.status === 'error' ? 'danger' : q.status === 'fast' ? 'warn' : 'muted'">
                    {{ q.status === "uploading" ? "上传中" : q.status === "hashing" ? "校验中" : q.status === "queued" ? "排队中" : q.status === "done" ? "完成" : q.status === "fast" ? "秒传" : "失败" }}
                  </Badge>
                </div>
                <p class="num mt-1 text-[11px] text-muted-foreground">
                  {{ formatBytes(q.size) }} · {{ formatSpeed(q.speed) }} <span v-if="q.eta > 0">· 剩余 {{ formatDuration(q.eta) }}</span>
                </p>
              </div>
              <span class="num w-11 text-right text-[12px] text-muted-foreground">{{ q.percent.toFixed(0) }}%</span>
            </div>
            <Progress class="mt-2.5" :value="q.percent" :tone="q.status === 'error' ? 'muted' : q.status === 'done' ? 'success' : 'primary'" />
          </div>
        </Card>
      </section>

      <!-- history -->
      <section class="mt-7">
        <div class="mb-2.5 flex items-center justify-between">
          <h2 class="text-[12.5px] font-medium text-foreground/80">上传记录</h2>
          <button v-if="upload.history.length" class="flex items-center gap-1 text-[11.5px] text-muted-foreground hover:text-red-300" @click="upload.clearHistory()">
            <Trash2 class="h-3 w-3" /> 清空记录
          </button>
        </div>

        <Card v-if="upload.history.length" class="overflow-hidden">
          <div class="grid grid-cols-[1fr_88px_96px_112px_76px] gap-3 border-b border-white/[0.05] px-4 py-2 text-[10.5px] uppercase tracking-wider text-muted-foreground/70">
            <span>文件</span><span class="text-right">大小</span><span class="text-right">平均速度</span><span class="text-right">时间</span><span class="text-right">状态</span>
          </div>
          <div v-for="h in upload.history" :key="h.fileMd5 + h.uploadedAt" class="grid grid-cols-[1fr_88px_96px_112px_76px] items-center gap-3 px-4 py-2.5 text-[12px] transition-colors hover:bg-white/[0.02]">
            <div class="flex min-w-0 items-center gap-2">
              <Music4 class="h-3.5 w-3.5 shrink-0 text-muted-foreground/70" />
              <span class="truncate">{{ h.fileName }}</span>
            </div>
            <span class="num text-right text-muted-foreground">{{ formatBytes(h.fileSize) }}</span>
            <span class="num text-right text-muted-foreground">{{ h.fast ? "—" : formatSpeed(h.avgSpeed) }}</span>
            <span class="num text-right text-muted-foreground">{{ formatTime(h.uploadedAt) }}</span>
            <span class="text-right"><Badge :variant="statusBadge(h).variant">{{ statusBadge(h).text }}</Badge></span>
          </div>
        </Card>

        <Card v-else class="border-dashed border-white/[0.08] bg-transparent">
          <EmptyState title="还没有上传记录" description="把音频文件拖进窗口，或点击右上角「选择文件」。支持 mp3 / flac / m4a / wav 等格式。">
            <template #icon><Zap class="h-4 w-4 text-primary/70" /></template>
            <Button size="sm" variant="secondary" @click="upload.pickFiles()"><FilePlus2 class="h-3.5 w-3.5" /> 选择文件</Button>
          </EmptyState>
        </Card>
      </section>
    </div>

    <!-- drop overlay -->
    <transition
      enter-active-class="transition duration-150 ease-out" enter-from-class="opacity-0" leave-active-class="transition duration-150 ease-in" leave-to-class="opacity-0"
    >
      <div v-if="dragging" class="pointer-events-none absolute inset-3 z-20 grid place-items-center rounded-2xl border-2 border-dashed border-primary/50 bg-background/80 backdrop-blur-sm">
        <div class="flex flex-col items-center gap-2">
          <FilePlus2 class="h-7 w-7 text-primary" />
          <p class="text-[13px] font-medium">松手即可加入上传队列</p>
        </div>
      </div>
    </transition>
  </div>
</template>
