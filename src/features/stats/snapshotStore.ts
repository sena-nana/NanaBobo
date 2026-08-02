import type { RoomInfo, StatsSnapshot } from "../../contracts/bilibili";

const SNAPSHOT_STORAGE_KEY = "nanabobo.live.snapshots.v1";
export const MAX_SNAPSHOTS = 2_000;

function trimSnapshots(snapshots: StatsSnapshot[]) {
  const counts = new Map<number, number>();
  const kept: StatsSnapshot[] = [];
  for (let index = snapshots.length - 1; index >= 0; index -= 1) {
    const snapshot = snapshots[index];
    const count = counts.get(snapshot.room_id) ?? 0;
    if (count >= MAX_SNAPSHOTS) continue;
    counts.set(snapshot.room_id, count + 1);
    kept.push(snapshot);
  }
  return kept.reverse();
}

function isSnapshot(value: unknown): value is StatsSnapshot {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<StatsSnapshot>;
  return typeof candidate.room_id === "number"
    && typeof candidate.captured_at === "number"
    && typeof candidate.viewer_count === "number"
    && (candidate.follower_count === null || typeof candidate.follower_count === "number")
    && typeof candidate.live_status === "string";
}

export function readSnapshots(): StatsSnapshot[] {
  try {
    const raw = localStorage.getItem(SNAPSHOT_STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? trimSnapshots(parsed.filter(isSnapshot)) : [];
  } catch {
    return [];
  }
}

function writeSnapshots(snapshots: StatsSnapshot[]) {
  try {
    localStorage.setItem(SNAPSHOT_STORAGE_KEY, JSON.stringify(trimSnapshots(snapshots)));
  } catch {
    // Local persistence is optional; the current session still remains usable.
  }
}

export function appendSnapshot(info: RoomInfo): StatsSnapshot {
  const snapshot: StatsSnapshot = {
    room_id: info.room_id,
    captured_at: info.fetched_at,
    viewer_count: info.viewer_count,
    follower_count: info.follower_count,
    live_status: info.live_status,
  };
  writeSnapshots([...readSnapshots(), snapshot]);
  return snapshot;
}

export function snapshotsForRoom(roomId: number): StatsSnapshot[] {
  return readSnapshots().filter((snapshot) => snapshot.room_id === roomId);
}

export function clearSnapshots(roomId?: number) {
  const snapshots = roomId === undefined
    ? []
    : readSnapshots().filter((snapshot) => snapshot.room_id !== roomId);
  writeSnapshots(snapshots);
}

export function storageKey() {
  return SNAPSHOT_STORAGE_KEY;
}
