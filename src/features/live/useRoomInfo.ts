import { onBeforeUnmount, ref } from "vue";
import type { BilibiliApiError, RoomInfo } from "../../contracts/bilibili";
import { bilibiliApi, type BilibiliApi } from "../../services/bilibili";
import { isTauriRuntime } from "../../services/runtime";
import { normalizeBilibiliError } from "../account/useAccountSession";

export function useRoomInfo(
  api: BilibiliApi = bilibiliApi,
  desktopRuntime: () => boolean = isTauriRuntime,
) {
  const available = desktopRuntime();
  const roomId = ref("");
  const info = ref<RoomInfo | null>(null);
  const loading = ref(false);
  const error = ref<BilibiliApiError | null>(null);

  async function query() {
    error.value = null;
    info.value = null;
    if (!available) {
      error.value = { code: "desktop_unavailable", message: "请在桌面应用中查询直播间。" };
      return;
    }
    const normalized = roomId.value.trim();
    if (!/^\d+$/.test(normalized) || Number(normalized) <= 0) {
      error.value = { code: "invalid_input", message: "请输入有效的直播间号。" };
      return;
    }
    loading.value = true;
    try {
      info.value = await api.getRoomInfo(normalized);
    } catch (cause) {
      error.value = normalizeBilibiliError(cause);
    } finally {
      loading.value = false;
    }
  }

  onBeforeUnmount(() => { loading.value = false; });

  return { available, roomId, info, loading, error, query };
}
