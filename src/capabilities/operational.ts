import { inject, reactive, readonly, type InjectionKey } from "vue";
import type { OperationalState } from "../ui/contract";
import type { TemplateAppCapability } from "../ui/types";

export interface OperationalStore {
  state: Readonly<OperationalState>;
  update: (patch: Partial<OperationalState>) => void;
}

const operationalKey: InjectionKey<OperationalStore> = Symbol("templateOperationalState");

export function createOperationalStore(initial: OperationalState = {}): OperationalStore {
  const state = reactive<OperationalState>({
    resource: "idle",
    connection: "disconnected",
    runtime: "idle",
    output: "inactive",
    save: "clean",
    pendingChanges: false,
    ...initial,
  });
  return {
    state: readonly(state),
    update(patch) { Object.assign(state, patch); },
  };
}

export function operationalCapability(store = createOperationalStore()): TemplateAppCapability {
  return { id: "operational-state", install: (app) => app.provide(operationalKey, store) };
}

export function useOperationalStore(): OperationalStore {
  const store = inject(operationalKey);
  if (!store) throw new Error("Operational state capability is not installed.");
  return store;
}
