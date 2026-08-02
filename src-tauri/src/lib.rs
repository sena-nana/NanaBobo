mod bilibili;
mod commands;
mod credential_store;
mod models;

use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = bilibili::BilibiliClient::new().expect("failed to initialize HTTP client");
    let credentials = Arc::new(credential_store::KeyringCredentialStore::new(
        "com.senanana.nanabobo",
    ));
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_lilia::init())
        .manage(commands::AppState::new(client, credentials))
        .invoke_handler(tauri::generate_handler![
            commands::auth_qr_start,
            commands::auth_qr_poll,
            commands::auth_status,
            commands::auth_logout,
            commands::room_get_info,
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
