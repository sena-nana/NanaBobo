//! 应用命令核心:与 UI 框架无关的命令实现。
//!
//! 宿主层只做参数解包与状态提取,
//! 业务语义、错误码与前端契约都定义在这里。

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use uuid::Uuid;

use crate::{
    bilibili::{BilibiliClient, BilibiliError, DanmakuManager, QrPollResult, QrSession},
    credential_store::{CredentialStore, CredentialStoreError},
    events::EventSink,
    models::{
        AccountStatus, AuthPollResponse, DanmakuConnection, DanmakuStatus, QrStartResponse,
        RoomInfo,
    },
};

pub struct AppState {
    pub client: BilibiliClient,
    pub credentials: Arc<dyn CredentialStore>,
    pub qr_sessions: Mutex<HashMap<String, QrSession>>,
    pub danmaku: Arc<DanmakuManager>,
}

impl AppState {
    pub fn new(
        client: BilibiliClient,
        credentials: Arc<dyn CredentialStore>,
        runtime: tokio::runtime::Handle,
        sink: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            client,
            credentials,
            qr_sessions: Mutex::new(HashMap::new()),
            danmaku: Arc::new(DanmakuManager::new(runtime, sink)),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    CredentialStore,
    InvalidInput,
    NotAuthenticated,
    QrExpired,
    RequestLimited,
    UpstreamUnavailable,
    ConnectionUnavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    fn credential_store() -> Self {
        Self {
            code: ErrorCode::CredentialStore,
            message: "登录状态暂时无法访问，请稍后重试。".to_owned(),
        }
    }

    fn qr_expired() -> Self {
        Self {
            code: ErrorCode::QrExpired,
            message: "二维码已过期，请重新生成。".to_owned(),
        }
    }

    fn connection_unavailable() -> Self {
        Self {
            code: ErrorCode::ConnectionUnavailable,
            message: "弹幕连接暂时不可用，请稍后重试。".to_owned(),
        }
    }
}

impl From<CredentialStoreError> for AppError {
    fn from(_: CredentialStoreError) -> Self {
        Self::credential_store()
    }
}

impl From<BilibiliError> for AppError {
    fn from(error: BilibiliError) -> Self {
        match error {
            BilibiliError::HttpStatus(412 | 429) => Self {
                code: ErrorCode::RequestLimited,
                message: "请求过于频繁，请稍后再试。".to_owned(),
            },
            BilibiliError::Api {
                code: 412 | 429, ..
            } => Self {
                code: ErrorCode::RequestLimited,
                message: "请求过于频繁，请稍后再试。".to_owned(),
            },
            BilibiliError::InvalidLoginUrl => Self {
                code: ErrorCode::UpstreamUnavailable,
                message: "登录服务返回了无法使用的结果，请重新尝试。".to_owned(),
            },
            BilibiliError::Request(_)
            | BilibiliError::HttpStatus(_)
            | BilibiliError::Api { .. }
            | BilibiliError::InvalidResponse => Self {
                code: ErrorCode::UpstreamUnavailable,
                message: "B站服务暂时不可用，请稍后重试。".to_owned(),
            },
        }
    }
}

pub async fn auth_qr_start(state: &AppState) -> Result<QrStartResponse, AppError> {
    let (session, payload) = state.client.generate_qr().await.map_err(AppError::from)?;
    let session_id = Uuid::new_v4().to_string();
    let expires_at = session.expires_at;
    state
        .qr_sessions
        .lock()
        .map_err(|_| AppError::credential_store())?
        .insert(session_id.clone(), session);
    Ok(QrStartResponse {
        session_id,
        payload,
        expires_at,
    })
}

pub async fn auth_qr_poll(
    state: &AppState,
    session_id: String,
) -> Result<AuthPollResponse, AppError> {
    if session_id.trim().is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidInput,
            message: "登录会话无效，请重新生成二维码。".to_owned(),
        });
    }
    let session = state
        .qr_sessions
        .lock()
        .map_err(|_| AppError::credential_store())?
        .get(&session_id)
        .cloned()
        .ok_or_else(AppError::qr_expired)?;
    if session.expires_at <= now_seconds() {
        remove_session(state, &session_id)?;
        return Err(AppError::qr_expired());
    }

    match state
        .client
        .poll_qr(&session.qrcode_key)
        .await
        .map_err(AppError::from)?
    {
        QrPollResult::Pending => Ok(AuthPollResponse::Pending),
        QrPollResult::Scanned => Ok(AuthPollResponse::Scanned),
        QrPollResult::Expired => {
            remove_session(state, &session_id)?;
            Ok(AuthPollResponse::Expired)
        }
        QrPollResult::Success { cookie } => {
            state.credentials.save(&cookie)?;
            let account = state
                .client
                .account_status(&cookie)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| AppError {
                    code: ErrorCode::NotAuthenticated,
                    message: "扫码成功，但账号状态仍未确认。".to_owned(),
                })?;
            remove_session(state, &session_id)?;
            Ok(AuthPollResponse::Success { account })
        }
    }
}

pub async fn auth_status(state: &AppState) -> Result<AccountStatus, AppError> {
    let Some(cookie) = state.credentials.load()? else {
        return Ok(AccountStatus {
            authenticated: false,
            account: None,
        });
    };
    let account = state
        .client
        .account_status(&cookie)
        .await
        .map_err(AppError::from)?;
    Ok(AccountStatus {
        authenticated: account.is_some(),
        account,
    })
}

pub fn auth_logout(state: &AppState) -> Result<(), AppError> {
    state.danmaku.stop(None);
    state.credentials.clear()?;
    state
        .qr_sessions
        .lock()
        .map_err(|_| AppError::credential_store())?
        .clear();
    Ok(())
}

pub fn danmaku_start(state: &AppState, room_id: u64) -> Result<DanmakuConnection, AppError> {
    if room_id == 0 {
        return Err(AppError {
            code: ErrorCode::InvalidInput,
            message: "请输入有效的直播间号。".to_owned(),
        });
    }
    state
        .danmaku
        .start(state.client.clone(), room_id)
        .map_err(|_| AppError::connection_unavailable())
}

pub fn danmaku_stop(state: &AppState, connection_id: String) -> Result<(), AppError> {
    if connection_id.trim().is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidInput,
            message: "弹幕连接无效，请重新连接。".to_owned(),
        });
    }
    state.danmaku.stop(Some(connection_id.trim()));
    Ok(())
}

pub fn danmaku_status(state: &AppState) -> Result<DanmakuStatus, AppError> {
    Ok(state.danmaku.status())
}

pub async fn room_get_info(state: &AppState, room_id: String) -> Result<RoomInfo, AppError> {
    let room_id = room_id
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError {
            code: ErrorCode::InvalidInput,
            message: "请输入有效的直播间号。".to_owned(),
        })?;
    state
        .client
        .room_info(room_id)
        .await
        .map_err(AppError::from)
}

fn remove_session(state: &AppState, session_id: &str) -> Result<(), AppError> {
    state
        .qr_sessions
        .lock()
        .map_err(|_| AppError::credential_store())?
        .remove(session_id);
    Ok(())
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::ErrorCode;

    #[test]
    fn request_limits_are_a_stable_error_code() {
        let error = super::AppError::from(crate::bilibili::BilibiliError::HttpStatus(429));
        assert_eq!(error.code, ErrorCode::RequestLimited);
        assert!(!error.message.contains("429"));
    }
}
