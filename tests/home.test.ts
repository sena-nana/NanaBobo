import { fireEvent, render, screen, waitFor } from "@testing-library/vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { defineComponent, h } from "vue";
import type { AccountStatus, RoomInfo } from "../src/contracts/bilibili";

const mocks = vi.hoisted(() => ({
  isTauriRuntime: vi.fn(() => true),
  api: {
    startQr: vi.fn(),
    pollQr: vi.fn(),
    getStatus: vi.fn(),
    logout: vi.fn(),
    getRoomInfo: vi.fn(),
    startDanmaku: vi.fn(),
    stopDanmaku: vi.fn(),
    getDanmakuStatus: vi.fn(),
    listenDanmakuMessage: vi.fn(),
    listenDanmakuStatus: vi.fn(),
  },
}));

vi.mock("../src/services/runtime", () => ({ isTauriRuntime: mocks.isTauriRuntime }));
vi.mock("../src/services/bilibili", () => ({ bilibiliApi: mocks.api }));

import HomePage from "../src/features/home/HomePage.vue";
import NanaSessionProvider from "../src/features/session/NanaSessionProvider.vue";

const RouterLinkStub = defineComponent({
  props: { to: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h("a", { href: props.to }, slots.default?.());
  },
});

const account: AccountStatus = {
  authenticated: true,
  account: { mid: 7, username: "Nana", avatar_url: null },
};

const room: RoomInfo = {
  room_id: 123,
  owner_id: 7,
  owner_name: "Nana",
  owner_avatar_url: "https://example.com/avatar.png",
  title: "直播测试",
  live_status: "live",
  viewer_count: 42,
  follower_count: 8,
  cover_url: null,
  fetched_at: 1,
};

beforeEach(() => {
  vi.clearAllMocks();
  mocks.isTauriRuntime.mockReturnValue(true);
  mocks.api.getStatus.mockResolvedValue(account);
  mocks.api.getRoomInfo.mockResolvedValue(room);
  mocks.api.startQr.mockResolvedValue({ session_id: "session", svg: "<svg />", expires_at: 10 });
  mocks.api.pollQr.mockResolvedValue({ status: "success", account: account.account });
  mocks.api.logout.mockResolvedValue(undefined);
  mocks.api.startDanmaku.mockResolvedValue({ connection_id: "connection", room_id: 123 });
  mocks.api.stopDanmaku.mockResolvedValue(undefined);
  mocks.api.getDanmakuStatus.mockResolvedValue({ connection_id: null, room_id: null, state: "idle", message: null });
  mocks.api.listenDanmakuMessage.mockResolvedValue(() => undefined);
  mocks.api.listenDanmakuStatus.mockResolvedValue(() => undefined);
});

function renderHome() {
  return render(NanaSessionProvider, {
    slots: { default: () => h(HomePage) },
    global: { stubs: { RouterLink: RouterLinkStub } },
  });
}

describe("首页直播间工作流", () => {
  it("未登录时只显示登录入口，不显示房间输入", async () => {
    mocks.api.getStatus.mockResolvedValue({ authenticated: false, account: null });
    renderHome();

    await screen.findByRole("heading", { name: "登录后连接直播间" });
    expect(screen.queryByRole("textbox", { name: "直播间号" })).toBeNull();
    expect(screen.getByRole("img", { name: "直播间在线人数和关注数趋势图" })).toBeVisible();
    expect(screen.getByText("连接直播间后开始记录数据")).toBeVisible();
    expect(screen.getAllByText("暂无")).toHaveLength(2);
    expect(screen.queryByText("数据趋势")).toBeNull();
    expect(screen.queryByText("直播间", { exact: true })).toBeNull();
    expect(screen.queryByText("接收当前直播间的实时弹幕")).toBeNull();
  });

  it("扫码成功后才显示房间连接输入", async () => {
    mocks.api.getStatus.mockResolvedValue({ authenticated: false, account: null });
    renderHome();

    await waitFor(() => expect(mocks.api.getStatus).toHaveBeenCalledOnce());
    const loginButton = await screen.findByRole("button", { name: "扫码登录" });
    await fireEvent.click(loginButton);
    await fireEvent.click(await screen.findByRole("button", { name: "检查登录状态" }));

    await screen.findByRole("textbox", { name: "直播间号" });
    expect(screen.getByRole("button", { name: "连接直播间" })).toBeVisible();
    expect(screen.queryByText("输入房间号，查看主播和当前直播状态。")).toBeNull();
  });

  it("恢复保存的房间并显示主播头像，切换时清除保存状态", async () => {
    localStorage.setItem("nanabobo.live.room-id", "123");
    renderHome();

    await screen.findByText("Nana");
    const avatar = screen.getByRole("img", { name: "" });
    expect(avatar.getAttribute("src")).toBe(room.owner_avatar_url);
    expect(mocks.api.getRoomInfo).toHaveBeenCalledWith("123");
    expect(screen.getByRole("img", { name: "直播间在线人数和关注数趋势图" })).toBeVisible();
    expect(screen.getAllByText("42").length).toBeGreaterThan(0);
    expect(screen.getByText("最近流水")).toBeVisible();
    expect(screen.getByText("当前关注数")).toBeVisible();
    expect(screen.queryByText("直播测试")).toBeNull();
    expect(screen.queryByText("直播状态")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "切换直播间" }));
    await waitFor(() => expect(localStorage.getItem("nanabobo.live.room-id")).toBeNull());
    expect(screen.getByRole("textbox", { name: "直播间号" })).toBeVisible();
  });
});
