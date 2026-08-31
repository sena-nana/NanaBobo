import { afterEach, describe, expect, it, vi } from "vitest";
import type { AccountStatus, RoomInfo } from "../src/contracts/bilibili";
import { createBilibiliApi } from "../src/services/bilibili";
import { useAccountSession } from "../src/features/account/useAccountSession";
import { useRoomInfo } from "../src/features/live/useRoomInfo";

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

afterEach(() => vi.useRealTimers());

describe("B站 command facade", () => {
  it("keeps command names and payloads at the app boundary", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const api = createBilibiliApi(
      async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        if (command === "auth_qr_start") return { session_id: "session", svg: "<svg />", expires_at: 10 } as T;
        return room as T;
      },
      async <T>(_event: string, _handler: (payload: T) => void) => () => {},
    );

    await api.startQr();
    await api.getRoomInfo("123");
    expect(calls).toEqual([
      { command: "auth_qr_start", args: undefined },
      { command: "room_get_info", args: { roomId: "123" } },
    ]);
  });
});

describe("account session state", () => {
  it("moves from QR pending to authenticated and can log out", async () => {
    const api = {
      startQr: vi.fn(async () => ({ session_id: "session", svg: "<svg />", expires_at: 10 })),
      pollQr: vi.fn()
        .mockResolvedValueOnce({ status: "pending" as const })
        .mockResolvedValueOnce({ status: "success" as const, account: account.account! }),
      getStatus: vi.fn(async () => account),
      logout: vi.fn(async () => undefined),
      getRoomInfo: vi.fn(async () => room),
      startDanmaku: vi.fn(),
      stopDanmaku: vi.fn(),
      getDanmakuStatus: vi.fn(),
      listenDanmakuMessage: vi.fn(),
      listenDanmakuStatus: vi.fn(),
    };
    const session = useAccountSession(api, () => true);

    await session.startLogin();
    expect(session.qr.value?.session_id).toBe("session");
    await session.pollOnce();
    expect(session.qrState.value).toBe("pending");
    await session.pollOnce();
    expect(session.status.value).toEqual(account);
    expect(session.qr.value).toBeNull();

    await session.logout();
    expect(session.status.value).toEqual({ authenticated: false, account: null });
    expect(api.logout).toHaveBeenCalledOnce();
  });
});

describe("room lookup state", () => {
  it("rejects invalid ids before invoking the backend", async () => {
    const api = {
      startQr: vi.fn(),
      pollQr: vi.fn(),
      getStatus: vi.fn(),
      logout: vi.fn(),
      getRoomInfo: vi.fn(async () => room),
      startDanmaku: vi.fn(),
      stopDanmaku: vi.fn(),
      getDanmakuStatus: vi.fn(),
      listenDanmakuMessage: vi.fn(),
      listenDanmakuStatus: vi.fn(),
    };
    const lookup = useRoomInfo(api, () => true);

    await lookup.query();
    expect(lookup.error.value?.code).toBe("invalid_input");
    expect(api.getRoomInfo).not.toHaveBeenCalled();

    lookup.roomId.value = "123";
    await lookup.query();
    expect(lookup.info.value).toEqual(room);
  });
});
