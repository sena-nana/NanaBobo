import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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

export function createBilibiliApi(invokeCommand: TauriInvoke = invoke): BilibiliApi {
  return {
    startQr: () => invokeCommand<QrStartResponse>("auth_qr_start"),
    pollQr: (sessionId) => invokeCommand<AuthPollResponse>("auth_qr_poll", { sessionId }),
    getStatus: () => invokeCommand<AccountStatus>("auth_status"),
    logout: () => invokeCommand<void>("auth_logout"),
    getRoomInfo: (roomId) => invokeCommand<RoomInfo>("room_get_info", { roomId }),
    startDanmaku: (roomId) => invokeCommand<DanmakuConnection>("danmaku_start", { roomId }),
    stopDanmaku: (connectionId) => invokeCommand<void>("danmaku_stop", { connectionId }),
    getDanmakuStatus: () => invokeCommand<DanmakuStatus>("danmaku_status"),
    listenDanmakuMessage: (handler) => listen<DanmakuMessage>("nanabobo://danmaku/message", (event) => handler(event.payload)),
    listenDanmakuStatus: (handler) => listen<DanmakuStatus>("nanabobo://danmaku/status", (event) => handler(event.payload)),
  };
}

export const bilibiliApi = createBilibiliApi();
