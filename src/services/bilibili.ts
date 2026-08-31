import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { nanaHost } from "../ui/nanaHost";
import type {
    AccountStatus,
    AuthPollResponse,
    DanmakuConnection,
    DanmakuMessage,
    DanmakuStatus,
    QrStartResponse,
    RoomInfo,
} from "../contracts/bilibili";

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
export type EventListen = <T>(
    event: string,
    handler: (payload: T) => void,
) => Promise<UnlistenFn>;

export interface BilibiliApi {
  startQr: () => Promise<QrStartResponse>;
  pollQr: (sessionId: string) => Promise<AuthPollResponse>;
  getStatus: () => Promise<AccountStatus>;
  logout: () => Promise<void>;
  getRoomInfo: (roomId: string) => Promise<RoomInfo>;
  startDanmaku: (roomId: number) => Promise<DanmakuConnection>;
  stopDanmaku: (connectionId: string) => Promise<void>;
  getDanmakuStatus: () => Promise<DanmakuStatus>;
  listenDanmakuMessage: (handler: (message: DanmakuMessage) => void) => Promise<UnlistenFn>;
  listenDanmakuStatus: (handler: (status: DanmakuStatus) => void) => Promise<UnlistenFn>;
}

function tauriListen(): EventListen {
  return (event, handler) =>
    listen(event, (backendEvent) => handler(backendEvent.payload as never));
}

function nanaListen(): EventListen {
  const host = nanaHost();
  return async (event, handler) => {
    host.on(event, handler as (payload: unknown) => void);
    return () => {};
  };
}

/** 宿主运行时探测:Tauri WebView 与 NanaUI 宿主共用同一套命令契约。 */
function detectTransport(): { invoke: TauriInvoke; listen: EventListen } {
  if (typeof globalThis !== "undefined" && "Nana" in globalThis) {
    const host = nanaHost();
    return {
      invoke: ((command, args) =>
        host.invoke(command, args === undefined ? [] : [args])) as TauriInvoke,
      listen: nanaListen(),
    };
  }
  return { invoke, listen: tauriListen() };
}

const transport = detectTransport();

export function createBilibiliApi(
  invokeCommand: TauriInvoke = transport.invoke,
  listenCommand: EventListen = transport.listen,
): BilibiliApi {
  return {
    startQr: () => invokeCommand<QrStartResponse>("auth_qr_start"),
    pollQr: (sessionId) => invokeCommand<AuthPollResponse>("auth_qr_poll", { sessionId }),
    getStatus: () => invokeCommand<AccountStatus>("auth_status"),
    logout: () => invokeCommand<void>("auth_logout"),
    getRoomInfo: (roomId) => invokeCommand<RoomInfo>("room_get_info", { roomId }),
    startDanmaku: (roomId) => invokeCommand<DanmakuConnection>("danmaku_start", { roomId }),
    stopDanmaku: (connectionId) => invokeCommand<void>("danmaku_stop", { connectionId }),
    getDanmakuStatus: () => invokeCommand<DanmakuStatus>("danmaku_status"),
    listenDanmakuMessage: (handler) => listenCommand<DanmakuMessage>("nanabobo://danmaku/message", handler),
    listenDanmakuStatus: (handler) => listenCommand<DanmakuStatus>("nanabobo://danmaku/status", handler),
  };
}

export const bilibiliApi = createBilibiliApi();
