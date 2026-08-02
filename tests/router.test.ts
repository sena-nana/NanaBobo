import { screen } from "@testing-library/vue";
import { createMemoryHistory } from "vue-router";
import { afterEach, describe, expect, it } from "vitest";
import { createNanaBoboApp } from "../src/app";

const mounted: Array<() => void> = [];

async function mountAt(path: string) {
  const root = document.createElement("div");
  document.body.append(root);
  const { app, router } = createNanaBoboApp(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  app.mount(root);
  mounted.push(() => { app.unmount(); root.remove(); });
  return { root, router };
}

afterEach(() => { while (mounted.length) mounted.pop()?.(); });

describe("NanaBobo routes", () => {
  it("renders the real home workflow and reachable settings navigation", async () => {
    const { root } = await mountAt("/");
    await screen.findByRole("heading", { level: 1, name: "Nana播播工具箱" });
    expect(root.querySelector('[data-agent-id="account.panel"]')).not.toBeNull();
    expect(root.querySelector('[data-agent-id="room.panel"]')).not.toBeNull();
    expect(screen.getByRole("link", { name: "设置" }).getAttribute("href")).toBe("/settings");
  });

  it("loads the shared settings route", async () => {
    await mountAt("/settings");
    await screen.findByRole("heading", { level: 1, name: "外观" });
  });

  it("redirects unknown paths to the real home route", async () => {
    await mountAt("/missing");
    await screen.findByRole("heading", { level: 1, name: "Nana播播工具箱" });
  });
});
