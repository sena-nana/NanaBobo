//! 事件出口抽象:核心把弹幕状态与消息推给 UI 层,不关心 UI 层是什么。

use serde_json::Value;

/// 由宿主(Tauri、NanaUI 等)实现;事件名沿用 `nanabobo://` 约定,
/// 前端监听侧契约保持不变。
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: Value);
}

/// 静默出口:测试与无 UI 场景使用。
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn emit(&self, _event: &str, _payload: Value) {}
}
