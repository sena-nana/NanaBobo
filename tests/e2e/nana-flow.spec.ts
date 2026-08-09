import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
});

test("shows the real account and live-room tools", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "登录后连接直播间" })).toBeVisible();
  await expect(page.getByText("请在桌面应用中扫码登录B站账号。")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "直播间号" })).toHaveCount(0);

  const roomCard = page.locator('[data-agent-id="home.room"] [data-agent-id="account.panel"]');
  const trendCard = page.locator('[data-agent-id="home.trend-card"]');
  await expect(roomCard).toBeVisible();
  await expect(trendCard).toBeVisible();
  await expect(page.locator('[data-agent-id="home.summary"]')).toHaveCount(0);
  await page.evaluate(async () => { await document.fonts.ready; });

  const [roomLayout, trendLayout] = await Promise.all(
    [roomCard, trendCard].map((card) => card.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return {
        backgroundColor: style.backgroundColor,
        borderColor: style.borderTopColor,
        borderStyle: style.borderTopStyle,
        borderWidth: Number.parseFloat(style.borderTopWidth),
        rect: { height: rect.height, right: rect.right, x: rect.x, y: rect.y },
      };
    })),
  );

  expect(Math.abs(roomLayout.rect.height - trendLayout.rect.height)).toBeLessThanOrEqual(1);
  expect(Math.abs(roomLayout.rect.y - trendLayout.rect.y)).toBeLessThanOrEqual(1);
  expect(roomLayout.rect.right).toBeLessThanOrEqual(trendLayout.rect.x);
  const transparentColors = ["transparent", "rgba(0, 0, 0, 0)"];
  for (const layout of [roomLayout, trendLayout]) {
    expect(transparentColors).not.toContain(layout.backgroundColor);
    expect(layout.borderStyle).toBe("solid");
    expect(layout.borderWidth).toBeGreaterThan(0);
    expect(transparentColors).not.toContain(layout.borderColor);
  }
});

test("does not fake a room lookup in browser mode", async ({ page }) => {
  await expect(page.getByRole("textbox", { name: "直播间号" })).toHaveCount(0);
  await expect(page.locator('[data-agent-id="room.panel"]')).toHaveCount(0);
});
