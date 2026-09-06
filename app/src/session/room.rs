use super::*;
use std::time::Duration;
#[derive(Default)]
pub struct RoomState {
    pub input: String,
    pub remembered: String,
    pub info: Option<RoomInfo>,
    pub editing: bool,
    pub(super) request: Request,
    pub(super) next_refresh: Option<Instant>,
    failures: usize,
}
impl RoomState {
    pub fn new(remembered: String) -> Self {
        Self {
            input: remembered.clone(),
            remembered,
            ..Self::default()
        }
    }
    pub fn loading(&self) -> bool {
        self.request.state.loading()
    }
    pub fn error(&self) -> Option<&str> {
        self.request.state.error()
    }
    pub(super) fn schedule_refresh(&mut self) {
        if self.info.is_some() {
            self.next_refresh = Some(Instant::now() + Duration::from_secs(60));
        }
    }
}
impl Session {
    pub(super) fn query_room(&mut self, refresh: bool) {
        if !self.authenticated() {
            return;
        }
        if refresh && self.room.loading() {
            return;
        }
        let raw = if refresh {
            self.room
                .info
                .as_ref()
                .map(|r| r.room_id.to_string())
                .unwrap_or_default()
        } else {
            self.room.input.trim().to_owned()
        };
        if raw.parse::<u64>().ok().filter(|id| *id > 0).is_none() {
            self.room.request.cancel();
            self.room.schedule_refresh();
            self.room.request.state = Operation::Failed("请输入有效的直播间号。".into());
            self.revisions.room += 1;
            return;
        }
        self.room.next_refresh = None;
        let request = self.room.request.begin();
        self.revisions.room += 1;
        #[cfg(test)]
        if self.test_mode {
            return;
        }
        let core = self.core.clone();
        let inbox = self.inbox.clone();
        self.room.request.task = Some(self.runtime.spawn(async move {
            inbox.push(AppEvent::RoomLoaded {
                request,
                refresh,
                result: commands::room_get_info(&core, raw).await,
            });
        }));
    }
    pub(super) fn on_room_loaded(
        &mut self,
        id: u64,
        refresh: bool,
        result: Result<RoomInfo, AppError>,
    ) {
        if !self.room.request.finish(id) || !self.authenticated() {
            return;
        }
        match result {
            Ok(info) => {
                let changed = self.room.info.as_ref().map(|r| r.room_id) != Some(info.room_id);
                if changed {
                    self.stop_danmaku();
                    self.danmaku.clear();
                }
                self.room.remembered = info.room_id.to_string();
                if !refresh {
                    self.room.input = self.room.remembered.clone();
                    self.room.editing = false;
                }
                self.stats.selected_room.get_or_insert(info.room_id);
                if changed {
                    self.stats.selected_room = Some(info.room_id);
                }
                self.record_snapshot(&info);
                let avatar = info.owner_avatar_url.clone();
                let cover = info.cover_url.clone();
                self.room.info = Some(info);
                self.room.failures = 0;
                self.room.schedule_refresh();
                self.request_image(crate::images::ROOM_AVATAR, avatar.as_deref());
                self.request_image(crate::images::ROOM_COVER, cover.as_deref());
                if changed && self.desktop.is_open() {
                    self.start_danmaku();
                }
                self.persist(false);
            }
            Err(error) => {
                self.room.request.state = Operation::Failed(error.message);
                if self.room.info.is_some() {
                    let delay = [60, 120, 240, 300][self.room.failures.min(3)];
                    self.room.failures = self.room.failures.saturating_add(1);
                    self.room.next_refresh = Some(Instant::now() + Duration::from_secs(delay));
                }
            }
        }
        self.revisions.room += 1;
        self.revisions.stats += 1;
    }
    pub(super) fn disconnect_room(&mut self) {
        self.close_desktop();
        self.room.request.cancel();
        self.room.next_refresh = None;
        self.room.info = None;
        self.room.editing = false;
        self.stop_danmaku();
        self.danmaku.clear();
        self.clear_image(crate::images::ROOM_AVATAR);
        self.clear_image(crate::images::ROOM_COVER);
        self.revisions.room += 1;
        self.revisions.messages += 1;
        self.persist(false);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    pub fn logged_in(s: &mut Session) {
        s.auth.account = Some(AccountStatus {
            authenticated: true,
            account: Some(nanabobo_core::models::AccountSummary {
                mid: 1,
                username: "A".into(),
                avatar_url: None,
                level: None,
                coins: None,
                bcoin: None,
                vip: None,
            }),
        });
    }
    pub fn info(id: u64) -> RoomInfo {
        RoomInfo {
            room_id: id,
            owner_id: 2,
            owner_name: Some("主播".into()),
            owner_avatar_url: None,
            title: "直播".into(),
            live_status: LiveStatus::Live,
            viewer_count: 2,
            follower_count: None,
            cover_url: None,
            fetched_at: 100,
        }
    }
    #[test]
    fn old_room_cannot_replace_new_selection_or_restore_after_disconnect() {
        let mut s = Session::for_test();
        logged_in(&mut s);
        s.room.input = "1".into();
        s.query_room(false);
        let old = s.room.request.revision;
        s.room.input = "2".into();
        s.query_room(false);
        let new = s.room.request.revision;
        s.on_room_loaded(old, false, Ok(info(1)));
        assert!(s.room.info.is_none());
        s.on_room_loaded(new, false, Ok(info(2)));
        assert_eq!(s.room.info.as_ref().unwrap().room_id, 2);
        s.query_room(true);
        let id = s.room.request.revision;
        s.disconnect_room();
        s.on_room_loaded(id, true, Ok(info(2)));
        assert!(s.room.info.is_none());
    }
    #[test]
    fn independent_deadlines_are_consumed_and_requests_are_single_flight() {
        let mut s = Session::for_test();
        logged_in(&mut s);
        s.room.info = Some(info(1));
        s.auth.open = true;
        s.auth.qr = Some(QrStartResponse {
            session_id: "qr".into(),
            payload: "url".into(),
            expires_at: 999,
        });
        let now = Instant::now();
        s.auth.next_poll = Some(now);
        s.room.next_refresh = Some(now + Duration::from_secs(60));
        s.advance(now);
        let qr_id = s.auth.request.revision;
        assert!(s.auth.loading());
        assert!(!s.room.loading());
        assert_eq!(s.next_wakeup(), Some(now + Duration::from_secs(60)));
        s.advance(now + Duration::from_secs(60));
        let room_id = s.room.request.revision;
        assert!(s.room.loading());
        assert!(s.next_wakeup().is_none());
        s.advance(now + Duration::from_secs(120));
        assert_eq!(s.auth.request.revision, qr_id);
        assert_eq!(s.room.request.revision, room_id);
        s.on_room_loaded(
            room_id,
            true,
            Err(AppError {
                code: commands::ErrorCode::UpstreamUnavailable,
                message: "暂时不可用".into(),
            }),
        );
        assert!(!s.room.loading());
        assert!(s.room.next_refresh.is_some());
    }
    #[test]
    fn candidate_failure_preserves_current_room() {
        let mut s = Session::for_test();
        logged_in(&mut s);
        s.room.info = Some(info(1));
        s.room.input = "2".into();
        s.query_room(false);
        s.on_room_loaded(
            s.room.request.revision,
            false,
            Err(AppError {
                code: commands::ErrorCode::UpstreamUnavailable,
                message: "暂时不可用".into(),
            }),
        );
        assert_eq!(s.room.info.as_ref().unwrap().room_id, 1);
    }

    #[test]
    fn invalid_submission_during_refresh_does_not_stop_future_sampling() {
        let mut s = Session::for_test();
        logged_in(&mut s);
        s.room.info = Some(info(1));
        s.query_room(true);
        let old = s.room.request.revision;
        s.room.input = "invalid".into();
        s.query_room(false);
        assert!(s.room.next_refresh.is_some());
        s.on_room_loaded(old, true, Ok(info(1)));
        assert!(s.room.error().is_some());
        s.advance(s.room.next_refresh.unwrap());
        assert!(s.room.loading());
    }

    #[test]
    fn selecting_room_stays_disconnected_until_window_opens_then_switch_reconnects() {
        let mut s = Session::for_test();
        logged_in(&mut s);
        s.room.input = "1".into();
        s.query_room(false);
        s.on_room_loaded(s.room.request.revision, false, Ok(info(1)));
        assert!(!s.danmaku.active());
        s.apply(AppEvent::OpenDesktopDanmaku);
        s.apply(AppEvent::DesktopOpened {
            generation: s.desktop.generation,
        });
        assert_eq!(s.danmaku.status.room_id, Some(1));
        let old = s.danmaku.status.connection_id.clone();
        s.room.input = "2".into();
        s.query_room(false);
        s.on_room_loaded(s.room.request.revision, false, Ok(info(2)));
        assert_eq!(s.danmaku.status.room_id, Some(2));
        assert_ne!(s.danmaku.status.connection_id, old);
        s.apply(AppEvent::CloseDesktopDanmaku);
        s.room.input = "3".into();
        s.query_room(false);
        s.on_room_loaded(s.room.request.revision, false, Ok(info(3)));
        assert!(!s.danmaku.active());
    }
}
