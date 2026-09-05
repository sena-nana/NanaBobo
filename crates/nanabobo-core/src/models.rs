use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccountSummary {
    pub mid: u64,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccountStatus {
    pub authenticated: bool,
    pub account: Option<AccountSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QrStartResponse {
    pub session_id: String,
    pub payload: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuthPollResponse {
    Pending,
    Scanned,
    Expired,
    Success { account: AccountSummary },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RoomInfo {
    pub room_id: u64,
    pub owner_id: u64,
    pub owner_name: Option<String>,
    pub owner_avatar_url: Option<String>,
    pub title: String,
    pub live_status: String,
    pub viewer_count: u64,
    pub follower_count: Option<u64>,
    pub cover_url: Option<String>,
    pub fetched_at: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DanmakuConnection {
    pub connection_id: String,
    pub room_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DanmakuConnectionState {
    Idle,
    Connecting,
    Connected,
    Reconnecting,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DanmakuStatus {
    pub connection_id: Option<String>,
    pub room_id: Option<u64>,
    pub state: DanmakuConnectionState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DanmakuMessage {
    pub connection_id: String,
    pub room_id: u64,
    pub sender_name: String,
    pub text: String,
    pub sent_at: u64,
}
