export type BilibiliErrorCode =
  | "credential_store"
  | "invalid_input"
  | "not_authenticated"
  | "qr_expired"
  | "request_limited"
  | "upstream_unavailable"
  | "connection_unavailable"
  | "desktop_unavailable";

export interface BilibiliApiError {
  code: BilibiliErrorCode;
  message: string;
}

export interface AccountSummary {
  mid: number;
  username: string;
  avatar_url: string | null;
}

export interface AccountStatus {
  authenticated: boolean;
  account: AccountSummary | null;
}

export interface QrStartResponse {
  session_id: string;
  svg: string;
  expires_at: number;
}

export type AuthPollResponse =
  | { status: "pending" }
  | { status: "scanned" }
  | { status: "expired" }
  | { status: "success"; account: AccountSummary };

export interface RoomInfo {
  room_id: number;
  owner_id: number;
  owner_name: string | null;
  owner_avatar_url: string | null;
  title: string;
  live_status: "live" | "round" | "offline" | string;
  viewer_count: number;
  follower_count: number | null;
  cover_url: string | null;
  fetched_at: number;
}

export interface StatsSnapshot {
  room_id: number;
  captured_at: number;
  viewer_count: number;
  follower_count: number | null;
  live_status: RoomInfo["live_status"];
}

export type DanmakuConnectionState =
  | "idle"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "stopped"
  | "error";

export interface DanmakuStatus {
  connection_id: string | null;
  room_id: number | null;
  state: DanmakuConnectionState;
  message: string | null;
}

export interface DanmakuConnection {
  connection_id: string;
  room_id: number;
}

export interface DanmakuMessage {
  connection_id: string;
  room_id: number;
  sender_name: string;
  text: string;
  sent_at: number;
}
