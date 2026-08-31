import { mount, type VueWrapper } from "@vue/test-utils";
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
  return mount(NanaSessionProvider, {
    slots: { default: () => h(HomePage) },
    global: { stubs: { RouterLink: RouterLinkStub } },
  });
}

/** 等价于 testing-library 的 queryByText 精确匹配:存在自身文本恰好等于目标的元素。 */
function hasExactText(wrapper: VueWrapper, text: string) {
  return wrapper.findAll("*").some((node) => node.text() === text && node.children().length === 0);
}

async function waitForText(wrapper: VueWrapper, text: string) {
  await vi.waitFor(() => expect(wrapper.text()).toContain(text));
}

function expectHomeSummaryRemoved(wrapper: VueWrapper) {
  expect(wrapper.find('[data-agent-id="home.summary"]').exists()).toBe(false);
  expect(wrapper.text()).not.toContain("最近流水");
  expect(wrapper.text()).not.toContain("当前关注数");
}

describe("首页直播间工作流", () => {
  it("未登录时只显示登录入口，不显示房间输入", async () => {
    mocks.api.getStatus.mockResolvedValue({ authenticated: false, account: null });
    const wrapper = renderHome();

    await waitForText(wrapper, "登录后连接直播间");
    expect(wrapper.find('[data-agent-id="room.input"]').exists()).toBe(false);
    expect(wrapper.find('svg[aria-label="直播间在线人数和关注数趋势图"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("连接直播间后开始记录数据");
    expectHomeSummaryRemoved(wrapper);
    expect(wrapper.find('[data-agent-id="home.trend-card"]').exists()).toBe(true);
    expect(wrapper.text()).not.toContain("数据趋势");
    expect(hasExactText(wrapper, "直播间")).toBe(false);
    expect(wrapper.text()).not.toContain("接收当前直播间的实时弹幕");
  });

  it("扫码成功后才显示房间连接输入", async () => {
    mocks.api.getStatus.mockResolvedValue({ authenticated: false, account: null });
    const wrapper = renderHome();

    await vi.waitFor(() => expect(mocks.api.getStatus).toHaveBeenCalledOnce());
    const loginButton = await vi.waitFor(() => {
      const button = wrapper.find('[data-agent-id="account.login"]');
      expect(button.exists()).toBe(true);
      return button;
    });
    await loginButton.trigger("click");

    const checkButton = await vi.waitFor(() => {
      const button = wrapper.find('[data-agent-id="account.check-login"]');
      expect(button.exists()).toBe(true);
      return button;
    });
    await checkButton.trigger("click");

    await vi.waitFor(() => expect(wrapper.find('[data-agent-id="room.input"]').exists()).toBe(true));
    expect(wrapper.find('[data-agent-id="room.query"]').exists()).toBe(true);
    expect(wrapper.text()).not.toContain("输入房间号，查看主播和当前直播状态。");
  });

  it("恢复保存的房间并显示主播头像，切换时清除保存状态", async () => {
    localStorage.setItem("nanabobo.live.room-id", "123");
    const wrapper = renderHome();

    await waitForText(wrapper, "Nana");
    await vi.waitFor(() => expect(wrapper.find("img.owner-avatar").exists()).toBe(true));
    const avatar = wrapper.get("img.owner-avatar");
    expect(avatar.attributes("src")).toBe(room.owner_avatar_url);
    expect(avatar.attributes("referrerpolicy")).toBe("no-referrer");
    await vi.waitFor(() => expect(mocks.api.getRoomInfo).toHaveBeenCalledWith("123"));
    expect(wrapper.find('svg[aria-label="直播间在线人数和关注数趋势图"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("42");
    expectHomeSummaryRemoved(wrapper);
    expect(wrapper.text()).not.toContain("直播测试");
    expect(wrapper.text()).not.toContain("直播状态");

    await wrapper.get('[data-agent-id="room.disconnect"]').trigger("click");
    await vi.waitFor(() => expect(localStorage.getItem("nanabobo.live.room-id")).toBeNull());
    await vi.waitFor(() => expect(wrapper.find('[data-agent-id="room.input"]').exists()).toBe(true));
  });
});
