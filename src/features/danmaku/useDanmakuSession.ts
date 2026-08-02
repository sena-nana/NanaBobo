import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { DanmakuMessage, DanmakuStatus, BilibiliApiError } from "../../contracts/bilibili";
import { bilibiliApi, type BilibiliApi } from "../../services/bilibili";
import { isTauriRuntime } from "../../services/runtime";
import { normalizeBilibiliError } from "../account/useAccountSession";
import type { RoomInfoSession } from "../live/useRoomInfo";

const INITIAL_STATUS: DanmakuStatus = {
  connection_id: null,
  room_id: null,
  state: "idle",
  message: null,
};

export function useDanmakuSession(
  room: RoomInfoSession,
  api: BilibiliApi = bilibiliApi,
  desktopRuntime: () => boolean = isTauriRuntime,
) {
  const available = desktopRuntime();
  const status = ref<DanmakuStatus>({ ...INITIAL_STATUS });
  const messages = ref<DanmakuMessage[]>([]);
  const loading = ref(false);
  const error = ref<BilibiliApiError | null>(null);
  let unlistenMessage: (() => void) | undefined;
  let unlistenStatus: (() => void) | undefined;

  async function start() {
    const roomId = room.info.value?.room_id;
    if (!available) {
      error.value = { code: "desktop_unavailable", message: "请在桌面应用中使用弹幕助手。" };
      return;
    }
    if (roomId === undefined) {
      error.value = { code: "invalid_input", message: "请先连接一个直播间。" };
      return;
    }
    loading.value = true;
    error.value = null;
    messages.value = [];
    try {
      const connection = await api.startDanmaku(roomId);
      status.value = { ...status.value, connection_id: connection.connection_id, room_id: roomId, state: "connecting", message: null };
    } catch (cause) {
      error.value = normalizeBilibiliError(cause);
    } finally {
      loading.value = false;
    }
  }

  async function stop() {
    const connectionId = status.value.connection_id;
    if (!available || !connectionId) {
      status.value = { ...INITIAL_STATUS, state: "stopped" };
      return;
    }
    loading.value = true;
    try {
      await api.stopDanmaku(connectionId);
      status.value = { ...INITIAL_STATUS, state: "stopped" };
    } catch (cause) {
      error.value = normalizeBilibiliError(cause);
    } finally {
      loading.value = false;
    }
  }

  function handleMessage(message: DanmakuMessage) {
    if (message.room_id !== room.info.value?.room_id) return;
    messages.value = [...messages.value, message].slice(-1_000);
  }

  function handleStatus(next: DanmakuStatus) {
    status.value = next;
    if (next.state === "error") {
      error.value = { code: "connection_unavailable", message: next.message ?? "弹幕连接暂时不可用，请稍后重试。" };
    }
  }

  onMounted(async () => {
    if (!available) return;
    try {
      unlistenMessage = await api.listenDanmakuMessage(handleMessage);
      unlistenStatus = await api.listenDanmakuStatus(handleStatus);
      status.value = await api.getDanmakuStatus();
    } catch (cause) {
      error.value = normalizeBilibiliError(cause);
    }
  });

  watch(() => room.info.value?.room_id, (roomId, previousRoomId) => {
    if (previousRoomId !== undefined && roomId !== previousRoomId && status.value.connection_id) {
      void stop();
    }
    if (roomId === undefined) {
      messages.value = [];
      status.value = { ...INITIAL_STATUS, state: "idle" };
    }
  });

  onBeforeUnmount(() => {
    unlistenMessage?.();
    unlistenStatus?.();
    if (status.value.connection_id) void stop();
  });

  return { available, status, messages, loading, error, start, stop };
}

export type DanmakuSession = ReturnType<typeof useDanmakuSession>;
