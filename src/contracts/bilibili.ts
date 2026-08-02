export type BilibiliErrorCode =
  | "credential_store"
  | "invalid_input"
  | "not_authenticated"
  | "qr_expired"
  | "request_limited"
  | "upstream_unavailable"
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
  title: string;
  live_status: "live" | "round" | "offline" | string;
  viewer_count: number;
  cover_url: string | null;
  fetched_at: number;
}
