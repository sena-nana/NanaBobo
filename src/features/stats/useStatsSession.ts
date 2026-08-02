import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { StatsSnapshot } from "../../contracts/bilibili";
import type { RoomInfoSession } from "../live/useRoomInfo";
import { appendSnapshot, clearSnapshots, readSnapshots } from "./snapshotStore";

const POLL_INTERVAL_MS = 60_000;

export function useStatsSession(room: RoomInfoSession) {
  const snapshots = ref<StatsSnapshot[]>(readSnapshots());
  let timer: ReturnType<typeof setInterval> | undefined;

  function recordCurrentRoom() {
    const info = room.info.value;
    if (!info) return;
    if (snapshots.value.some((snapshot) => snapshot.room_id === info.room_id && snapshot.captured_at === info.fetched_at)) return;
    const snapshot = appendSnapshot(info);
    snapshots.value = [...snapshots.value, snapshot];
  }

  function stopPolling() {
    if (timer !== undefined) clearInterval(timer);
    timer = undefined;
  }

  function startPolling() {
    stopPolling();
    timer = setInterval(() => { void room.query({ preserveInfo: true }); }, POLL_INTERVAL_MS);
  }

  const current = computed(() => {
    const roomId = room.info.value?.room_id;
    return roomId === undefined ? [] : snapshots.value.filter((snapshot) => snapshot.room_id === roomId);
  });

  function clearRoomHistory(roomId = room.info.value?.room_id) {
    if (roomId === undefined) return;
    clearSnapshots(roomId);
    snapshots.value = snapshots.value.filter((snapshot) => snapshot.room_id !== roomId);
  }

  watch(() => room.info.value?.room_id, (roomId) => {
    stopPolling();
    if (roomId !== undefined) {
      recordCurrentRoom();
      startPolling();
    }
  }, { immediate: true });

  watch(() => room.info.value?.fetched_at, () => {
    recordCurrentRoom();
  });

  onBeforeUnmount(stopPolling);

  return { snapshots, current, recordCurrentRoom, clearRoomHistory, stopPolling };
}

export type StatsSession = ReturnType<typeof useStatsSession>;
