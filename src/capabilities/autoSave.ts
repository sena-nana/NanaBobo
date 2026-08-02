import { inject, onBeforeUnmount, onMounted, watch, type InjectionKey } from "vue";
import { onBeforeRouteLeave } from "vue-router";
import { useAutoSave, type AutoSaveController } from "../ui/state";
import type { TemplateAppCapability } from "../ui/types";
import { useOperationalStore } from "./operational";

export interface AutoSaveDefaults { delayMs: number }
const autoSaveKey: InjectionKey<AutoSaveDefaults> = Symbol("templateAutoSaveDefaults");

export function autoSaveCapability(defaults: AutoSaveDefaults = { delayMs: 600 }): TemplateAppCapability {
  return { id: "auto-save", install: (app) => app.provide(autoSaveKey, defaults) };
}

export function useTemplateAutoSave<T>(options: {
  serialize: () => T;
  save: (snapshot: T) => Promise<void>;
}): AutoSaveController {
  const defaults = inject(autoSaveKey, { delayMs: 600 });
  const operational = useOperationalStore();
  const controller = useAutoSave({ ...options, delayMs: defaults.delayMs });
  const stop = watch(controller.state, (save) => operational.update({
    save,
    pendingChanges: save === "dirty" || save === "saving" || save === "failed",
  }), { immediate: true });

  const beforeUnload = (event: BeforeUnloadEvent) => {
    if (!operational.state.pendingChanges) return;
    event.preventDefault();
    event.returnValue = "";
  };
  onMounted(() => window.addEventListener("beforeunload", beforeUnload));
  onBeforeUnmount(() => { stop(); window.removeEventListener("beforeunload", beforeUnload); });
  onBeforeRouteLeave(async () => operational.state.pendingChanges ? await controller.flush() : true);
  return controller;
}
