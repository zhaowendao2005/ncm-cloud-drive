import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import { mockApi, mockListen } from "./mock";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const listen: typeof tauriListen = isTauri ? tauriListen : (mockListen as unknown as typeof tauriListen);

export { invoke };
export type Unlisten = UnlistenFn;

export interface TrackProgress {
  kind: "progress";
  file: string;
  index: number;
  total: number;
  loaded: number;
  totalBytes: number;
  percent: number;
  speed: number;
  avgSpeed: number;
  eta: number;
}

export interface TrackLog {
  level: "info" | "warn" | "error" | "success";
  message: string;
  time: number;
  scope?: string;
}

export interface AuthStatus {
  status: number; // 801 待扫 802 授权中 803 成功 800 过期
  message?: string;
  qrUrl?: string;
}

export interface HistoryItem {
  fileName: string;
  fileSize: number;
  fileMd5: string;
  fileId: number;
  taskId: number;
  fast: boolean;
  status: string;
  uploadedAt: number;
  durationMs: number;
  avgSpeed: number;
}

const realApi = {
  state: () =>
    invoke<{ hasCredentials: boolean; loggedIn: boolean; nickname: string | null; avatar: string | null; deviceId: string }>("get_state"),
  saveCredentials: (appId: string, privateKey: string) => invoke<void>("save_credentials_cmd", { appId, privateKey }),
  loginStart: () => invoke<{ qrUrl: string; uniKey: string }>("login_start"),
  logout: () => invoke<void>("logout"),
  profile: () => invoke<{ nickname: string; avatarUrl: string; userId: number } | null>("get_profile"),
  history: () => invoke<HistoryItem[]>("get_history"),
  clearHistory: () => invoke<void>("clear_history"),
  pickFiles: () => invoke<string[]>("pick_files"),
  pickFolder: () => invoke<string[]>("pick_folder"),
  upload: (files: string[]) => invoke<void>("upload_files", { files }),
  cancelAll: () => invoke<void>("cancel_uploads"),
};

export const api = isTauri ? realApi : (mockApi as unknown as typeof realApi);
