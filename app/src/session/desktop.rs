use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopDanmakuSettings {
    pub width: f32,
    pub height: f32,
    pub position: Option<[f32; 2]>,
    pub font_size: f32,
    pub background_opacity: f32,
}

impl Default for DesktopDanmakuSettings {
    fn default() -> Self {
        Self {
            width: 420.0,
            height: 640.0,
            position: None,
            font_size: 18.0,
            background_opacity: 0.0,
        }
    }
}
impl DesktopDanmakuSettings {
    pub fn normalize(&mut self) {
        self.width = finite_clamp(self.width, 420.0, 280.0, 8192.0);
        self.height = finite_clamp(self.height, 640.0, 240.0, 8192.0);
        self.font_size = finite_clamp(self.font_size, 18.0, 12.0, 36.0);
        self.background_opacity = finite_clamp(self.background_opacity, 0.0, 0.0, 1.0);
        if self
            .position
            .is_some_and(|p| p.iter().any(|v| !v.is_finite()))
        {
            self.position = None;
        }
    }
}
fn finite_clamp(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DesktopDanmakuPhase {
    #[default]
    Closed,
    Creating,
    Adjusting,
    Locked,
}
#[derive(Debug, Clone)]
pub enum DesktopEffect {
    Open {
        generation: u64,
        settings: DesktopDanmakuSettings,
    },
    Close {
        generation: u64,
    },
    Focus {
        generation: u64,
    },
    SetPassthrough {
        generation: u64,
        request: u64,
        enabled: bool,
    },
}
#[derive(Default)]
pub struct DesktopDanmakuState {
    pub phase: DesktopDanmakuPhase,
    pub generation: u64,
    pub settings: DesktopDanmakuSettings,
    pub error: Option<String>,
    effects: Vec<DesktopEffect>,
    request: u64,
    pending_passthrough: Option<(u64, bool)>,
}
impl DesktopDanmakuState {
    pub fn new(mut settings: DesktopDanmakuSettings) -> Self {
        settings.normalize();
        Self {
            settings,
            ..Self::default()
        }
    }
    pub fn is_open(&self) -> bool {
        matches!(
            self.phase,
            DesktopDanmakuPhase::Adjusting | DesktopDanmakuPhase::Locked
        )
    }
}
impl Session {
    pub fn take_desktop_effects(&mut self) -> Vec<DesktopEffect> {
        std::mem::take(&mut self.desktop.effects)
    }
    pub(super) fn open_desktop(&mut self) {
        if self.desktop.is_open() {
            self.adjust_desktop();
            return;
        }
        if self.desktop.phase == DesktopDanmakuPhase::Creating
            || !self.authenticated()
            || self.room.info.is_none()
        {
            return;
        }
        self.desktop.generation = self.desktop.generation.wrapping_add(1);
        self.desktop.phase = DesktopDanmakuPhase::Creating;
        self.desktop.error = None;
        self.desktop.effects.push(DesktopEffect::Open {
            generation: self.desktop.generation,
            settings: self.desktop.settings.clone(),
        });
        self.revisions.desktop += 1;
    }
    pub(super) fn close_desktop(&mut self) {
        if self.desktop.phase != DesktopDanmakuPhase::Closed {
            self.desktop.effects.push(DesktopEffect::Close {
                generation: self.desktop.generation,
            });
        }
        self.desktop.phase = DesktopDanmakuPhase::Closed;
        self.desktop.pending_passthrough = None;
        self.desktop.error = None;
        self.stop_danmaku();
        self.revisions.desktop += 1;
    }
    pub(super) fn desktop_opened(&mut self, generation: u64) {
        if generation != self.desktop.generation
            || self.desktop.phase != DesktopDanmakuPhase::Creating
        {
            if generation != self.desktop.generation
                || self.desktop.phase == DesktopDanmakuPhase::Closed
            {
                self.desktop
                    .effects
                    .push(DesktopEffect::Close { generation });
            }
            return;
        }
        self.desktop.phase = DesktopDanmakuPhase::Adjusting;
        self.apply(AppEvent::FollowLatest);
        self.start_danmaku();
        self.revisions.desktop += 1;
    }
    pub(super) fn desktop_open_failed(&mut self, generation: u64) {
        if generation != self.desktop.generation
            || self.desktop.phase != DesktopDanmakuPhase::Creating
        {
            return;
        }
        self.desktop.phase = DesktopDanmakuPhase::Closed;
        self.desktop.error = Some("弹幕窗口暂时无法打开，请重试。".into());
        self.revisions.desktop += 1;
    }
    pub(super) fn desktop_closed(&mut self, generation: u64) {
        if generation != self.desktop.generation {
            return;
        }
        self.desktop.phase = DesktopDanmakuPhase::Closed;
        self.desktop.pending_passthrough = None;
        self.stop_danmaku();
        self.revisions.desktop += 1;
    }
    pub(super) fn adjust_desktop(&mut self) {
        if self.desktop.phase == DesktopDanmakuPhase::Locked {
            self.set_desktop_passthrough(false);
        } else if self.desktop.phase == DesktopDanmakuPhase::Adjusting {
            self.desktop.effects.push(DesktopEffect::Focus {
                generation: self.desktop.generation,
            });
        }
    }
    pub(super) fn set_desktop_passthrough(&mut self, enabled: bool) {
        if !self.desktop.is_open()
            || self.desktop.pending_passthrough.is_some()
            || (self.desktop.phase == DesktopDanmakuPhase::Locked) == enabled
        {
            return;
        }
        self.desktop.request = self.desktop.request.wrapping_add(1);
        let request = self.desktop.request;
        self.desktop.pending_passthrough = Some((request, enabled));
        self.desktop.effects.push(DesktopEffect::SetPassthrough {
            generation: self.desktop.generation,
            request,
            enabled,
        });
    }
    pub(super) fn desktop_passthrough_result(
        &mut self,
        generation: u64,
        request: u64,
        enabled: bool,
        success: bool,
    ) {
        if generation != self.desktop.generation
            || self.desktop.pending_passthrough != Some((request, enabled))
            || !self.desktop.is_open()
        {
            return;
        }
        self.desktop.pending_passthrough = None;
        if success {
            self.desktop.error = None;
            self.desktop.phase = if enabled {
                DesktopDanmakuPhase::Locked
            } else {
                DesktopDanmakuPhase::Adjusting
            };
            if enabled {
                self.apply(AppEvent::FollowLatest);
            } else {
                self.desktop
                    .effects
                    .push(DesktopEffect::Focus { generation });
            }
        } else {
            self.desktop.error = Some(
                if enabled {
                    "暂时无法锁定弹幕，请重试。"
                } else {
                    "暂时无法调整弹幕，请重试或关闭后重新打开。"
                }
                .into(),
            );
        }
        self.revisions.desktop += 1;
    }
    pub(super) fn set_desktop_font_size(&mut self, value: f32) {
        if !value.is_finite() {
            return;
        }
        self.desktop.settings.font_size = value.clamp(12.0, 36.0);
        self.revisions.desktop += 1;
        self.revisions.messages += 1;
        self.persist(false);
    }
    pub(super) fn set_desktop_opacity(&mut self, value: f32) {
        if !value.is_finite() {
            return;
        }
        self.desktop.settings.background_opacity = value.clamp(0.0, 1.0);
        self.revisions.desktop += 1;
        self.persist(false);
    }
    pub(super) fn desktop_geometry(
        &mut self,
        generation: u64,
        width: f32,
        height: f32,
        position: [f32; 2],
    ) {
        if generation != self.desktop.generation
            || !self.desktop.is_open()
            || !width.is_finite()
            || !height.is_finite()
            || position.iter().any(|v| !v.is_finite())
        {
            return;
        }
        let old = self.desktop.settings.clone();
        self.desktop.settings.width = width;
        self.desktop.settings.height = height;
        self.desktop.settings.position = Some(position);
        self.desktop.settings.normalize();
        if old != self.desktop.settings {
            self.persist(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        let mut s = Session::for_test();
        s.auth.account = Some(AccountStatus {
            authenticated: true,
            account: Some(nanabobo_core::models::AccountSummary {
                mid: 1,
                username: "主播".into(),
                avatar_url: None,
                level: None,
                coins: None,
                bcoin: None,
                vip: None,
            }),
        });
        s.room.info = Some(RoomInfo {
            room_id: 1,
            owner_id: 1,
            owner_name: None,
            owner_avatar_url: None,
            title: "直播".into(),
            live_status: LiveStatus::Live,
            viewer_count: 1,
            follower_count: None,
            cover_url: None,
            fetched_at: 0,
        });
        s
    }
    fn open(s: &mut Session) -> u64 {
        s.apply(AppEvent::OpenDesktopDanmaku);
        let generation = s.desktop.generation;
        s.apply(AppEvent::DesktopOpened { generation });
        s.take_desktop_effects();
        generation
    }
    #[test]
    fn connection_waits_for_native_success_and_duplicate_open_reuses_window() {
        let mut s = session();
        s.apply(AppEvent::OpenDesktopDanmaku);
        s.apply(AppEvent::OpenDesktopDanmaku);
        let generation = s.desktop.generation;
        assert!(!s.danmaku.active());
        assert_eq!(s.take_desktop_effects().len(), 1);
        s.apply(AppEvent::DesktopOpenFailed { generation });
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Closed);
        assert!(!s.danmaku.active());
        let generation = open(&mut s);
        assert!(s.danmaku.active());
        let connection = s.danmaku.status.connection_id.clone();
        s.apply(AppEvent::OpenDesktopDanmaku);
        assert_eq!(s.danmaku.status.connection_id, connection);
        assert!(
            matches!(s.take_desktop_effects().as_slice(), [DesktopEffect::Focus { generation: g }] if *g == generation)
        );
    }
    #[test]
    fn cancelled_creation_cannot_open_or_replace_a_new_window() {
        let mut s = session();
        s.apply(AppEvent::OpenDesktopDanmaku);
        let old = s.desktop.generation;
        s.apply(AppEvent::CloseDesktopDanmaku);
        let new = open(&mut s);
        s.apply(AppEvent::DesktopOpened { generation: old });
        assert!(
            matches!(s.take_desktop_effects().as_slice(), [DesktopEffect::Close { generation }] if *generation == old)
        );
        s.apply(AppEvent::DesktopOpenFailed { generation: old });
        assert_eq!(s.desktop.generation, new);
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Adjusting);
        assert!(s.danmaku.active());
    }
    #[test]
    fn close_keeps_history_rejects_late_events_and_reopen_follows_latest() {
        let mut s = session();
        let old = open(&mut s);
        let message = DanmakuMessage {
            connection_id: s.danmaku.status.connection_id.clone().unwrap(),
            room_id: 1,
            sender_name: "读者".into(),
            text: "你好".into(),
            sent_at: 1,
        };
        s.apply(AppEvent::DanmakuMessage(message.clone()));
        s.apply(AppEvent::Reading {
            following: false,
            offset: 50.0,
        });
        s.apply(AppEvent::CloseDesktopDanmaku);
        assert!(!s.danmaku.active());
        s.apply(AppEvent::DanmakuMessage(message));
        assert_eq!(s.danmaku.messages.len(), 1);
        let new = open(&mut s);
        assert_ne!(old, new);
        s.apply(AppEvent::DesktopClosed { generation: old });
        assert!(s.danmaku.active());
        assert!(s.danmaku.following);
        s.apply(AppEvent::CloseDesktopDanmaku);
        s.apply(AppEvent::DesktopOpened { generation: new });
        assert!(!s.danmaku.active());
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Closed);
    }
    #[test]
    fn lock_commits_only_after_success_and_unlock_focuses_after_success() {
        let mut s = session();
        let generation = open(&mut s);
        s.danmaku.following = false;
        for success in [false, true] {
            s.apply(AppEvent::LockDesktopDanmaku);
            let [DesktopEffect::SetPassthrough { request, .. }] = s.take_desktop_effects()[..]
            else {
                panic!()
            };
            assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Adjusting);
            assert!(!s.danmaku.following);
            s.apply(AppEvent::DesktopPassthroughResult {
                generation,
                request,
                enabled: true,
                success,
            });
        }
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Locked);
        assert!(s.danmaku.following);
        s.apply(AppEvent::Reading {
            following: false,
            offset: 50.0,
        });
        assert!(s.danmaku.following);
        s.apply(AppEvent::AdjustDesktopDanmaku);
        let [DesktopEffect::SetPassthrough {
            request,
            enabled: false,
            ..
        }] = s.take_desktop_effects()[..]
        else {
            panic!()
        };
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Locked);
        s.apply(AppEvent::DesktopPassthroughResult {
            generation,
            request,
            enabled: false,
            success: true,
        });
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Adjusting);
        assert!(matches!(
            s.take_desktop_effects().as_slice(),
            [DesktopEffect::Focus { .. }]
        ));
        s.apply(AppEvent::DisconnectRoom);
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Closed);
        assert!(!s.danmaku.active());
    }
    #[test]
    fn logout_during_creation_closes_window_and_rejects_late_ready() {
        let mut s = session();
        s.apply(AppEvent::OpenDesktopDanmaku);
        let generation = s.desktop.generation;
        s.apply(AppEvent::Logout);
        s.apply(AppEvent::DesktopOpened { generation });
        assert_eq!(s.desktop.phase, DesktopDanmakuPhase::Closed);
        assert!(!s.danmaku.active());
        assert!(!s.authenticated());
        assert!(s.room.info.is_none());
    }
    #[test]
    fn persisted_settings_restore_without_opening_or_locking() {
        let mut s = session();
        let generation = open(&mut s);
        s.apply(AppEvent::DesktopGeometry {
            generation,
            width: 510.0,
            height: 720.0,
            position: [-320.0, 10.0],
        });
        s.apply(AppEvent::DesktopFontSize(24.0));
        s.apply(AppEvent::DesktopBackgroundOpacity(0.35));
        let stored = serde_json::to_vec(&StoredState::from_session(&s)).unwrap();
        let loaded: StoredState = serde_json::from_slice(&stored).unwrap();
        let restored = DesktopDanmakuState::new(loaded.desktop);
        assert_eq!(restored.settings, s.desktop.settings);
        assert_eq!(restored.phase, DesktopDanmakuPhase::Closed);
        assert!(restored.effects.is_empty());
        s.apply(AppEvent::DesktopGeometry {
            generation: generation + 1,
            width: 900.0,
            height: 900.0,
            position: [0.0, 0.0],
        });
        assert_eq!(s.desktop.settings.width, 510.0);
    }
}
