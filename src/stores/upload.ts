import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, listen, type HistoryItem, type TrackLog, type TrackProgress } from "@/lib/tauri";

export interface QueueItem {
  file: string;
  name: string;
  size: number;
  percent: number;
  speed: number;
  eta: number;
  status: "queued" | "hashing" | "uploading" | "done" | "fast" | "error";
  error?: string;
}

export const useUploadStore = defineStore("upload", () => {
  const queue = ref<QueueItem[]>([]);
  const history = ref<HistoryItem[]>([]);
  const logs = ref<TrackLog[]>([]);
  const running = ref(false);

  const active = computed(() => queue.value.find((q) => q.status === "uploading" || q.status === "hashing"));
  const totalBytes = computed(() => history.value.reduce((a, b) => a + b.fileSize, 0));
  const avgSpeed = computed(() => {
    const done = history.value.filter((h) => h.avgSpeed > 0 && !h.fast);
    if (!done.length) return 0;
    return done.reduce((a, b) => a + b.avgSpeed, 0) / done.length;
  });
  const todayCount = computed(() => {
    const start = new Date().setHours(0, 0, 0, 0);
    return history.value.filter((h) => h.uploadedAt >= start).length;
  });

  function pushLog(level: TrackLog["level"], message: string, scope?: string) {
    logs.value.push({ level, message, time: Date.now(), scope });
    if (logs.value.length > 3000) logs.value.splice(0, logs.value.length - 3000);
  }

  async function refreshHistory() {
    history.value = await api.history();
  }

  async function installListeners() {
    await listen<TrackProgress>("upload://progress", (ev) => {
      const p = ev.payload;
      const idx = queue.value.findIndex((q) => q.file === p.file);
      if (idx >= 0) {
        const item = queue.value[idx];
        item.percent = p.percent;
        item.speed = p.speed;
        item.eta = p.eta;
        item.size = p.totalBytes;
        if (p.loaded > 0) item.status = "uploading";
      }
    });
    await listen<TrackLog>("upload://log", (ev) => {
      const l = ev.payload;
      pushLog(l.level, l.message, l.scope);
      if (/完成|已存在|秒传/.test(l.message)) void refreshHistory();
    });
    await listen<string>("upload://file-done", async (ev) => {
      const idx = queue.value.findIndex((q) => q.file === ev.payload);
      if (idx >= 0) queue.value[idx].status = "done";
      await refreshHistory();
    });
    await listen<string>("upload://file-error", async (ev) => {
      const idx = queue.value.findIndex((q) => q.file === ev.payload);
      if (idx >= 0) queue.value[idx].status = "error";
    });
    await listen<boolean>("upload://running", (ev) => {
      running.value = ev.payload;
      if (!ev.payload) void refreshHistory();
    });
  }

  async function addFiles(files: string[]) {
    for (const f of files) {
      if (queue.value.some((q) => q.file === f && (q.status === "queued" || q.status === "uploading"))) continue;
      queue.value.push({
        file: f,
        name: f.split(/[\\/]/).pop() || f,
        size: 0,
        percent: 0,
        speed: 0,
        eta: 0,
        status: "queued",
      });
    }
    await api.upload(files);
  }

  async function pickFiles() {
    const files = await api.pickFiles();
    if (files.length) await addFiles(files);
  }

  async function pickFolder() {
    const files = await api.pickFolder();
    if (files.length) await addFiles(files);
  }

  function clearFinished() {
    queue.value = queue.value.filter((q) => q.status !== "done" && q.status !== "fast" && q.status !== "error");
  }

  async function clearHistory() {
    await api.clearHistory();
    await refreshHistory();
  }

  return { queue, history, logs, running, active, totalBytes, avgSpeed, todayCount, refreshHistory, installListeners, addFiles, pickFiles, pickFolder, clearFinished, clearHistory, pushLog };
});
