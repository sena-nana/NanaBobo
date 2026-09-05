//! Typed business events with an adapter for legacy named consumers.
use crate::models::{DanmakuMessage, DanmakuStatus};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum CoreEvent {
    DanmakuStatus(DanmakuStatus),
    DanmakuMessage(DanmakuMessage),
}

pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, _event: &str, _payload: Value) {}
    fn emit_typed(&self, event: CoreEvent) {
        let (name, payload) = match event {
            CoreEvent::DanmakuStatus(value) => (
                crate::bilibili::DANMAKU_STATUS_EVENT,
                serde_json::to_value(value),
            ),
            CoreEvent::DanmakuMessage(value) => (
                crate::bilibili::DANMAKU_MESSAGE_EVENT,
                serde_json::to_value(value),
            ),
        };
        if let Ok(payload) = payload {
            self.emit(name, payload);
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEventSink;
impl EventSink for NullEventSink {}
