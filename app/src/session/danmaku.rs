use super::*;
use nanabobo_core::models::DanmakuConnectionState;
use std::collections::VecDeque;
pub struct MessageRow {
    pub id: u64,
    pub message: DanmakuMessage,
}
pub struct DanmakuState {
    pub status: DanmakuStatus,
    pub messages: VecDeque<MessageRow>,
    pub following: bool,
    pub unread: usize,
    pub scroll_offset: f32,
    next_id: u64,
}
impl Default for DanmakuState {
    fn default() -> Self {
        Self {
            status: DanmakuStatus {
                connection_id: None,
                room_id: None,
                state: DanmakuConnectionState::Idle,
                message: None,
            },
            messages: VecDeque::new(),
            following: true,
            unread: 0,
            scroll_offset: 0.0,
            next_id: 0,
        }
    }
}
impl DanmakuState {
    pub fn active(&self) -> bool {
        self.status.connection_id.is_some()
    }
    pub fn clear(&mut self) {
        self.messages.clear();
        self.following = true;
        self.unread = 0;
        self.scroll_offset = 0.0;
    }
}
impl Session {
    pub(super) fn start_danmaku(&mut self) {
        if !self.authenticated() || self.danmaku.active() || !self.desktop.is_open() {
            return;
        }
        let Some(room_id) = self.room.info.as_ref().map(|r| r.room_id) else {
            return;
        };
        #[cfg(test)]
        let result = if self.test_mode {
            Ok(nanabobo_core::models::DanmakuConnection {
                connection_id: format!("test-{}-{room_id}", self.desktop.generation),
                room_id,
            })
        } else {
            commands::danmaku_start(&self.core, room_id)
        };
        #[cfg(not(test))]
        let result = commands::danmaku_start(&self.core, room_id);
        match result {
            Ok(connection) => {
                self.danmaku.status = DanmakuStatus {
                    connection_id: Some(connection.connection_id),
                    room_id: Some(room_id),
                    state: DanmakuConnectionState::Connecting,
                    message: None,
                };
            }
            Err(e) => {
                self.danmaku.status.state = DanmakuConnectionState::Error;
                self.danmaku.status.message = Some(e.message);
            }
        }
        self.revisions.messages += 1;
    }
    pub(super) fn stop_danmaku(&mut self) {
        if let Some(id) = self.danmaku.status.connection_id.take() {
            let _ = commands::danmaku_stop(&self.core, id);
        }
        self.danmaku.status = DanmakuStatus {
            connection_id: None,
            room_id: self.room.info.as_ref().map(|r| r.room_id),
            state: DanmakuConnectionState::Stopped,
            message: None,
        };
        self.revisions.messages += 1;
    }
    pub(super) fn on_danmaku_status(&mut self, status: DanmakuStatus) {
        if status.connection_id.is_none()
            || status.connection_id != self.danmaku.status.connection_id
            || status.room_id != self.room.info.as_ref().map(|r| r.room_id)
        {
            return;
        }
        self.danmaku.status = status;
        self.revisions.messages += 1;
    }
    pub(super) fn on_danmaku_message(&mut self, message: DanmakuMessage) {
        if self.danmaku.status.connection_id.as_deref() != Some(message.connection_id.as_str())
            || self.room.info.as_ref().map(|r| r.room_id) != Some(message.room_id)
        {
            return;
        }
        self.danmaku.next_id += 1;
        if self.danmaku.messages.len() == 1000 {
            self.danmaku.messages.pop_front();
        }
        self.danmaku.messages.push_back(MessageRow {
            id: self.danmaku.next_id,
            message,
        });
        if !self.danmaku.following {
            self.danmaku.unread += 1;
        }
        self.revisions.messages += 1;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_connection_messages_are_rejected_and_stop_keeps_reading_history() {
        let mut s = Session::for_test();
        s.room.info = Some(RoomInfo {
            room_id: 2,
            owner_id: 1,
            title: "test".into(),
            owner_name: None,
            owner_avatar_url: None,
            live_status: LiveStatus::Live,
            viewer_count: 1,
            follower_count: None,
            cover_url: None,
            fetched_at: 0,
        });
        s.danmaku.status.connection_id = Some("new".into());
        let mut message = DanmakuMessage {
            connection_id: "old".into(),
            room_id: 2,
            sender_name: "A".into(),
            text: "hello".into(),
            sent_at: 1,
        };
        s.on_danmaku_message(message.clone());
        assert!(s.danmaku.messages.is_empty());
        message.connection_id = "new".into();
        s.on_danmaku_message(message.clone());
        let id = s.danmaku.messages[0].id;
        s.stop_danmaku();
        s.on_danmaku_message(message);
        assert_eq!(s.danmaku.messages.len(), 1);
        assert_eq!(s.danmaku.messages[0].id, id);
    }
}
