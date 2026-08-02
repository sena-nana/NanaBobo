import { render } from "@testing-library/vue";
import { nextTick, ref, defineComponent } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RoomInfo } from "../src/contracts/bilibili";
import { appendSnapshot, readSnapshots, storageKey } from "../src/features/stats/snapshotStore";
import { useStatsSession } from "../src/features/stats/useStatsSession";

const room: RoomInfo = {
  room_id: 123,
  owner_id: 7,
  owner_name: "Nana",
  owner_avatar_url: "https://example.com/avatar.png",
  title: "直播测试",
  live_status: "live",
  viewer_count: 42,
  follower_count: 8,
  cover_url: "https://example.com/cover.png",
  fetched_at: 1,
};

beforeEach(() => localStorage.clear());
afterEach(() => {
  vi.useRealTimers();
  localStorage.clear();
});

describe("脱敏直播快照", () => {
  it("只持久化稳定字段，不保存主播或封面原始字段", () => {
    appendSnapshot(room);

    const stored = localStorage.getItem(storageKey());
    expect(stored).toContain('"room_id":123');
    expect(stored).toContain('"follower_count":8');
    expect(stored).not.toContain("Nana");
    expect(stored).not.toContain("avatar.png");
    expect(stored).not.toContain("cover.png");
  });

  it("按房间分别裁剪到 2,000 条", () => {
    const roomOne = Array.from({ length: 2_001 }, (_, index) => ({
      room_id: 123,
      captured_at: index,
      viewer_count: index,
      follower_count: null,
      live_status: "live",
    }));
    const roomTwo = [{
      room_id: 456,
      captured_at: 1,
      viewer_count: 1,
      follower_count: null,
      live_status: "offline",
    }];
    localStorage.setItem(storageKey(), JSON.stringify([...roomOne, ...roomTwo]));

    const snapshots = readSnapshots();
    expect(snapshots.filter((snapshot) => snapshot.room_id === 123)).toHaveLength(2_000);
    expect(snapshots.filter((snapshot) => snapshot.room_id === 456)).toHaveLength(1);
    expect(snapshots.find((snapshot) => snapshot.room_id === 123)?.captured_at).toBe(1);
  });
});

describe("统计会话", () => {
  it("连接后记录快照并启动 60 秒轮询", async () => {
    vi.useFakeTimers();
    const roomSession = { info: ref<RoomInfo | null>(null), query: vi.fn(async () => undefined) };
    let stats: ReturnType<typeof useStatsSession> | undefined;
    const Host = defineComponent({
      setup() {
        stats = useStatsSession(roomSession as never);
        return () => null;
      },
    });
    render(Host);

    roomSession.info.value = room;
    await nextTick();
    expect(stats?.current.value).toHaveLength(1);

    await vi.advanceTimersByTimeAsync(60_000);
    expect(roomSession.query).toHaveBeenCalledWith({ preserveInfo: true });
  });
});
