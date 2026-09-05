//! Event-source regressions run without a GPU; native_input_acceptance covers the host loop.
use super::*;

fn keyboard(key: &str) -> nana_ui_platform::InputEvent {
    nana_ui_platform::InputEvent::Keyboard {
        pressed: true,
        key: key.into(),
        text: None,
        code: key.into(),
        repeat: false,
        modifiers: nana_ui_platform::InputModifiers::default(),
    }
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
        let disposition = nana_ui::RuntimeInputAdapter::default()
            .dispatch(document.context_mut(), id, &keyboard("Escape"))
            .unwrap();
        assert!(disposition.prevent_default);
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
    for (key, selected, focused) in [
        ("ArrowRight", crate::session::StatsTab::History, history),
        ("ArrowLeft", crate::session::StatsTab::Trend, trend),
    ] {
        nana_ui::RuntimeInputAdapter::default()
            .dispatch(document.context_mut(), id, &keyboard(key))
            .unwrap();
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
