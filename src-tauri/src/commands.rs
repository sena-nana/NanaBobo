//! Tauri command 薄壳:只做参数解包与状态提取,业务实现全部在
//! `nanabobo-core`。命令名、参数名与返回结构与迁移前保持一致。

use nanabobo_core::commands::{self as core, AppError, AppState};
use nanabobo_core::models::{
    AccountStatus, AuthPollResponse, DanmakuConnection, DanmakuStatus, QrStartResponse, RoomInfo,
};
use tauri::State;

#[tauri::command]
pub async fn auth_qr_start(state: State<'_, AppState>) -> Result<QrStartResponse, AppError> {
    core::auth_qr_start(&state).await
}

#[tauri::command]
pub async fn auth_qr_poll(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<AuthPollResponse, AppError> {
    core::auth_qr_poll(&state, session_id).await
}

#[tauri::command]
pub async fn auth_status(state: State<'_, AppState>) -> Result<AccountStatus, AppError> {
    core::auth_status(&state).await
}

#[tauri::command]
pub fn auth_logout(state: State<'_, AppState>) -> Result<(), AppError> {
    core::auth_logout(&state)
}

#[tauri::command]
pub fn danmaku_start(
    room_id: u64,
    state: State<'_, AppState>,
) -> Result<DanmakuConnection, AppError> {
    core::danmaku_start(&state, room_id)
}

#[tauri::command]
pub fn danmaku_stop(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    core::danmaku_stop(&state, connection_id)
}

#[tauri::command]
pub fn danmaku_status(state: State<'_, AppState>) -> Result<DanmakuStatus, AppError> {
    core::danmaku_status(&state)
}

#[tauri::command]
pub async fn room_get_info(
    room_id: String,
    state: State<'_, AppState>,
) -> Result<RoomInfo, AppError> {
    core::room_get_info(&state, room_id).await
}
