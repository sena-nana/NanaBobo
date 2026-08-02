import { createApp, defineComponent, h, nextTick } from "vue";
import { RouterView, createMemoryHistory, createRouter } from "vue-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { autoSaveCapability, useTemplateAutoSave } from "../src/capabilities/autoSave";
import { createOperationalStore, operationalCapability } from "../src/capabilities/operational";
import { mapAppError } from "../src/capabilities/recovery";
import { createUndoManager } from "../src/ui/state";

const cleanups: Array<() => void> = [];
afterEach(() => { while (cleanups.length) cleanups.pop()?.(); });

async function mountAutoSave(save: (value: string) => Promise<void>) {
  const Probe = defineComponent({
    setup() {
      let value = "initial";
      const controller = useTemplateAutoSave({ serialize: () => value, save });
      return () => h("section", [
        h("output", { "data-testid": "state" }, controller.state.value),
        h("button", { onClick: () => { value = "changed"; controller.markDirty(); } }, "Edit"),
        h("button", { onClick: () => { void controller.retry(); } }, "Retry"),
      ]);
    },
  });
  const Done = defineComponent({ setup: () => () => h("p", "Done") });
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/edit", component: Probe }, { path: "/done", component: Done }],
  });
  const root = document.createElement("div");
  document.body.append(root);
  const app = createApp(defineComponent({ setup: () => () => h(RouterView) }));
  const operational = createOperationalStore();
  operationalCapability(operational).install(app);
  autoSaveCapability({ delayMs: 60_000 }).install(app);
  app.use(router);
  await router.push("/edit");
  await router.isReady();
  app.mount(root);
  cleanups.push(() => { app.unmount(); root.remove(); });
  return { operational, root, router };
}

describe("Nana application capabilities", () => {
  it("flushes dirty edits before route navigation and keeps state truthful", async () => {
    const save = vi.fn(async () => undefined);
    const { operational, root, router } = await mountAutoSave(save);
    root.querySelector<HTMLButtonElement>("button")?.click();
    await nextTick();
    expect(operational.state.save).toBe("dirty");

    await router.push("/done");

    expect(save).toHaveBeenCalledWith("changed");
    expect(operational.state.save).toBe("saved");
    expect(operational.state.pendingChanges).toBe(false);
    expect(router.currentRoute.value.path).toBe("/done");
  });

  it("keeps failed saves pending and allows a real retry", async () => {
    const save = vi.fn()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(undefined);
    const { operational, root, router } = await mountAutoSave(save);
    root.querySelector<HTMLButtonElement>("button")?.click();
    await nextTick();

    await router.push("/done");
    expect(router.currentRoute.value.path).toBe("/edit");
    expect(operational.state.save).toBe("failed");
    expect(operational.state.pendingChanges).toBe(true);

    root.querySelectorAll<HTMLButtonElement>("button")[1]?.click();
    await vi.waitFor(() => expect(operational.state.save).toBe("saved"));
    expect(save).toHaveBeenCalledTimes(2);
  });

  it("maps recovery details and preserves undo/redo behavior", async () => {
    const retry = vi.fn();
    const recovery = mapAppError(new Error("network closed"), retry);
    expect(recovery.actions[0]?.run).toBe(retry);
    expect(recovery.technicalDetails).toBe("network closed");

    let value = 0;
    const undo = createUndoManager();
    await undo.execute({ id: "increment", execute: () => { value += 1; }, undo: () => { value -= 1; } });
    expect(value).toBe(1);
    await undo.undo();
    expect(value).toBe(0);
    await undo.redo();
    expect(value).toBe(1);
  });
});
