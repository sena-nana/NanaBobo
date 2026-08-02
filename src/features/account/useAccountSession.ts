import { onBeforeUnmount, ref } from "vue";
import type {
  AccountStatus,
  AuthPollResponse,
  BilibiliApiError,
  QrStartResponse,
} from "../../contracts/bilibili";
import { bilibiliApi, type BilibiliApi } from "../../services/bilibili";
import { isTauriRuntime } from "../../services/runtime";

export function normalizeBilibiliError(error: unknown): BilibiliApiError {
  if (typeof error === "object" && error !== null) {
    const candidate = error as Partial<BilibiliApiError>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return { code: candidate.code as BilibiliApiError["code"], message: candidate.message };
    }
  }
  return { code: "upstream_unavailable", message: "B站服务暂时不可用，请稍后重试。" };
}

export function useAccountSession(
  api: BilibiliApi = bilibiliApi,
  desktopRuntime: () => boolean = isTauriRuntime,
) {
  const available = desktopRuntime();
  const status = ref<AccountStatus | null>(null);
  const qr = ref<QrStartResponse | null>(null);
  const qrState = ref<"idle" | "pending" | "scanned" | "expired">("idle");
  const loading = ref(false);
  const polling = ref(false);
  const error = ref<BilibiliApiError | null>(null);
  let pollTimer: ReturnType<typeof setTimeout> | undefined;

  function stopPolling() {
    if (pollTimer !== undefined) clearTimeout(pollTimer);
    pollTimer = undefined;
  }

  function schedulePoll() {
    stopPolling();
    pollTimer = setTimeout(() => { void pollOnce(); }, 1500);
  }

  async function loadStatus() {
    if (!available) return;
    loading.value = true;
    error.value = null;
    try {
      status.value = await api.getStatus();
    } catch (cause) {
      error.value = normalizeBilibiliError(cause);
    } finally {
      loading.value = false;
    }
  }

  async function startLogin() {
    if (!available) {
      error.value = { code: "desktop_unavailable", message: "请在桌面应用中管理B站账号。" };
      return;
    }
    stopPolling();
    loading.value = true;
    error.value = null;
    qrState.value = "idle";
    try {
      qr.value = await api.startQr();
      qrState.value = "pending";
      schedulePoll();
    } catch (cause) {
      qr.value = null;
      error.value = normalizeBilibiliError(cause);
    } finally {
      loading.value = false;
    }
  }

  async function pollOnce() {
    if (!qr.value || polling.value) return;
    polling.value = true;
    error.value = null;
    try {
      const result = await api.pollQr(qr.value.session_id);
      applyPollResult(result);
    } catch (cause) {
      stopPolling();
      error.value = normalizeBilibiliError(cause);
    } finally {
      polling.value = false;
    }
  }

  function applyPollResult(result: AuthPollResponse) {
    if (result.status === "pending" || result.status === "scanned") {
      qrState.value = result.status;
      schedulePoll();
      return;
    }
    stopPolling();
    if (result.status === "expired") {
      qrState.value = "expired";
      return;
    }
    status.value = { authenticated: true, account: result.account };
    qr.value = null;
    qrState.value = "idle";
  }

  async function logout() {
    if (!available) return;
    loading.value = true;
    error.value = null;
    try {
      await api.logout();
      status.value = { authenticated: false, account: null };
      qr.value = null;
      qrState.value = "idle";
      stopPolling();
    } catch (cause) {
      error.value = normalizeBilibiliError(cause);
    } finally {
      loading.value = false;
    }
  }

  onBeforeUnmount(stopPolling);

  return {
    available,
    status,
    qr,
    qrState,
    loading,
    polling,
    error,
    loadStatus,
    startLogin,
    pollOnce,
    logout,
  };
}

export type AccountSession = ReturnType<typeof useAccountSession>;
