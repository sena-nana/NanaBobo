import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
});

test("shows the real account and live-room tools", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "登录后连接直播间" })).toBeVisible();
  await expect(page.getByText("请在桌面应用中扫码登录B站账号。")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "直播间号" })).toHaveCount(0);
});

test("does not fake a room lookup in browser mode", async ({ page }) => {
  await expect(page.getByRole("textbox", { name: "直播间号" })).toHaveCount(0);
  await expect(page.locator('[data-agent-id="room.panel"]')).toHaveCount(0);
});
