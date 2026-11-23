import { defineStore } from "pinia";
import { ref, watch } from "vue";

export const useUiStore = defineStore("ui", () => {
  const collapsed = ref(localStorage.getItem("sidebar-collapsed") === "1");
  const page = ref<"login" | "workspace" | "logs">("login");
  watch(collapsed, (v) => localStorage.setItem("sidebar-collapsed", v ? "1" : "0"));
  return { collapsed, page };
});
