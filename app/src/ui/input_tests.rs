//! Event-source regressions run without a GPU; native_input_acceptance covers the host loop.
use super::*;

fn press(
    input: &mut HeadlessInput,
    document: &mut RuntimeDocument,
    key: &'static str,
) -> nana_ui::InputRouteOutcome {
    input
        .press(
            document.context_mut(),
            nana_ui::KeyInput::named(
                key,
                key,
                nana_ui::KeyState::Pressed,
                nana_ui::InputModifiers::default(),
            ),
            None,
            None,
        )
        .expect("route key")
}

fn fixture(login: bool) -> (Session, RuntimeDocument, Shell) {
    let mut session = Session::for_test();
    session.stats.selected_room = Some(42);
    session.apply(if login {
        AppEvent::OpenLogin
    } else {
        AppEvent::AskClearStats
    });
    let mut document = RuntimeDocument::new(DocumentId::new(1).unwrap());
    let shell = Shell::mount(&mut document, &session).unwrap();
    (session, document, shell)
}

#[test]
fn escape_closes_the_active_business_dialog_once() {
    for login in [true, false] {
        let (mut session, mut document, mut shell) = fixture(login);
        let id = document.document();
        let mut input = HeadlessInput::bind(document.context_mut(), id);
        assert!(press(&mut input, &mut document, "Escape").prevent_default);
        let events = session.inbox.drain();
        assert_eq!(events.len(), 1, "one close event per Escape");
        assert!(matches!(
            (&events[0], login),
            (AppEvent::CloseLogin, true) | (AppEvent::CancelClearStats, false)
        ));
        session.apply(events.into_iter().next().unwrap());
        shell.sync(&mut document, &session).unwrap();
        assert!(!session.auth.open);
        assert!(session.stats.confirm_clear.is_none());
        assert!(session.inbox.drain().is_empty());
    }
}

#[test]
fn stats_refresh_preserves_focus_for_consecutive_arrow_navigation() {
    let mut session = Session::for_test();
    session.page = Page::Stats;
    session.stats.selected_room = Some(42);
    session.stats.snapshots.push(crate::session::Snapshot {
        room_id: 42,
        room_name: "本地验收".into(),
        captured_at: 1,
        viewer_count: 1,
        follower_count: None,
        live_status: nanabobo_core::models::LiveStatus::Live,
    });
    let id = DocumentId::new(1).unwrap();
    let mut document = RuntimeDocument::new(id);
    let mut shell = Shell::mount(&mut document, &session).unwrap();
    let tabs = shell.stats_tabs.unwrap();
    let options = document
        .context()
        .read(tabs, |tabs| tabs.option_nodes().to_vec())
        .unwrap();
    let trend = options[0].1;
    let history = options[1].1;
    assert!(document.context_mut().focus_node(id, trend).unwrap());
    let mut input = HeadlessInput::bind(document.context_mut(), id);
    for (key, selected, focused) in [
        ("ArrowRight", crate::session::StatsTab::History, history),
        ("ArrowLeft", crate::session::StatsTab::Trend, trend),
    ] {
        press(&mut input, &mut document, key);
        for event in session.inbox.drain() {
            session.apply(event);
        }
        shell.sync(&mut document, &session).unwrap();
        assert_eq!(session.stats.tab, selected);
        assert_eq!(document.context().world().focused(id), Some(focused));
    }
}

#[test]
fn session_dismissal_does_not_queue_a_close_for_the_next_opening() {
    for login in [true, false] {
        let (mut session, mut document, mut shell) = fixture(login);
        session.apply(if login {
            AppEvent::CloseLogin
        } else {
            AppEvent::CancelClearStats
        });
        shell.sync(&mut document, &session).unwrap();
        session.apply(if login {
            AppEvent::OpenLogin
        } else {
            AppEvent::AskClearStats
        });
        shell.sync(&mut document, &session).unwrap();
        assert!(session.inbox.drain().is_empty(), "no stale close on reopen");
        assert_eq!(session.auth.open, login);
        assert_eq!(session.stats.confirm_clear, (!login).then_some(42));
        assert!(document
            .context()
            .has_blocking_runtime_overlay(document.document()));
    }
}

#[test]
fn closed_dialogs_stay_mounted_and_the_settings_page_fills() {
    let mut session = Session::for_test();
    session.page = Page::Settings;
    let mut document = RuntimeDocument::new(DocumentId::new(1).unwrap());
    let mut shell = Shell::mount(&mut document, &session).unwrap();
    let login = shell.login.stable_id();
    let clear = shell.clear.stable_id();
    assert!(document.context().world().contains(login));
    assert!(document.context().world().contains(clear));
    session.apply(AppEvent::OpenLogin);
    shell.sync(&mut document, &session).unwrap();
    session.apply(AppEvent::CloseLogin);
    shell.sync(&mut document, &session).unwrap();
    assert_eq!(shell.login.stable_id(), login);
    assert_eq!(shell.clear.stable_id(), clear);
    assert!(document.context().world().contains(login));
    assert!(document.context().world().contains(clear));
    assert!(!document
        .context()
        .has_blocking_runtime_overlay(document.document()));

    let id = document.document();
    document
        .context_mut()
        .layout_document(id, LayoutViewport::new(1200.0, 800.0))
        .unwrap();
    let settings = labeled(&document, "设置");
    let tallest = settings
        .iter()
        .map(|node| node.bounds.height)
        .fold(0.0_f32, f32::max);
    assert!(tallest > 400.0, "settings page collapsed: {settings:#?}");

    session.page = Page::Stats;
    session.stats.selected_room = Some(42);
    session.stats.snapshots.push(crate::session::Snapshot {
        room_id: 42,
        room_name: "本地验收".into(),
        captured_at: 1,
        viewer_count: 1,
        follower_count: None,
        live_status: nanabobo_core::models::LiveStatus::Live,
    });
    shell.sync(&mut document, &session).unwrap();
    document
        .context_mut()
        .layout_document(id, LayoutViewport::new(1200.0, 800.0))
        .unwrap();
    let trend = labeled(&document, "直播间趋势");
    let trend_height = trend
        .iter()
        .map(|node| node.bounds.height)
        .fold(0.0_f32, f32::max);
    assert!(
        trend_height > 200.0,
        "stats trend scroll collapsed: {trend:#?}"
    );
}

#[test]
fn desktop_drag_handle_follows_the_lock_and_rows_update_in_place() {
    let mut session = Session::for_test();
    session.room.info = Some(nanabobo_core::models::RoomInfo {
        room_id: 7,
        owner_id: 1,
        owner_name: None,
        owner_avatar_url: None,
        title: "本地".into(),
        live_status: nanabobo_core::models::LiveStatus::Live,
        viewer_count: 1,
        follower_count: None,
        cover_url: None,
        fetched_at: 1,
    });
    session.danmaku.status.connection_id = Some("feed".into());
    session.danmaku.status.room_id = Some(7);
    session.danmaku.status.state = nanabobo_core::models::DanmakuConnectionState::Connected;
    session.desktop.phase = crate::session::DesktopDanmakuPhase::Adjusting;
    session.apply(AppEvent::DanmakuMessage(
        nanabobo_core::models::DanmakuMessage {
            connection_id: "feed".into(),
            room_id: 7,
            sender_name: "听众".into(),
            text: "唯一弹幕".into(),
            sent_at: 1,
        },
    ));
    let mut document = RuntimeDocument::new(DocumentId::new(2).unwrap());
    let mut view = DesktopDanmakuView::mount(&mut document, &session).unwrap();
    view.sync(&mut document, &session).unwrap();
    let desktop_id = document.document();
    document
        .context_mut()
        .layout_document(desktop_id, LayoutViewport::new(480.0, 640.0))
        .unwrap();
    let feed = labeled(&document, "实时弹幕");
    let feed_height = feed
        .iter()
        .map(|node| node.bounds.height)
        .fold(0.0_f32, f32::max);
    assert!(feed_height > 300.0, "desktop feed collapsed: {feed:#?}");
    assert_eq!(count_label(&document, "拖动窗口"), 1);
    assert_eq!(count_label(&document, "唯一弹幕"), 1);
    let row_text = text_ids(&document);
    view.sync(&mut document, &session).unwrap();
    assert_eq!(count_label(&document, "唯一弹幕"), 1);
    assert_eq!(text_ids(&document), row_text, "row texts were remounted");

    session.desktop.phase = crate::session::DesktopDanmakuPhase::Locked;
    session.revisions.desktop += 1;
    view.sync(&mut document, &session).unwrap();
    assert_eq!(count_label(&document, "拖动窗口"), 0);
    assert!(view.drag_handle_bounds(&document).is_none());

    session.desktop.phase = crate::session::DesktopDanmakuPhase::Adjusting;
    session.revisions.desktop += 1;
    view.sync(&mut document, &session).unwrap();
    assert_eq!(count_label(&document, "拖动窗口"), 1);
}

fn labeled(document: &RuntimeDocument, label: &str) -> Vec<nana_ui::runtime::AccessibilityNode> {
    document
        .context()
        .world()
        .project_accessibility(document.document())
        .into_iter()
        .filter(|node| node.label.as_deref() == Some(label))
        .collect()
}

fn count_label(document: &RuntimeDocument, label: &str) -> usize {
    labeled(document, label).len()
}

fn text_ids(document: &RuntimeDocument) -> Vec<nana_ui::runtime::StableNodeId> {
    labeled(document, "唯一弹幕")
        .into_iter()
        .map(|node| node.id)
        .collect()
}
