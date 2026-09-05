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
    client: BilibiliClient,
    credentials: Arc<dyn CredentialStore>,
    auth: Mutex<AuthLifecycle>,
    auth_poll: tokio::sync::Mutex<()>,
    danmaku: Arc<DanmakuManager>,
}

#[derive(Default)]
struct AuthLifecycle {
    generation: u64,
    sessions: HashMap<String, QrSession>,
}
impl AuthLifecycle {
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.sessions.clear();
    }
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
            auth: Mutex::new(AuthLifecycle::default()),
            auth_poll: tokio::sync::Mutex::new(()),
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
    let generation = {
        let mut auth = state
            .auth
            .lock()
            .map_err(|_| AppError::credential_store())?;
        auth.invalidate();
        auth.generation
    };
    let (session, payload) = state.client.generate_qr().await.map_err(AppError::from)?;
    let session_id = Uuid::new_v4().to_string();
    let expires_at = session.expires_at;
    let mut auth = state
        .auth
        .lock()
        .map_err(|_| AppError::credential_store())?;
    if auth.generation != generation {
        return Err(AppError::qr_expired());
    }
    auth.sessions.insert(session_id.clone(), session);
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
    let _poll = state.auth_poll.try_lock().map_err(|_| AppError {
        code: ErrorCode::RequestLimited,
        message: "正在确认扫码结果，请稍候。".to_owned(),
    })?;
    let (generation, session) = {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::credential_store())?;
        (
            auth.generation,
            auth.sessions
                .get(&session_id)
                .cloned()
                .ok_or_else(AppError::qr_expired)?,
        )
    };
    if session.expires_at <= now_seconds() {
        remove_session(state, &session_id)?;
        return Err(AppError::qr_expired());
    }

    let result = state
        .client
        .poll_qr(&session.qrcode_key)
        .await
        .map_err(AppError::from)?;
    {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::credential_store())?;
        if auth.generation != generation || !auth.sessions.contains_key(&session_id) {
            return Err(AppError::qr_expired());
        }
    }
    match result {
        QrPollResult::Pending => Ok(AuthPollResponse::Pending),
        QrPollResult::Scanned => Ok(AuthPollResponse::Scanned),
        QrPollResult::Expired => {
            remove_session(state, &session_id)?;
            Ok(AuthPollResponse::Expired)
        }
        QrPollResult::Success { cookie } => {
            let account = state
                .client
                .account_status(&cookie)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| AppError {
                    code: ErrorCode::NotAuthenticated,
                    message: "扫码成功，但账号状态仍未确认。".to_owned(),
                })?;
            commit_login(state, generation, &session_id, &cookie)?;
            Ok(AuthPollResponse::Success { account })
        }
    }
}

fn commit_login(
    state: &AppState,
    generation: u64,
    session_id: &str,
    cookie: &str,
) -> Result<(), AppError> {
    let mut auth = state
        .auth
        .lock()
        .map_err(|_| AppError::credential_store())?;
    if auth.generation != generation || !auth.sessions.contains_key(session_id) {
        return Err(AppError::qr_expired());
    }
    state.credentials.save(cookie)?;
    auth.invalidate();
    Ok(())
}

pub async fn auth_status(state: &AppState) -> Result<AccountStatus, AppError> {
    let (generation, cookie) = {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::credential_store())?;
        (auth.generation, state.credentials.load()?)
    };
    let Some(cookie) = cookie else {
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
    if state
        .auth
        .lock()
        .map_err(|_| AppError::credential_store())?
        .generation
        != generation
    {
        return Err(AppError {
            code: ErrorCode::NotAuthenticated,
            message: "登录状态已更新，请重试。".into(),
        });
    }
    Ok(AccountStatus {
        authenticated: account.is_some(),
        account,
    })
}

pub fn auth_qr_cancel(state: &AppState) -> Result<(), AppError> {
    state
        .auth
        .lock()
        .map_err(|_| AppError::credential_store())?
        .invalidate();
    Ok(())
}

pub fn auth_logout(state: &AppState) -> Result<(), AppError> {
    let mut auth = state
        .auth
        .lock()
        .map_err(|_| AppError::credential_store())?;
    auth.invalidate();
    state.credentials.clear()?;
    state.danmaku.stop(None);
    Ok(())
}

pub async fn fetch_image(state: &AppState, url: &str) -> Result<Vec<u8>, AppError> {
    state.client.fetch_bytes(url).await.map_err(AppError::from)
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
        .auth
        .lock()
        .map_err(|_| AppError::credential_store())?
        .sessions
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
    fn fixture() -> (tokio::runtime::Runtime, super::AppState) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = super::AppState::new(
            crate::bilibili::BilibiliClient::new().unwrap(),
            std::sync::Arc::new(crate::credential_store::MemoryCredentialStore::default()),
            runtime.handle().clone(),
            std::sync::Arc::new(crate::events::NullEventSink),
        );
        (runtime, state)
    }
    fn pending(state: &super::AppState) -> u64 {
        let mut auth = state.auth.lock().unwrap();
        auth.sessions.insert(
            "active".into(),
            crate::bilibili::QrSession {
                qrcode_key: "test".into(),
                expires_at: u64::MAX,
            },
        );
        auth.generation
    }
    #[test]
    fn cancelled_or_logged_out_login_cannot_restore_credentials() {
        let (_runtime, state) = fixture();
        for logout in [false, true] {
            let generation = pending(&state);
            if logout {
                super::auth_logout(&state).unwrap();
            } else {
                super::auth_qr_cancel(&state).unwrap();
            }
            assert_eq!(
                super::commit_login(&state, generation, "active", "test")
                    .unwrap_err()
                    .code,
                ErrorCode::QrExpired
            );
            assert!(state.credentials.load().unwrap().is_none());
        }
    }
    #[test]
    fn successful_login_consumes_session_and_logout_clears_it() {
        let (_runtime, state) = fixture();
        let generation = pending(&state);
        super::commit_login(&state, generation, "active", "test").unwrap();
        assert_eq!(state.credentials.load().unwrap().as_deref(), Some("test"));
        assert!(super::commit_login(&state, generation, "active", "late").is_err());
        super::auth_logout(&state).unwrap();
        assert!(state.credentials.load().unwrap().is_none());
    }
}
