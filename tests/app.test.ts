import { screen, waitFor } from "@testing-library/vue";
import { defineComponent, h } from "vue";
import { RouterView, createMemoryHistory } from "vue-router";
import { afterEach, describe, expect, it } from "vitest";
import { createTemplateApp } from "../src/app";
import type { TemplateUIPresetAdapter } from "../src/ui/preset";

const mounted: Array<() => void> = [];
afterEach(() => { while (mounted.length) mounted.pop()?.(); });

async function mountPreset(preset: TemplateUIPresetAdapter, path = "/") {
  const root = document.createElement("div");
  document.body.append(root);
  const { app, router } = createTemplateApp(createMemoryHistory(), preset);
  await router.push(path);
  await router.isReady();
  app.mount(root);
  mounted.push(() => { app.unmount(); root.remove(); });
  return root;
}

const policy = {
  density: "comfortable",
  advancedDisclosure: "collapsed",
  errorPresentation: "recovery-first",
  selectionPresentation: "filled",
  feedbackStrength: "reinforced",
  sidebarDefault: "expanded",
  destructiveAction: "confirm-or-undo",
} as const;

function mockPreset(id: "lilia" | "nana", label: string): TemplateUIPresetAdapter {
  const Shell = defineComponent({
    setup: () => () => h("main", { "data-agent-id": `mock.${id}.shell` }, h(RouterView)),
  });
  const Page = defineComponent({ setup: () => () => h("h1", label) });
  return {
    id,
    shell: Shell,
    policy,
    defaultDensity: policy.density,
    capabilities: [],
    routes: [{ path: "", component: async () => Page }],
  };
}

describe("application assembly", () => {
  it("mounts the active Nana provider, shell, capabilities, and async settings page", async () => {
    const active = (await import("../src/ui/preset")).activeUIPreset;
    const root = await mountPreset(active, active.id === "nana" ? "/settings?tab=advanced" : "/");
    const shellTarget = active.id === "nana" ? "nana.shell" : "app-shell";
    await waitFor(() => expect(root.querySelector(`[data-agent-id="${shellTarget}"]`)).not.toBeNull());
    if (active.id === "nana") expect(root.querySelector('[data-agent-id="settings.page"]')).not.toBeNull();
    expect(window.__liliaAgentDebug).toBeUndefined();
  });

  it("creates independent routers from injected Lilia and Nana adapters", async () => {
    const liliaRoot = await mountPreset(mockPreset("lilia", "Lilia page"));
    await screen.findByRole("heading", { name: "Lilia page" });
    expect(liliaRoot.querySelector('[data-agent-id="mock.lilia.shell"]')).not.toBeNull();

    const nanaRoot = await mountPreset(mockPreset("nana", "Nana page"));
    await screen.findByRole("heading", { name: "Nana page" });
    expect(nanaRoot.querySelector('[data-agent-id="mock.nana.shell"]')).not.toBeNull();
  });
});
