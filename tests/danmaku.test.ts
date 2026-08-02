import { render } from "@testing-library/vue";
import { defineComponent, nextTick, ref } from "vue";
import { describe, expect, it, vi } from "vitest";
import type { DanmakuMessage, DanmakuStatus, RoomInfo } from "../src/contracts/bilibili";
import type { BilibiliApi } from "../src/services/bilibili";
import { useDanmakuSession } from "../src/features/danmaku/useDanmakuSession";

const roomInfo: RoomInfo = {
  room_id: 123,
  owner_id: 7,
  owner_name: "Nana",
  owner_avatar_url: null,
  title: "直播测试",
  live_status: "live",
  viewer_count: 42,
  follower_count: null,
  cover_url: null,
  fetched_at: 1,
};

function roomSession() {
  return { info: ref<RoomInfo | null>(roomInfo) };
}

describe("弹幕接收会话", () => {
  it("浏览器模式不伪造连接", async () => {
    const api = { startDanmaku: vi.fn() } as unknown as BilibiliApi;
    let session: ReturnType<typeof useDanmakuSession> | undefined;
    const Host = defineComponent({
      setup() {
        session = useDanmakuSession(roomSession() as never, api, () => false);
        return () => null;
      },
    });
    render(Host);
    await nextTick();

    await session?.start();

    expect(api.startDanmaku).not.toHaveBeenCalled();
    expect(session?.error.value?.code).toBe("desktop_unavailable");
  });

  it("接收状态和消息，并只保留最近 1,000 条", async () => {
    let onMessage: ((message: DanmakuMessage) => void) | undefined;
    let onStatus: ((status: DanmakuStatus) => void) | undefined;
    const api = {
      startDanmaku: vi.fn(async () => ({ connection_id: "connection", room_id: 123 })),
      stopDanmaku: vi.fn(async () => undefined),
      getDanmakuStatus: vi.fn(async () => ({ connection_id: null, room_id: null, state: "idle" as const, message: null })),
      listenDanmakuMessage: vi.fn(async (handler: (message: DanmakuMessage) => void) => {
        onMessage = handler;
        return () => undefined;
      }),
      listenDanmakuStatus: vi.fn(async (handler: (status: DanmakuStatus) => void) => {
        onStatus = handler;
        return () => undefined;
      }),
    };
    let session: ReturnType<typeof useDanmakuSession> | undefined;
    const Host = defineComponent({
      setup() {
        session = useDanmakuSession(roomSession() as never, api, () => true);
        return () => null;
      },
    });
    render(Host);
    await nextTick();

    onStatus?.({ connection_id: "connection", room_id: 123, state: "connected", message: null });
    for (let index = 0; index < 1_001; index += 1) {
      onMessage?.({
        connection_id: "connection",
        room_id: 123,
        sender_name: `观众${index}`,
        text: "你好",
        sent_at: index,
      });
    }

    expect(session?.status.value.state).toBe("connected");
    expect(session?.messages.value).toHaveLength(1_000);
    expect(session?.messages.value[0]?.sender_name).toBe("观众1");
  });
});
