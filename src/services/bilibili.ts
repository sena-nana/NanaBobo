import { invoke } from "@tauri-apps/api/core";
import type {
  AccountStatus,
  AuthPollResponse,
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
}

export function createBilibiliApi(invokeCommand: TauriInvoke = invoke): BilibiliApi {
  return {
    startQr: () => invokeCommand<QrStartResponse>("auth_qr_start"),
    pollQr: (sessionId) => invokeCommand<AuthPollResponse>("auth_qr_poll", { sessionId }),
    getStatus: () => invokeCommand<AccountStatus>("auth_status"),
    logout: () => invokeCommand<void>("auth_logout"),
    getRoomInfo: (roomId) => invokeCommand<RoomInfo>("room_get_info", { roomId }),
  };
}

export const bilibiliApi = createBilibiliApi();
