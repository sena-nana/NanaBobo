import { inject, type InjectionKey } from "vue";
import { createUndoManager } from "../ui/state";
import type { TemplateAppCapability } from "../ui/types";

export type UndoManager = ReturnType<typeof createUndoManager>;
const undoKey: InjectionKey<UndoManager> = Symbol("templateUndoManager");

export function undoCapability(manager = createUndoManager()): TemplateAppCapability {
  return { id: "undo", install: (app) => app.provide(undoKey, manager) };
}

export function useUndoManager(): UndoManager {
  const manager = inject(undoKey);
  if (!manager) throw new Error("Undo capability is not installed.");
  return manager;
}
