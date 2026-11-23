<script setup lang="ts">
import { onMounted } from "vue";
import Sidebar from "./components/layout/Sidebar.vue";
import LoginPage from "./pages/LoginPage.vue";
import WorkspacePage from "./pages/WorkspacePage.vue";
import LogsPage from "./pages/LogsPage.vue";
import { useUiStore } from "./stores/ui";
import { useAuthStore } from "./stores/auth";
import { useUploadStore } from "./stores/upload";

const ui = useUiStore();
const auth = useAuthStore();
const upload = useUploadStore();

onMounted(async () => {
  await auth.installListener();
  await upload.installListeners();
  await auth.bootstrap();
  await upload.refreshHistory();
  if (auth.loggedIn) ui.page = "workspace";
});
</script>

<template>
  <div class="relative flex h-full w-full overflow-hidden bg-background">
    <div class="pointer-events-none absolute -left-40 -top-40 h-[420px] w-[420px] rounded-full bg-primary/[0.07] blur-[120px]" />
    <div class="pointer-events-none absolute -bottom-40 right-0 h-[380px] w-[380px] rounded-full bg-sky-500/[0.05] blur-[120px]" />
    <Sidebar />
    <main class="relative flex min-w-0 flex-1 flex-col">
      <LoginPage v-if="ui.page === 'login'" />
      <WorkspacePage v-else-if="ui.page === 'workspace'" />
      <LogsPage v-else />
    </main>
  </div>
</template>
