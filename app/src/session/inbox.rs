use super::AppEvent;
use nanabobo_core::events::{CoreEvent, EventSink};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
type Dispatch = Arc<dyn Fn() + Send + Sync>;
#[derive(Default)]
struct Queues {
    controls: VecDeque<AppEvent>,
    messages: VecDeque<AppEvent>,
}
#[derive(Clone, Default)]
pub struct Inbox {
    queues: Arc<Mutex<Queues>>,
    dispatch: Arc<Mutex<Option<Dispatch>>>,
}
impl Inbox {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn bind(&self, dispatch: Dispatch) {
        *self.dispatch.lock().expect("dispatch") = Some(dispatch);
    }
    fn wake(&self) {
        let fire = self.dispatch.lock().expect("dispatch").clone();
        if let Some(fire) = fire {
            fire();
        }
    }
    pub fn push(&self, event: AppEvent) {
        let notify = {
            let mut q = self.queues.lock().expect("inbox");
            let empty = q.controls.is_empty() && q.messages.is_empty();
            if matches!(event, AppEvent::DanmakuMessage(_)) {
                if q.messages.len() == 1_000 {
                    q.messages.pop_front();
                }
                q.messages.push_back(event);
            } else {
                q.controls.push_back(event);
            }
            empty
        };
        if notify {
            self.wake();
        }
    }
    pub fn drain(&self) -> Vec<AppEvent> {
        let (events, more) = {
            let mut q = self.queues.lock().expect("inbox");
            let mut events: Vec<_> = q.controls.drain(..).collect();
            let count = q.messages.len().min(200);
            events.extend(q.messages.drain(..count));
            (events, !q.messages.is_empty())
        };
        if more {
            self.wake();
        }
        events
    }
}
impl EventSink for Inbox {
    fn emit_typed(&self, event: CoreEvent) {
        self.push(match event {
            CoreEvent::DanmakuStatus(s) => AppEvent::DanmakuStatus(s),
            CoreEvent::DanmakuMessage(m) => AppEvent::DanmakuMessage(m),
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use nanabobo_core::models::DanmakuMessage;
    #[test]
    fn bounded_messages_never_delay_control_and_drain_in_batches() {
        let inbox = Inbox::new();
        for n in 0..2_000 {
            inbox.push(AppEvent::DanmakuMessage(DanmakuMessage {
                connection_id: "c".into(),
                room_id: 1,
                sender_name: "a".into(),
                text: n.to_string(),
                sent_at: n,
            }));
        }
        inbox.push(AppEvent::Logout);
        let batch = inbox.drain();
        assert!(matches!(batch[0], AppEvent::Logout));
        assert_eq!(batch.len(), 201);
        assert!(matches!(&batch[1], AppEvent::DanmakuMessage(m) if m.sent_at == 1_000));
        assert_eq!((0..4).map(|_| inbox.drain().len()).sum::<usize>(), 800);
        assert!(inbox.drain().is_empty());
    }
}
