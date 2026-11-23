// 浏览器预览用的假实现：没有 Tauri 运行时（window.__TAURI_INTERNALS__）时接管 api/事件。
// 仅用于本地看界面，打包后永远不会被选中。
import type { HistoryItem, TrackLog, TrackProgress } from "./tauri";

type Handler = (ev: { payload: any }) => void;
const bus = new Map<string, Set<Handler>>();

function emit(event: string, payload: any) {
  bus.get(event)?.forEach((h) => h({ payload }));
}

const now = Date.now();
const history: HistoryItem[] = [
  { fileName: "久石让 - 人生的旋转木马.mp3", fileSize: 8_388_608, fileMd5: "a1b2c3d4e5f60718293a4b5c6d7e8f90", fileId: 1902117733, taskId: 19622815, fast: false, status: "done", uploadedAt: now - 1000 * 60 * 12, durationMs: 3060, avgSpeed: 2_739_000 },
  { fileName: "YOASOBI - 向夜晚奔去.mp3", fileSize: 11_111_000, fileMd5: "b2c3d4e5f60718293a4b5c6d7e8f90a1", fileId: 1902117702, taskId: 19669639, fast: false, status: "done", uploadedAt: now - 1000 * 60 * 26, durationMs: 4210, avgSpeed: 2_642_000 },
  { fileName: "Lucia - 天狼星的心脏.mp3", fileSize: 11_952_000, fileMd5: "c3d4e5f60718293a4b5c6d7e8f90a1b2", fileId: 1902117688, taskId: 19649726, fast: false, status: "done", uploadedAt: now - 1000 * 60 * 41, durationMs: 5530, avgSpeed: 2_155_000 },
  { fileName: "酔シグレ feat. Lucia - 藍悼花.mp3", fileSize: 9_437_000, fileMd5: "d4e5f60718293a4b5c6d7e8f90a1b2c3", fileId: 1902117651, taskId: 19638733, fast: true, status: "exists", uploadedAt: now - 1000 * 60 * 55, durationMs: 0, avgSpeed: 0 },
  { fileName: "GRISSINI PROJECT - 魔女宅急便 弦乐四重奏.mp3", fileSize: 12_582_000, fileMd5: "e5f60718293a4b5c6d7e8f90a1b2c3d4", fileId: 1902117602, taskId: 19622814, fast: false, status: "done", uploadedAt: now - 1000 * 60 * 68, durationMs: 4830, avgSpeed: 2_603_000 },
  { fileName: "鈴莉れん - 今はいいんだよ。.mp3", fileSize: 5_872_000, fileMd5: "f60718293a4b5c6d7e8f90a1b2c3d4e5", fileId: 1902117588, taskId: 19679202, fast: false, status: "done", uploadedAt: now - 1000 * 60 * 79, durationMs: 2370, avgSpeed: 2_477_000 },
];

const samples = [
  { name: "とあ - 拼湊出的斷音.mp3", size: 10_697_000 },
  { name: "ロクデナシ - ただ声一つ.mp3", size: 6_504_000 },
  { name: "MIMI - 今はいいんだよ。 (Cover).mp3", size: 5_872_000 },
];

function log(level: TrackLog["level"], message: string, scope = "upload") {
  emit("upload://log", { level, message, time: Date.now(), scope });
}

async function simulateUpload(files: string[]) {
  emit("upload://running", true);
  log("info", `队列开始，共 ${files.length} 个文件`);
  for (let i = 0; i < files.length; i++) {
    const file = files[i];
    const name = file.split(/[\\/]/).pop()!;
    const totalBytes = samples[i % samples.length].size;
    log("info", `开始处理 ${name}`);
    log("info", "MD5 计算完成，正在检查秒传");
    const started = Date.now();
    for (let p = 0; p <= 100; p += 4) {
      await new Promise((r) => setTimeout(r, 55));
      const loaded = Math.round((totalBytes * p) / 100);
      const elapsed = (Date.now() - started) / 1000;
      const progress: TrackProgress = {
        kind: "progress",
        file,
        index: i,
        total: files.length,
        loaded,
        totalBytes,
        percent: p,
        speed: 2_400_000 + Math.random() * 600_000,
        avgSpeed: loaded / Math.max(elapsed, 0.3),
        eta: Math.max(0, (totalBytes - loaded) / 2_500_000),
      };
      emit("upload://progress", progress);
    }
    const durationMs = Date.now() - started;
    log("info", `${name} 数据已上传（2.5 MB/s），等待服务端导入 taskId=${19622815 + i}`);
    await new Promise((r) => setTimeout(r, 700));
    history.unshift({
      fileName: name,
      fileSize: totalBytes,
      fileMd5: "0123456789abcdef0123456789abcdef",
      fileId: 1902117800 + i,
      taskId: 19622815 + i,
      fast: false,
      status: "done",
      uploadedAt: Date.now(),
      durationMs,
      avgSpeed: totalBytes / (durationMs / 1000),
    });
    log("success", `${name} 上传完成，fileId=${1902117800 + i}`);
    emit("upload://file-done", file);
  }
  emit("upload://running", false);
}

const mode = typeof location !== "undefined" ? new URLSearchParams(location.search).get("mock") : null;

export const mockApi = {
  state: async () => ({
    hasCredentials: mode !== "setup",
    loggedIn: mode !== "setup" && mode !== "login",
    nickname: "苏暮辄",
    avatar: null as string | null,
    deviceId: "b41ab407d9229e6d57c1d16a605b7186",
  }),
  saveCredentials: async () => {},
  loginStart: async () => ({ qrUrl: "https://163cn.tv/preview", uniKey: "preview" }),
  logout: async () => {},
  profile: async () => ({ nickname: "苏暮辄", avatarUrl: "", userId: 548799224 }),
  history: async () => [...history],
  clearHistory: async () => {
    history.length = 0;
  },
  pickFiles: async () => samples.map((s) => `/Users/qihoo/Music/${s.name}`),
  pickFolder: async () => samples.slice(0, 2).map((s) => `/Users/qihoo/Music/Album/${s.name}`),
  upload: async (files: string[]) => {
    void simulateUpload(files);
  },
  cancelAll: async () => {},
};

export function mockListen<T>(event: string, handler: (ev: { payload: T }) => void) {
  const set = bus.get(event) ?? new Set<Handler>();
  set.add(handler as Handler);
  bus.set(event, set);
  return Promise.resolve(() => set.delete(handler as Handler));
}
