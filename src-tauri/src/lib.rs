mod commands;

use std::sync::Arc;

use nanabobo_core::bilibili::BilibiliClient;
use nanabobo_core::commands::AppState;
use nanabobo_core::credential_store::KeyringCredentialStore;
use nanabobo_core::events::EventSink;
use tauri::{Emitter, Manager};

/// 把核心层事件转成 Tauri 前端事件;事件名沿用 `nanabobo://` 约定。
struct TauriEventSink(tauri::AppHandle);

impl EventSink for TauriEventSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let _ = self.0.emit(event, payload);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = BilibiliClient::new().expect("failed to initialize HTTP client");
    let credentials = Arc::new(KeyringCredentialStore::new(
        "com.senanana.nanabobo",
    ));
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_lilia::init())
        .setup(move |app| {
            let sink: Arc<dyn EventSink> = Arc::new(TauriEventSink(app.handle().clone()));
            app.manage(AppState::new(
                client,
                credentials,
                tauri::async_runtime::handle().inner().clone(),
                sink,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth_qr_start,
            commands::auth_qr_poll,
            commands::auth_status,
            commands::auth_logout,
            commands::room_get_info,
            commands::danmaku_start,
            commands::danmaku_stop,
            commands::danmaku_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn main_window_starts_hidden_until_plugin_restores_it() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let main_window = config["app"]["windows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|window| window["label"].as_str() == Some("main"))
            .unwrap();

        assert_eq!(main_window["visible"].as_bool(), Some(false));
    }
}
