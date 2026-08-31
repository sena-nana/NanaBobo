import { computed, ref } from "vue";

export type PageKey = "home" | "assistant" | "stats" | "history" | "settings";

const current = ref<PageKey>("home");

export function currentPage() {
  return computed(() => current.value);
}

export function navigate(page: PageKey) {
  current.value = page;
}
