import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
});

test("uses Nana policy, task home, and icon-only primary navigation", async ({ page }) => {
  await page.reload();
  await expect(page.locator(".nana-ui[data-density='comfortable']")).toBeVisible();
  await expect(page.getByRole("heading", { name: "开始工作" })).toBeVisible();
  await page.getByRole("button", { name: "收起侧边栏" }).click();
  await expect(page.locator(".nana-sidebar--icon")).toBeVisible();
  await expect(page.locator(".nana-sidebar__nav .nana-sidebar__item")).toHaveCount(2);
  await expect(page.locator(".nana-sidebar__footer [href='/settings']")).toBeVisible();
});

test("shows editor context only for a selection and preserves save truth", async ({ page }) => {
  await page.getByRole("button", { name: "快速开始" }).click();
  await expect(page.getByRole("heading", { name: "实时预览" })).toBeVisible();
  await expect(page.locator('[data-agent-id="editor.context"]')).toBeVisible();
  await page.getByRole("button", { name: "取消选择" }).click();
  await expect(page.locator('[data-agent-id="editor.context"]')).toHaveCount(0);
  await page.getByRole("button", { name: "主场景" }).click();
  await page.getByRole("textbox", { name: "项目名称" }).fill("");
  await expect(page.getByText("操作未完成")).toBeVisible();
  await expect(page.getByText("Project name is required.")).toHaveCount(0);
  await page.getByRole("textbox", { name: "项目名称" }).fill("恢复后的项目");
  await expect(page.getByText("已保存").first()).toBeVisible();
});

test("keeps advanced settings collapsed and applies density without changing routes", async ({ page }) => {
  await page.goto("/settings");
  await page.getByRole("tab", { name: "高级" }).click();
  await expect(page.getByText("使用高级选项")).toHaveCount(0);
  await page.getByRole("button", { name: "高级设置" }).click();
  await expect(page.getByText("使用高级选项")).toBeVisible();
  await page.getByRole("tab", { name: "外观" }).click();
  await page.getByRole("switch", { name: "紧凑密度" }).click();
  await expect(page.locator(".nana-ui[data-density='compact']")).toBeVisible();
  await expect(page).toHaveURL(/tab=appearance/);
});

test("recovers onboarding failure, resumes later, and reduces completion motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/onboarding");
  await page.getByRole("button", { name: "继续", exact: true }).click();
  await expect(page.getByRole("heading", { name: "检查设备", level: 1 })).toBeVisible();
  await page.getByRole("button", { name: "继续", exact: true }).click();
  await expect(page.getByText("操作未完成")).toBeVisible();
  await page.getByRole("button", { name: "重试" }).click();
  await expect(page.getByRole("heading", { name: "选择资源", level: 1 })).toBeVisible();
  const completion = page.locator('[data-agent-id="onboarding.completion"]');
  await expect(completion).toBeVisible();
  expect(await completion.evaluate((node) => getComputedStyle(node).animationName)).toBe("none");
  await page.getByRole("button", { name: "稍后继续" }).click();
  await page.goto("/onboarding");
  await expect(page.getByRole("heading", { name: "选择资源", level: 1 })).toBeVisible();
});
