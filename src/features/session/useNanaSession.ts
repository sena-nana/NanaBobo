import { computed, onMounted, provide, watch, inject, type InjectionKey } from "vue";
import { usePersistentString } from "../../ui";
import { useAccountSession } from "../account/useAccountSession";
import { useDanmakuSession } from "../danmaku/useDanmakuSession";
import { useRoomInfo } from "../live/useRoomInfo";
import { useStatsSession } from "../stats/useStatsSession";

export function createNanaSession() {
  const account = useAccountSession();
  const room = useRoomInfo();
  const stats = useStatsSession(room);
  const danmaku = useDanmakuSession(room);
  const savedRoomId = usePersistentString("nanabobo.live.room-id", "");
  const authenticated = computed(() => Boolean(
    account.status.value?.authenticated && account.status.value.account,
  ));
  let restoreAttempted = false;

  function clearSavedRoom() {
    savedRoomId.value = "";
    try {
      localStorage.removeItem("nanabobo.live.room-id");
    } catch {
      // Local storage may be unavailable in restricted browser contexts.
    }
  }

  function disconnectRoom() {
    void danmaku.stop();
    room.clear();
    clearSavedRoom();
  }

  onMounted(() => {
    void account.loadStatus();
  });

  watch(() => account.status.value, (status) => {
    if (!status) return;
    if (!status.authenticated || !status.account) {
      void danmaku.stop();
      room.clear();
      clearSavedRoom();
      restoreAttempted = false;
      return;
    }
    if (restoreAttempted) return;
    restoreAttempted = true;
    const previousRoomId = savedRoomId.value.trim();
    if (!previousRoomId) return;
    room.roomId.value = previousRoomId;
    void room.query();
  }, { immediate: true });

  watch(() => room.info.value?.room_id, (roomId) => {
    if (roomId) savedRoomId.value = String(roomId);
  });

  return {
    account,
    room,
    stats,
    danmaku,
    authenticated,
    disconnectRoom,
  };
}

export type NanaSession = ReturnType<typeof createNanaSession>;
export const nanaSessionKey: InjectionKey<NanaSession> = Symbol("nanabobo-session");

export function provideNanaSession() {
  const session = createNanaSession();
  provide(nanaSessionKey, session);
  return session;
}

export function useNanaSession() {
  const session = inject(nanaSessionKey);
  if (!session) throw new Error("NanaBobo session provider is missing");
  return session;
}
