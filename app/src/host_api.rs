//! JS 经 `Nana.host` 调用的宿主能力注册表。
//!
//! 对应迁移前 Tauri command 的职责:结构化输入、结构化输出、可恢复错误;
//! 错误以 `{ code, message }` 形态抛给 JS,不含原始上游响应或敏感凭据。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use nanabobo_core::bilibili::BilibiliClient;
use nanabobo_core::commands as core;
use nanabobo_core::commands::{AppError, AppState};
use nanabobo_core::credential_store::KeyringCredentialStore;
use nanabobo_core::events::EventSink;
use nana_js_engine::{HostApiRegistry, HostCompletion, HostEventSender, HostValue, JsException};
use serde::Serialize;

const STORAGE_FILE_NAME: &str = "nanabobo-storage.json";
const CREDENTIAL_SERVICE: &str = "com.senanana.nanabobo";

/// 把核心层事件转成 JS 事件;事件名沿用 `nanabobo://` 约定。
struct NanaEventSink(HostEventSender);

impl EventSink for NanaEventSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        self.0.send(event, HostValue::from_json_value(payload));
    }
}

fn tokio_handle() -> tokio::runtime::Handle {
    static TOKIO: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    TOKIO
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("创建 tokio 运行时")
        })
        .handle()
        .clone()
}

pub fn registry(events: HostEventSender) -> HostApiRegistry {
    let mut api = HostApiRegistry::new();
    install_storage(&mut api);
    install_business(&mut api, events.clone());
    install_spike_probes(&mut api, events);
    api
}

/// 最近一次 spike_echo 的内容,供无头测试诊断 JS 侧状态。
static LAST_PROBE: OnceLock<Mutex<String>> = OnceLock::new();

fn record_probe(message: &str) {
    LAST_PROBE
        .get_or_init(|| Mutex::new(String::new()))
        .lock()
        .map(|mut slot| *slot = message.to_owned())
        .ok();
}

/// 迁移期冒烟探针:随阶段 2.4 的 spike 页面一起删除。
fn install_spike_probes(api: &mut HostApiRegistry, events: HostEventSender) {
    api.register("spike_echo", move |args| {
        let message = arg_string(args, 0)?;
        record_probe(&message);
        Ok(HostValue::String(format!("echo: {message}")))
    });
    api.register("spike_ping_event", move |_args| {
        events.send("spike:pong", HostValue::String("来自 Rust 的事件".into()));
        Ok(HostValue::Null)
    });
}

/// 最近一次 spike_echo 的内容,供无头测试诊断 JS 侧状态。
pub fn probe_last() -> String {
    LAST_PROBE
        .get_or_init(|| Mutex::new(String::new()))
        .lock()
        .map(|slot| slot.clone())
        .unwrap_or_default()
}

fn install_business(api: &mut HostApiRegistry, events: HostEventSender) {
    let client = BilibiliClient::new().expect("初始化 B 站客户端");
    let credentials = Arc::new(KeyringCredentialStore::new(CREDENTIAL_SERVICE));
    let state = Arc::new(AppState::new(
        client,
        credentials,
        tokio_handle(),
        Arc::new(NanaEventSink(events)),
    ));

    {
        let state = Arc::clone(&state);
        api.register_async("auth_qr_start", move |_args, context| {
            let state = Arc::clone(&state);
            let (completion, pending) = context.pending();
            tokio_handle().spawn(async move {
                complete_with(completion, core::auth_qr_start(&state).await);
            });
            Ok(pending)
        });
    }
    {
        let state = Arc::clone(&state);
        api.register_async("auth_qr_poll", move |args, context| {
            let session_id = arg_field_string(&args, "sessionId")?;
            let state = Arc::clone(&state);
            let (completion, pending) = context.pending();
            tokio_handle().spawn(async move {
                complete_with(completion, core::auth_qr_poll(&state, session_id).await);
            });
            Ok(pending)
        });
    }
    {
        let state = Arc::clone(&state);
        api.register_async("auth_status", move |_args, context| {
            let state = Arc::clone(&state);
            let (completion, pending) = context.pending();
            tokio_handle().spawn(async move {
                complete_with(completion, core::auth_status(&state).await);
            });
            Ok(pending)
        });
    }
    {
        let state = Arc::clone(&state);
        api.register("auth_logout", move |_args| {
            result_to_host(core::auth_logout(&state))
        });
    }
    {
        let state = Arc::clone(&state);
        api.register_async("room_get_info", move |args, context| {
            let room_id = arg_field_string(&args, "roomId")?;
            let state = Arc::clone(&state);
            let (completion, pending) = context.pending();
            tokio_handle().spawn(async move {
                complete_with(completion, core::room_get_info(&state, room_id).await);
            });
            Ok(pending)
        });
    }
    {
        let state = Arc::clone(&state);
        api.register("danmaku_start", move |args| {
            let room_id = arg_field_u64(&args, "roomId")?;
            result_to_host(core::danmaku_start(&state, room_id))
        });
    }
    {
        let state = Arc::clone(&state);
        api.register("danmaku_stop", move |args| {
            let connection_id = arg_field_string(&args, "connectionId")?;
            result_to_host(core::danmaku_stop(&state, connection_id))
        });
    }
    {
        let state = Arc::clone(&state);
        api.register("danmaku_status", move |_args| {
            result_to_host(core::danmaku_status(&state))
        });
    }
}

fn complete_with<T: Serialize>(completion: HostCompletion, result: Result<T, AppError>) {
    completion.complete(result_to_host(result));
}

fn result_to_host<T: Serialize>(result: Result<T, AppError>) -> Result<HostValue, JsException> {
    let value = result.map_err(error_into_exception)?;
    to_host(value)
}

fn to_host<T: Serialize>(value: T) -> Result<HostValue, JsException> {
    let json = serde_json::to_value(value)
        .map_err(|_| JsException::new("命令结果序列化失败"))?;
    Ok(HostValue::from_json_value(json))
}

fn error_into_exception(error: AppError) -> JsException {
    let code = serde_json::to_value(&error.code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned));
    let exception = JsException::new(error.message);
    match code {
        Some(code) => exception.with_code(code),
        None => exception,
    }
}

fn arg_payload(args: &[HostValue]) -> Result<&BTreeMap<String, HostValue>, JsException> {
    match args.first() {
        Some(HostValue::Object(fields)) => Ok(fields),
        Some(_) | None => Err(JsException::new("命令参数格式不正确")),
    }
}

fn arg_field_string(args: &[HostValue], field: &str) -> Result<String, JsException> {
    match arg_payload(args)?.get(field) {
        Some(HostValue::String(value)) => Ok(value.clone()),
        Some(other) => Ok(other.to_json_value().to_string()),
        None => Err(JsException::new(format!("缺少参数 {field}"))),
    }
}

fn arg_field_u64(args: &[HostValue], field: &str) -> Result<u64, JsException> {
    match arg_payload(args)?.get(field) {
        Some(HostValue::Number(value)) if *value >= 0.0 => Ok(*value as u64),
        Some(HostValue::String(value)) => value
            .trim()
            .parse::<u64>()
            .map_err(|_| JsException::new(format!("参数 {field} 不是有效数字"))),
        _ => Err(JsException::new(format!("缺少参数 {field}"))),
    }
}

fn install_storage(api: &mut HostApiRegistry) {
    let store = Arc::new(Mutex::new(KeyValueStore::open()));

    {
        let store = Arc::clone(&store);
        api.register("storage_load", move |_args| {
            let store = store.lock().map_err(|_| poisoned())?;
            Ok(store.to_host_value())
        });
    }
    {
        let store = Arc::clone(&store);
        api.register("storage_save", move |args| {
            let key = arg_string(&args, 0)?;
            let value = arg_string(&args, 1)?;
            let mut store = store.lock().map_err(|_| poisoned())?;
            store.set(key, value)?;
            Ok(HostValue::Null)
        });
    }
    {
        let store = Arc::clone(&store);
        api.register("storage_remove", move |args| {
            let key = arg_string(&args, 0)?;
            let mut store = store.lock().map_err(|_| poisoned())?;
            store.remove(key)?;
            Ok(HostValue::Null)
        });
    }
    {
        let store = Arc::clone(&store);
        api.register("storage_clear", move |_args| {
            let mut store = store.lock().map_err(|_| poisoned())?;
            store.clear()?;
            Ok(HostValue::Null)
        });
    }
}

fn poisoned() -> JsException {
    JsException::new("宿主存储状态不可用")
}

fn arg_string(args: &[HostValue], index: usize) -> Result<String, JsException> {
    match args.get(index) {
        Some(HostValue::String(value)) => Ok(value.clone()),
        Some(other) => Ok(other.to_json_value().to_string()),
        None => Err(JsException::new(format!(
            "缺少第 {} 个字符串参数",
            index + 1
        ))),
    }
}

/// localStorage 的文件背书:键值整体落盘,与 exe 同目录。
struct KeyValueStore {
    path: PathBuf,
    entries: BTreeMap<String, String>,
}

impl KeyValueStore {
    fn open() -> Self {
        let path = storage_path();
        let entries = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self { path, entries }
    }

    fn set(&mut self, key: String, value: String) -> Result<(), JsException> {
        self.entries.insert(key, value);
        self.flush()
    }

    fn remove(&mut self, key: String) -> Result<(), JsException> {
        self.entries.remove(&key);
        self.flush()
    }

    fn clear(&mut self) -> Result<(), JsException> {
        self.entries.clear();
        self.flush()
    }

    fn to_host_value(&self) -> HostValue {
        HostValue::Object(
            self.entries
                .iter()
                .map(|(key, value)| (key.clone(), HostValue::String(value.clone())))
                .collect(),
        )
    }

    fn flush(&self) -> Result<(), JsException> {
        let data = serde_json::to_vec_pretty(&self.entries)
            .map_err(|_| JsException::new("存储序列化失败"))?;
        std::fs::write(&self.path, data).map_err(|_| JsException::new("存储写入失败"))?;
        Ok(())
    }
}

fn storage_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(STORAGE_FILE_NAME)
}
