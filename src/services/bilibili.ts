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

export type UnlistenFn = () => void;
export type InvokeCommand = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
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

/** Nana 宿主传输:命令经 __nanaHost.invoke,事件经宿主桥 on 订阅。 */
function detectTransport(): { invoke: InvokeCommand; listen: EventListen } {
  const host = nanaHost();
  const invoke = ((command, args) =>
    host.invoke(command, args === undefined ? [] : [args])) as InvokeCommand;
  const listen: EventListen = async (event, handler) => {
    host.on(event, handler as (payload: unknown) => void);
    return () => {};
  };
  return { invoke, listen };
}

const transport = detectTransport();

export function createBilibiliApi(
  invokeCommand: InvokeCommand = transport.invoke,
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
