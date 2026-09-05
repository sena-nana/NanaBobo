//! GPU-backed acceptance checks. Fixtures never load persisted credentials.
use super::{DesktopDanmakuView, Shell};
use crate::session::{AppEvent, Page, Session, Snapshot};
use nana_ui::runtime::{DocumentId, RuntimeDocument};
use nana_ui::ThemeMode;
use nana_ui_devtools::agent::{AccessibilityDumpNode, RuntimeAgentSession};
use nanabobo_core::models::{
    AccountStatus, AccountSummary, DanmakuConnectionState, DanmakuMessage, LiveStatus, RoomInfo,
};
use std::path::PathBuf;

fn fixture() -> Session {
    let mut session = Session::for_test();
    session.auth.account = Some(AccountStatus {
        authenticated: true,
        account: Some(AccountSummary {
            mid: 42,
            username: "星河电台".into(),
            avatar_url: None,
        }),
    });
    session.room.info = Some(RoomInfo {
        room_id: 12345,
        owner_id: 42,
        owner_name: Some("星河电台".into()),
        owner_avatar_url: None,
        title: "周末音乐与故事 · 一起聊聊天".into(),
        live_status: LiveStatus::Live,
        viewer_count: 12580,
        follower_count: Some(32600),
        cover_url: None,
        fetched_at: 1788602400,
    });
    session.room.input = "12345".into();
    session.danmaku.status.connection_id = Some("acceptance".into());
    session.danmaku.status.room_id = Some(12345);
    session.danmaku.status.state = DanmakuConnectionState::Connected;
    for i in 0..36 {
        session.apply(AppEvent::DanmakuMessage(DanmakuMessage { connection_id: "acceptance".into(), room_id: 12345, sender_name: format!("听众 {}", i + 1), text: if i % 5 == 0 { "今天的歌单太棒了！这是一条较长的弹幕，需要在窗口较窄时自然换行，并且可以完整阅读。谢谢主播一直以来的陪伴。".into() } else { "晚上好，今天也来听歌了 🌙".into() }, sent_at: 1788602400 + i }));
    }
    session.stats.selected_room = Some(12345);
    session.stats.snapshots = (0..12)
        .map(|i| Snapshot {
            room_id: 12345,
            room_name: "星河电台".into(),
            captured_at: 1788601800 + i * 60,
            viewer_count: 10000 + i * 200,
            follower_count: if i == 5 { None } else { Some(32500 + i * 10) },
            live_status: LiveStatus::Live,
        })
        .collect();
    session
}

struct Harness {
    state: Session,
    shell: Option<Shell>,
    desktop: Option<DesktopDanmakuView>,
    agent: RuntimeAgentSession,
    clock: std::time::Duration,
}
impl Harness {
    fn new(state: Session, width: u32, height: u32, scale: f32) -> Self {
        let mut document = RuntimeDocument::new(DocumentId::new(1).unwrap());
        let shell = Shell::mount(&mut document, &state).expect("mount real application shell");
        let agent = RuntimeAgentSession::new_scaled(document, width, height, scale)
            .expect("layout application");
        let mut harness = Self {
            state,
            shell: Some(shell),
            desktop: None,
            agent,
            clock: std::time::Duration::ZERO,
        };
        harness.sync();
        harness
    }
    fn sync(&mut self) {
        for event in self.state.inbox.drain() {
            self.state.apply(event);
        }
        self.sync_view();
        self.agent.flush().expect("flush UI");
        for _ in 0..8 {
            if !self.needs_layout_sync() {
                return;
            }
            self.sync_view();
            self.agent.flush().expect("layout measured rows");
        }
        assert!(
            !self.needs_layout_sync(),
            "virtual row measurement must converge: {:#?}",
            self.agent.accessibility_dump()
        );
    }
    fn new_desktop(mut state: Session, width: u32, height: u32, scale: f32) -> Self {
        state.desktop.phase = crate::session::DesktopDanmakuPhase::Adjusting;
        let mut document = RuntimeDocument::new(DocumentId::new(2).unwrap());
        let desktop = DesktopDanmakuView::mount(&mut document, &state).unwrap();
        let agent = RuntimeAgentSession::new_scaled(document, width, height, scale).unwrap();
        let mut h = Self {
            state,
            shell: None,
            desktop: Some(desktop),
            agent,
            clock: std::time::Duration::ZERO,
        };
        h.sync();
        h
    }
    fn sync_view(&mut self) {
        if let Some(shell) = &mut self.shell {
            shell.sync(self.agent.document_mut(), &self.state).unwrap();
        }
        if let Some(desktop) = &mut self.desktop {
            desktop
                .sync(self.agent.document_mut(), &self.state)
                .unwrap();
        }
    }
    fn needs_layout_sync(&self) -> bool {
        self.shell
            .as_ref()
            .is_some_and(|s| s.needs_layout_sync(self.agent.document()))
            || self
                .desktop
                .as_ref()
                .is_some_and(|s| s.needs_layout_sync(self.agent.document()))
    }
    fn node(&self, label: &str) -> AccessibilityDumpNode {
        self.agent
            .accessibility_dump()
            .into_iter()
            .find(|n| n.label.as_deref() == Some(label) && !n.disabled)
            .unwrap_or_else(|| {
                panic!(
                    "reachable control {label}; tree={:#?}",
                    self.agent.accessibility_dump()
                )
            })
    }
    fn click(&mut self, label: &str) {
        let node = self.node(label);
        assert!(
            node.bounds.width > 8.0 && node.bounds.height > 8.0,
            "control has visible geometry: {label}"
        );
        self.agent
            .click_xy(
                node.bounds.x + node.bounds.width * 0.5,
                node.bounds.y + node.bounds.height * 0.5,
            )
            .expect("pointer click");
        self.sync();
    }
    fn screenshot(&mut self, name: &str, width: u32, height: u32) {
        self.clock += std::time::Duration::from_secs(1);
        self.agent
            .document_mut()
            .context_mut()
            .advance_animations(self.clock);
        self.agent.flush().expect("settle component animations");
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/ui-acceptance");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{name}.png"));
        self.agent
            .screenshot_png(&path)
            .expect("render GPU snapshot");
        let png = image::open(&path).expect("decode saved PNG").to_rgba8();
        assert_eq!(png.dimensions(), (width, height));
        let first = png.get_pixel(0, 0);
        let differing = png.pixels().filter(|pixel| *pixel != first).count();
        assert!(
            differing > (width * height / 100) as usize,
            "snapshot must contain rendered product content"
        );
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&self.agent.accessibility_dump()).unwrap(),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires a GPU adapter; generates visual acceptance artifacts"]
fn workbench_gpu_snapshots_at_multiple_sizes_themes_and_dpi() {
    for (width, height, scale, name) in [
        (960, 600, 1.0, "compact"),
        (1200, 800, 1.0, "desktop"),
        (1200, 800, 2.0, "desktop-2x"),
    ] {
        for theme in [ThemeMode::Light, ThemeMode::Dark] {
            let mut state = fixture();
            state.theme = theme;
            let mut h = Harness::new(state, width, height, scale);
            h.screenshot(
                &format!(
                    "workbench-{name}-{}",
                    if theme == ThemeMode::Light {
                        "light"
                    } else {
                        "dark"
                    }
                ),
                (width as f32 * scale) as u32,
                (height as f32 * scale) as u32,
            );
        }
    }
}

#[test]
#[ignore = "requires a GPU adapter; generates visual acceptance artifacts"]
fn navigation_and_room_edit_use_real_pointer_and_keyboard() {
    let mut h = Harness::new(fixture(), 1200, 800, 1.0);
    h.click("设置");
    assert_eq!(h.state.page, Page::Settings);
    h.screenshot("settings", 1200, 800);
    h.click("概览");
    assert_eq!(h.state.page, Page::Workbench);
    h.click("切换直播间");
    assert!(h.state.room.editing);
    for label in ["直播间号", "连接直播间", "取消"] {
        let bounds = h.node(label).bounds;
        assert!(
            bounds.x >= 260.0 && bounds.x + bounds.width <= 1200.0,
            "room form control outside viewport: {label}"
        );
    }
    h.click("直播间号");
    h.agent.type_text("6").expect("native text input");
    h.sync();
    assert!(h.state.room.input.contains('6'));
    let document_id = h.agent.document().document();
    nana_ui::RuntimeInputAdapter::default()
        .dispatch(
            h.agent.document_mut().context_mut(),
            document_id,
            &nana_ui_platform::InputEvent::Keyboard {
                pressed: true,
                key: "Enter".into(),
                text: None,
                code: "Enter".into(),
                repeat: false,
                modifiers: nana_ui_platform::InputModifiers::default(),
            },
        )
        .expect("submit by Enter");
    h.sync();
    assert!(
        h.state.room.loading(),
        "Enter starts the same room lookup as the button"
    );
    h.screenshot("room-edit", 1200, 800);
}
#[test]
#[ignore = "requires a GPU adapter; generates visual acceptance artifacts"]
fn data_history_and_login_have_real_layout_and_actions() {
    let mut h = Harness::new(fixture(), 1200, 800, 1.0);
    h.click("数据");
    assert_eq!(h.state.page, Page::Stats);
    h.screenshot("data-trend", 1200, 800);
    h.click("历史");
    assert_eq!(h.state.stats.tab, crate::session::StatsTab::History);
    h.screenshot("data-history", 1200, 800);
    h.click("清理记录");
    assert_eq!(h.state.stats.confirm_clear, Some(12345));
    h.screenshot("history-clear-confirm", 1200, 800);
    h.click("取消");
    assert!(h.state.stats.confirm_clear.is_none());
}

#[test]
#[ignore = "requires a GPU adapter; generates visual acceptance artifacts"]
fn login_qr_and_expiration_are_visible() {
    let mut h = Harness::new(Session::for_test(), 960, 600, 1.0);
    h.screenshot("guest", 960, 600);
    h.click("登录 B 站");
    assert!(h.state.auth.open);
    h.state.auth.qr = Some(nanabobo_core::models::QrStartResponse {
        session_id: "acceptance-only".into(),
        payload: "https://example.invalid/acceptance-login".into(),
        expires_at: u64::MAX,
    });
    h.state.auth.phase = crate::session::QrPhase::Pending;
    h.state.revisions.overlay += 1;
    h.sync();
    let qr = h.node("B站登录二维码");
    assert!(qr.bounds.width >= 200.0 && qr.bounds.height >= 200.0);
    assert!(h
        .agent
        .accessibility_dump()
        .iter()
        .any(|n| n.role == "dialog"));
    h.screenshot("login-qr", 960, 600);
    h.state.auth.phase = crate::session::QrPhase::Expired;
    h.state.revisions.overlay += 1;
    h.sync();
    h.screenshot("login-expired", 960, 600);
}
impl Harness {
    fn wheel_up(&mut self) {
        let viewport = self.node("实时弹幕").bounds;
        let document = self.agent.document().document();
        nana_ui::RuntimeInputAdapter::default()
            .dispatch(
                self.agent.document_mut().context_mut(),
                document,
                &nana_ui_platform::InputEvent::Wheel {
                    x: viewport.x + viewport.width * 0.5,
                    y: viewport.y + viewport.height * 0.5,
                    delta_x: 0.0,
                    delta_y: 240.0,
                    line_delta: false,
                    modifiers: nana_ui_platform::InputModifiers::default(),
                },
            )
            .unwrap();
        self.sync();
    }
    fn reading_anchor(&self) -> (String, f32) {
        let viewport = self.node("实时弹幕").bounds;
        let node = self
            .agent
            .accessibility_dump()
            .into_iter()
            .find(|node| {
                node.label
                    .as_deref()
                    .is_some_and(|label| label.starts_with("听众 "))
                    && node.bounds.y >= viewport.y + 10.0
                    && node.bounds.y + node.bounds.height < viewport.y + viewport.height - 20.0
            })
            .expect("at least one complete visible message");
        (node.label.clone().unwrap(), node.bounds.y - viewport.y)
    }
    fn assert_anchor(&self, anchor: &(String, f32)) {
        let viewport = self.node("实时弹幕").bounds;
        let node = self.node(&anchor.0);
        let actual = node.bounds.y - viewport.y;
        assert!(
            (actual - anchor.1).abs() <= 2.0,
            "reading anchor moved: {} from {} to {}",
            anchor.0,
            anchor.1,
            actual
        );
    }
    fn append(&mut self, index: u64) {
        self.state.apply(AppEvent::DanmakuMessage(DanmakuMessage {
            connection_id: "acceptance".into(),
            room_id: 12345,
            sender_name: format!("听众 {}", index + 1),
            text: format!("新消息 {index}，保持当前阅读位置。"),
            sent_at: 1788602400 + index,
        }));
    }
}

#[test]
#[ignore = "requires a GPU adapter; generates visual acceptance artifacts"]
fn reading_anchor_survives_messages_navigation_and_buffer_eviction() {
    let mut h = Harness::new_desktop(fixture(), 420, 640, 1.0);
    for i in 36..1000 {
        h.append(i);
    }
    h.sync();
    assert_eq!(h.state.danmaku.messages.len(), 1000);
    h.wheel_up();
    assert!(!h.state.danmaku.following);
    let anchor = h.reading_anchor();
    h.append(1000);
    h.sync();
    assert_eq!(h.state.danmaku.messages.len(), 1000);
    assert!(h.state.danmaku.unread > 0);
    h.screenshot("paused-feed-after-eviction", 420, 640);
    h.assert_anchor(&anchor);
    h.state.apply(AppEvent::Navigate(Page::Stats));
    h.sync();
    h.assert_anchor(&anchor);
    h.state.apply(AppEvent::FollowLatest);
    h.sync();
    assert!(h.state.danmaku.following);
    assert_eq!(h.state.danmaku.unread, 0);
    h.screenshot("feed-returned-to-latest", 420, 640);
    let newest = h
        .state
        .danmaku
        .messages
        .back()
        .unwrap()
        .message
        .sender_name
        .clone();
    let viewport = h.node("实时弹幕").bounds;
    assert!(h.agent.accessibility_dump().iter().any(|node| node
        .label
        .as_deref()
        .is_some_and(|label| label.starts_with(&format!("{newest} ")))
        && node.bounds.y >= viewport.y
        && node.bounds.y < viewport.y + viewport.height));
    h.screenshot("feed-returned-to-latest", 420, 640);
}

#[test]
#[ignore = "requires a GPU adapter; generates visual acceptance artifacts"]
fn long_room_names_and_message_senders_fit_compact_window() {
    let mut state = fixture();
    state.room.info.as_mut().unwrap().title = "非常长的直播间标题：今晚一起听音乐、分享故事、聊天，并且让窄窗口中的内容保持自然可读，不要覆盖操作按钮".into();
    state.room.info.as_mut().unwrap().owner_name =
        Some("一个很长很长的主播昵称·星河音乐电台".into());
    state
        .danmaku
        .messages
        .back_mut()
        .unwrap()
        .message
        .sender_name = "一个很长很长的弹幕发送者昵称·来自遥远星球的听众".into();
    let mut h = Harness::new(state, 960, 600, 1.0);
    h.screenshot("compact-long-content", 960, 600);
    for node in h.agent.accessibility_dump() {
        if node.bounds.width > 0.0 && node.bounds.y >= 36.0 && node.bounds.y < 600.0 {
            assert!(
                node.bounds.x + node.bounds.width <= 961.0,
                "visible content exceeds window: {:?}",
                node.label
            );
        }
    }
}

#[test]
#[ignore = "requires a GPU adapter; generates visual acceptance artifacts"]
fn desktop_controls_and_compact_layout() {
    for (width, height, scale) in [(280, 240, 1.0), (420, 640, 1.0), (420, 640, 2.0)] {
        let mut h = Harness::new_desktop(fixture(), width, height, scale);
        h.click("A+");
        assert_eq!(h.state.desktop.settings.font_size, 20.0);
        h.click("背景 0%");
        assert_eq!(h.state.desktop.settings.background_opacity, 0.25);
        for label in ["锁定", "关闭", "A−", "A+", "背景 25%"] {
            let node = h.node(label);
            assert!(
                node.bounds.x >= 0.0 && node.bounds.x + node.bounds.width <= width as f32 + 1.0
            );
            assert!(node.bounds.y + node.bounds.height <= height as f32);
        }
        h.screenshot(
            &format!("desktop-adjust-{width}-{scale}x"),
            (width as f32 * scale) as u32,
            (height as f32 * scale) as u32,
        );
        h.state.desktop.phase = crate::session::DesktopDanmakuPhase::Locked;
        h.state.revisions.desktop += 1;
        h.sync();
        assert!(!h
            .agent
            .accessibility_dump()
            .iter()
            .any(|n| n.role == "button"));
        h.screenshot(
            &format!("desktop-locked-{width}-{scale}x"),
            (width as f32 * scale) as u32,
            (height as f32 * scale) as u32,
        );
    }
}

#[test]
#[ignore = "requires a GPU adapter; validates overview launch controls"]
fn overview_launch_is_explicit_and_feed_is_separate() {
    let mut h = Harness::new(fixture(), 960, 600, 1.0);
    assert!(!h
        .agent
        .accessibility_dump()
        .iter()
        .any(|n| n.label.as_deref() == Some("实时弹幕")));
    h.click("启动桌面弹幕");
    assert_eq!(
        h.state.desktop.phase,
        crate::session::DesktopDanmakuPhase::Creating
    );
    h.click("查看数据");
    assert_eq!(h.state.page, Page::Stats);
    assert_eq!(h.state.stats.selected_room, Some(12345));
}

#[test]
#[ignore = "requires a GPU adapter; validates short desktop feed alignment"]
fn short_desktop_feed_starts_at_bottom() {
    let mut state = fixture();
    while state.danmaku.messages.len() > 1 {
        state.danmaku.messages.pop_front();
    }
    let mut h = Harness::new_desktop(state, 420, 640, 1.0);
    let viewport = h.node("实时弹幕").bounds;
    let sender = h.reading_anchor();
    assert!(
        sender.1 > viewport.height * 0.5,
        "short feed should rest at bottom"
    );
    h.screenshot("desktop-short-feed", 420, 640);
}
