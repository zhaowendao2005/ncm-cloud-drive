import { defineStore } from "pinia";
import { ref } from "vue";
import QRCode from "qrcode";
import { api, listen, type AuthStatus } from "@/lib/tauri";

export const useAuthStore = defineStore("auth", () => {
  const loggedIn = ref(false);
  const nickname = ref<string | null>(null);
  const avatar = ref<string | null>(null);
  const deviceId = ref("");
  const hasCredentials = ref(true);
  const savingCredentials = ref(false);
  const credentialsError = ref<string | null>(null);
  const qrDataUrl = ref("");
  const qrUrl = ref("");
  const status = ref(0); // 0 idle / 801 waiting / 802 authed / 803 ok / 800 expired
  const busy = ref(false);
  const error = ref<string | null>(null);

  async function bootstrap() {
    const s = await api.state();
    hasCredentials.value = s.hasCredentials;
    loggedIn.value = s.loggedIn;
    nickname.value = s.nickname;
    deviceId.value = s.deviceId;
    if (s.loggedIn) {
      const p = await api.profile().catch(() => null);
      if (p) {
        nickname.value = p.nickname;
        avatar.value = p.avatarUrl;
      }
    }
  }

  async function startLogin() {
    busy.value = true;
    error.value = null;
    try {
      const { qrUrl: url } = await api.loginStart();
      qrUrl.value = url;
      status.value = 801;
      qrDataUrl.value = await QRCode.toDataURL(url, {
        margin: 1,
        width: 480,
        color: { dark: "#0b0c0f", light: "#ffffff" },
        errorCorrectionLevel: "M",
      });
    } catch (e: any) {
      error.value = String(e);
    } finally {
      busy.value = false;
    }
  }

  async function installListener() {
    await listen<AuthStatus>("auth://status", async (ev) => {
      const s = ev.payload;
      status.value = s.status;
      if (s.status === 803) {
        await bootstrap();
      }
      if (s.status === 800) {
        status.value = 800;
      }
    });
  }

  async function saveCredentials(appId: string, privateKey: string) {
    savingCredentials.value = true;
    credentialsError.value = null;
    try {
      await api.saveCredentials(appId, privateKey);
      hasCredentials.value = true;
      await startLogin();
    } catch (e: any) {
      credentialsError.value = String(e);
    } finally {
      savingCredentials.value = false;
    }
  }

  async function logout() {
    await api.logout();
    loggedIn.value = false;
    nickname.value = null;
    avatar.value = null;
    qrDataUrl.value = "";
    status.value = 0;
  }

  return {
    loggedIn, nickname, avatar, deviceId, hasCredentials, savingCredentials, credentialsError,
    qrDataUrl, qrUrl, status, busy, error,
    bootstrap, startLogin, installListener, saveCredentials, logout,
  };
});
