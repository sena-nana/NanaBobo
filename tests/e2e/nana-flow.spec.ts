import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
});

test("shows the real account and live-room tools", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "Nana播播工具箱" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "B站账号" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "直播间信息" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "直播间号" })).toBeVisible();
  await expect(page.getByText("请在桌面应用中管理B站账号。")).toBeVisible();
});

test("does not fake a room lookup in browser mode", async ({ page }) => {
  await page.getByRole("textbox", { name: "直播间号" }).fill("abc");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByRole("alert")).toHaveText("请在桌面应用中查询直播间。");
});
