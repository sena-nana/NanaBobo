import { fireEvent, screen, waitFor } from "@testing-library/vue";
import { createMemoryHistory } from "vue-router";
import { afterEach, describe, expect, it } from "vitest";
import { createTemplateApp } from "../src/app";
import { activeUIPreset } from "../src/ui/preset";

const mounted: Array<() => void> = [];

async function mountAt(path: string) {
  const root = document.createElement("div");
  document.body.append(root);
  const { app, router } = createTemplateApp(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  app.mount(root);
  mounted.push(() => { app.unmount(); root.remove(); });
  return { root, router };
}

afterEach(() => { while (mounted.length) mounted.pop()?.(); });

describe.runIf(activeUIPreset.id === "nana")("Nana preset routes", () => {
  it("renders the task-oriented home and reachable shell navigation", async () => {
    const { root } = await mountAt("/");
    await screen.findByRole("heading", { level: 1, name: "开始工作" });
    expect(root.querySelector('[data-agent-id="nana.shell"]')).not.toBeNull();
    expect(screen.getByRole("link", { name: "编辑" }).getAttribute("href")).toBe("/editor");
    expect(screen.getByRole("link", { name: "设置" }).getAttribute("href")).toBe("/settings");
    expect(root.querySelector('[data-agent-id="app.operational.status"]')).not.toBeNull();
  });

  it("loads settings sections asynchronously and changes groups through tabs", async () => {
    await mountAt("/settings");
    await screen.findByRole("heading", { level: 1, name: "设置" });
    await screen.findByText("调整界面呈现方式。");
    await fireEvent.click(screen.getByRole("tab", { name: "高级" }));
    await screen.findByText("仅在需要时调整高级行为。");
  });

  it("opens onboarding and supports a real skip path", async () => {
    const { router } = await mountAt("/onboarding");
    await screen.findByRole("heading", { level: 1, name: "选择使用方式" });
    await fireEvent.click(screen.getByRole("button", { name: "稍后继续" }));
    await waitFor(() => expect(router.currentRoute.value.path).toBe("/"));
  });

  it("redirects unknown paths to the async home route", async () => {
    await mountAt("/missing");
    await screen.findByRole("heading", { level: 1, name: "开始工作" });
  });
});
