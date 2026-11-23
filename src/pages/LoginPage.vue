<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { QrCode, RefreshCw, ShieldCheck, ArrowRight, Loader2, KeyRound } from "lucide-vue-next";
import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import Badge from "@/components/ui/Badge.vue";
import { useAuthStore } from "@/stores/auth";
import { useUiStore } from "@/stores/ui";

const auth = useAuthStore();
const ui = useUiStore();

const statusText = computed(() => {
  switch (auth.status) {
    case 801: return "等待扫码";
    case 802: return "已扫码，请在手机上确认";
    case 803: return "登录成功";
    case 800: return "二维码已过期";
    default: return auth.qrDataUrl ? "等待扫码" : "尚未生成二维码";
  }
});
const statusTone = computed(() => (auth.status === 803 ? "success" : auth.status === 800 ? "warn" : "muted"));
const showQr = computed(() => !!auth.qrDataUrl && !auth.loggedIn);

const appId = ref("");
const privateKey = ref("");

onMounted(() => {
  if (auth.hasCredentials && !auth.loggedIn) void auth.startLogin();
});

function submitCredentials() {
  if (!appId.value.trim() || !privateKey.value.trim()) return;
  void auth.saveCredentials(appId.value.trim(), privateKey.value.trim());
}
</script>

<template>
  <div class="flex h-full flex-col overflow-y-auto">
    <header class="drag-region flex h-16 shrink-0 items-center px-8">
      <div class="no-drag text-[13px] text-muted-foreground">账号登录</div>
    </header>

    <div class="flex flex-1 items-center justify-center px-8 pb-16">
      <div class="w-full max-w-[820px]">
        <!-- 已登录 -->
        <Card v-if="auth.loggedIn" class="overflow-hidden">
          <div class="flex items-center gap-5 p-8">
            <img v-if="auth.avatar" :src="auth.avatar" class="h-16 w-16 rounded-2xl object-cover ring-1 ring-white/10" alt="" />
            <div v-else class="grid h-16 w-16 place-items-center rounded-2xl bg-white/10 text-xl font-semibold">{{ auth.nickname?.[0] }}</div>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <h2 class="truncate text-lg font-semibold tracking-tight">{{ auth.nickname }}</h2>
                <Badge variant="success"><ShieldCheck class="mr-1 h-3 w-3" /> 已登录</Badge>
              </div>
              <p class="mt-1 text-[12.5px] text-muted-foreground">
                凭据已加密保存在本机，仅用于调用开放平台接口；设备指纹
                <code class="rounded bg-white/[0.06] px-1 py-0.5 font-mono text-[11px] text-foreground/70">{{ auth.deviceId.slice(0, 12) }}…</code>
              </p>
            </div>
            <Button @click="ui.page = 'workspace'">进入上传工作台 <ArrowRight class="h-4 w-4" /></Button>
          </div>
        </Card>

        <!-- 未登录 -->

        <!-- 首次运行：填写开放平台凭据 -->
        <Card v-else-if="!auth.hasCredentials" class="p-8">
          <div class="flex items-start gap-3">
            <div class="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-primary/12 text-primary">
              <KeyRound class="h-4.5 w-4.5" />
            </div>
            <div>
              <h1 class="text-[19px] font-semibold tracking-tight">先配置你的开放平台凭据</h1>
              <p class="mt-1 max-w-lg text-[12.5px] leading-relaxed text-muted-foreground">
                在网易云音乐开放平台创建应用后可以得到 App ID 与 RSA 私钥。它们只保存在本机
                <code class="mx-1 rounded bg-white/[0.06] px-1 py-0.5 font-mono text-[11px]">~/Library/Application Support/com.qihoo.ncmclouddrive/</code>
                （文件权限 0600），不会被上传。
              </p>
            </div>
          </div>

          <div class="mt-7 grid gap-5">
            <label class="grid gap-2">
              <span class="text-[12px] font-medium text-foreground/80">App ID</span>
              <input
                v-model="appId"
                spellcheck="false"
                placeholder="b3010d0000000000a2dd0a8e41f88035"
                class="h-10 rounded-lg border border-white/10 bg-black/30 px-3 font-mono text-[12.5px] text-foreground placeholder:text-muted-foreground/60 focus:border-primary/60 focus:outline-none focus:ring-2 focus:ring-primary/20"
              />
            </label>
            <label class="grid gap-2">
              <span class="text-[12px] font-medium text-foreground/80">RSA 私钥</span>
              <textarea
                v-model="privateKey"
                rows="6"
                spellcheck="false"
                placeholder="粘贴完整 PEM（-----BEGIN PRIVATE KEY-----…）或裸 base64"
                class="resize-none rounded-lg border border-white/10 bg-black/30 px-3 py-2.5 font-mono text-[11.5px] leading-relaxed text-foreground placeholder:text-muted-foreground/60 focus:border-primary/60 focus:outline-none focus:ring-2 focus:ring-primary/20"
              />
              <span class="text-[11px] text-muted-foreground">私钥会立即做一次解析校验，格式不对会在这里提示。</span>
            </label>
          </div>

          <p v-if="auth.credentialsError" class="mt-4 rounded-lg border border-destructive/25 bg-destructive/10 px-3 py-2 font-mono text-[11.5px] text-red-300">
            {{ auth.credentialsError }}
          </p>

          <div class="mt-7 flex items-center justify-between">
            <span class="flex items-center gap-2 text-[11.5px] text-muted-foreground">
              <ShieldCheck class="h-3.5 w-3.5 text-emerald-500/80" /> 仅本机保存，不经过任何第三方
            </span>
            <Button :disabled="auth.savingCredentials || !appId.trim() || !privateKey.trim()" @click="submitCredentials">
              <Loader2 v-if="auth.savingCredentials" class="h-4 w-4 animate-spin" />
              <KeyRound v-else class="h-4 w-4" />
              保存并生成二维码
            </Button>
          </div>
        </Card>

        <div v-else class="grid gap-6 md:grid-cols-[320px_1fr]">
          <Card class="p-5">
            <div class="relative grid aspect-square place-items-center overflow-hidden rounded-lg bg-white">
              <img v-if="showQr" :src="auth.qrDataUrl" class="h-full w-full object-contain p-3" alt="登录二维码" />
              <div v-else class="flex flex-col items-center gap-2 text-neutral-400">
                <Loader2 v-if="auth.busy" class="h-5 w-5 animate-spin" />
                <QrCode v-else class="h-8 w-8" />
                <span class="text-[11px]">正在生成二维码…</span>
              </div>
              <div v-if="auth.status === 800" class="absolute inset-0 grid place-items-center bg-white/92 backdrop-blur-sm">
                <Button size="sm" variant="secondary" @click="auth.startLogin()"><RefreshCw class="h-3.5 w-3.5" /> 重新生成</Button>
              </div>
            </div>
            <div class="mt-4 flex items-center justify-between">
              <Badge :variant="statusTone as any">{{ auth.busy ? "生成中" : statusText }}</Badge>
              <button class="text-[11.5px] text-muted-foreground hover:text-foreground" @click="auth.startLogin()">刷新二维码</button>
            </div>
          </Card>

          <div class="flex flex-col justify-center">
            <h1 class="text-[26px] font-semibold leading-tight tracking-tight">用网易云音乐 App 扫码登录</h1>
            <p class="mt-2 max-w-md text-[13px] leading-relaxed text-muted-foreground">
              登录后即可把你的音乐文件上传到个人云盘。凭据只保存在本机，不会上传到任何第三方服务器。
            </p>

            <ol class="mt-7 space-y-4">
              <li v-for="(s, i) in ['打开手机上的网易云音乐 App', '点击首页右上角的「扫一扫」', '扫描左侧二维码，在手机上点「确认登录」']" :key="i" class="flex gap-3">
                <span class="mt-0.5 grid h-5 w-5 shrink-0 place-items-center rounded-full border border-white/10 bg-white/[0.04] text-[11px] text-muted-foreground">{{ i + 1 }}</span>
                <span class="text-[13px] leading-relaxed text-foreground/80">{{ s }}</span>
              </li>
            </ol>

            <p v-if="auth.error" class="mt-5 rounded-lg border border-destructive/25 bg-destructive/10 px-3 py-2 text-[12px] text-red-300">{{ auth.error }}</p>

            <div class="mt-8 flex items-center gap-2 text-[11.5px] text-muted-foreground">
              <ShieldCheck class="h-3.5 w-3.5 text-emerald-500/80" />
              凭证经 OS 级文件权限保护（0600），私钥仅存本地
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
