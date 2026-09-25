//! Real host-loop acceptance with synthetic input; never accesses user storage or the network.
use super::*;
use crate::session::{Page, Snapshot, StatsTab};
use nana_ui::runtime::{
    AccessibilityAction, AccessibilityActionRequest, AccessibilityNode, StableNodeId,
};
use nana_ui::RuntimeInputAdapter;
use nana_ui_platform::InputModifiers;
use nanabobo_core::models::{AccountStatus, AccountSummary, LiveStatus, QrStartResponse};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static COMPLETED: AtomicBool = AtomicBool::new(false);

struct InputProbe {
    app: NanaBoboProgram,
    stage: usize,
    deadline: Instant,
    return_focus: Option<StableNodeId>,
    request_before: u64,
}
impl InputProbe {
    fn node(&self, label: &str) -> AccessibilityNode {
        let nodes = self
            .app
            .document
            .context()
            .world()
            .project_accessibility(self.app.document.document());
        nodes
            .iter()
            .find(|node| node.label.as_deref() == Some(label) && !node.disabled)
            .unwrap_or_else(|| panic!("missing reachable {label}: {nodes:#?}"))
            .clone()
    }
    fn focused(&self) -> Option<StableNodeId> {
        self.app
            .document
            .context()
            .world()
            .focused(self.app.document.document())
    }
    fn key(&mut self, key: &str, modifiers: InputModifiers, context: &RuntimeProgramContext<Wake>) {
        let event = InputEvent::Keyboard {
            pressed: true,
            key: key.into(),
            text: None,
            code: key.into(),
            repeat: false,
            modifiers,
        };
        self.input(event, context);
    }
    fn input(&mut self, event: InputEvent, context: &RuntimeProgramContext<Wake>) {
        let document_id = self.app.document.document();
        let disposition = RuntimeInputAdapter::default()
            .dispatch(self.app.document.context_mut(), document_id, &event)
            .expect("runtime input");
        let update = self
            .app
            .input_event(
                WindowId::PRIMARY,
                nana_ui::RoutedInput {
                    event: &event,
                    pointer_hit: None,
                    disposition,
                },
                context,
            )
            .expect("application input hook");
        assert!(!update.exit && update.window_commands.is_empty());
    }
    fn tab_to(&mut self, label: &str, context: &RuntimeProgramContext<Wake>) {
        let target = self.node(label).id;
        for _ in 0..80 {
            if self.focused() == Some(target) {
                return;
            }
            self.key("Tab", InputModifiers::default(), context);
        }
        panic!("Tab cannot reach {label}; focus={:?}", self.focused());
    }
    fn activate(&mut self, label: &str, context: &RuntimeProgramContext<Wake>) {
        self.tab_to(label, context);
        self.key("Enter", InputModifiers::default(), context);
    }
    fn action(
        &mut self,
        label: &str,
        action: AccessibilityAction,
        context: &RuntimeProgramContext<Wake>,
    ) {
        let target = self.node(label).id;
        self.app
            .accessibility_action(
                WindowId::PRIMARY,
                AccessibilityActionRequest { target, action },
                context,
            )
            .expect("application accessibility hook");
    }
    fn qr(&self) {
        self.app.session.inbox.push(AppEvent::QrStarted(
            self.app.session.input_probe_request_ids().0,
            Ok(QrStartResponse {
                session_id: "isolated-probe".into(),
                payload: "https://example.invalid/local-probe".into(),
                expires_at: u64::MAX,
            }),
        ));
    }
    fn step(&mut self, context: &RuntimeProgramContext<Wake>) -> bool {
        let normal = InputModifiers::default();
        match self.stage {
            0 => {
                self.tab_to("登录 B 站", context);
                self.return_focus = self.focused();
                self.key("Enter", normal, context);
                assert!(!self.app.session.auth.open, "business work must await Wake");
            }
            1 => {
                assert!(self.app.session.auth.open);
                self.qr();
            }
            2 => {
                let allowed = [self.node("重新生成二维码").id, self.node("取消登录").id];
                for _ in 0..8 {
                    self.key("Tab", normal, context);
                    assert!(allowed.contains(&self.focused().expect("modal focus")));
                }
                self.key(
                    "Tab",
                    InputModifiers {
                        shift: true,
                        ..normal
                    },
                    context,
                );
                assert!(allowed.contains(&self.focused().expect("modal reverse focus")));
                self.activate("重新生成二维码", context);
            }
            3 => {
                assert!(self.app.session.auth.loading());
                self.qr();
            }
            4 => {
                self.key("Escape", normal, context);
            }
            5 => {
                assert!(!self.app.session.auth.open);
                assert_eq!(self.focused(), self.return_focus);
                self.activate("登录 B 站", context);
            }
            6 => {
                assert!(self.app.session.auth.open);
                self.activate("取消登录", context);
            }
            7 => {
                assert!(!self.app.session.auth.open);
                self.app.session.load_account();
                self.app.session.inbox.push(AppEvent::AuthStatus(
                    self.app.session.input_probe_request_ids().0,
                    Ok(AccountStatus {
                        authenticated: true,
                        account: Some(AccountSummary {
                            mid: 1,
                            username: "本地验收".into(),
                            avatar_url: None,
                            level: None,
                            coins: None,
                            bcoin: None,
                            vip: None,
                        }),
                    }),
                ));
            }
            8 => {
                assert!(self.app.session.authenticated());
                self.tab_to("直播间号", context);
                self.input(
                    InputEvent::Keyboard {
                        pressed: true,
                        key: "1234".into(),
                        text: Some("1234".into()),
                        code: String::new(),
                        repeat: false,
                        modifiers: normal,
                    },
                    context,
                );
                assert!(
                    self.app.session.room.input.is_empty(),
                    "text callback must await Wake"
                );
            }
            9 => {
                assert_eq!(self.app.session.room.input, "1234");
                self.key("Backspace", normal, context);
            }
            10 => {
                assert_eq!(self.app.session.room.input, "123");
                self.key(
                    "a",
                    InputModifiers {
                        control: true,
                        ..normal
                    },
                    context,
                );
                assert_eq!(
                    self.app
                        .document
                        .context()
                        .focused_selected_text(self.app.document.document())
                        .as_deref(),
                    Some("123")
                );
                self.key("Delete", normal, context);
            }
            11 => {
                assert_eq!(self.app.session.room.input, "");
                self.action(
                    "直播间号",
                    AccessibilityAction::SetValue("42".into()),
                    context,
                );
            }
            12 => {
                assert_eq!(self.app.session.room.input, "42");
                assert_eq!(
                    self.node("直播间号").value.as_deref(),
                    Some("42"),
                    "accessible value follows the controlled field"
                );
                self.action("直播间号", AccessibilityAction::Focus, context);
                assert_eq!(self.focused(), Some(self.node("直播间号").id));
                self.request_before = self.app.session.input_probe_request_ids().1;
                self.key("Enter", normal, context);
            }
            13 => {
                assert!(self.app.session.room.loading());
                assert_eq!(
                    self.app.session.input_probe_request_ids().1,
                    self.request_before + 1
                );
                self.activate("数据", context);
            }
            14 => {
                assert_eq!(self.app.session.page, Page::Stats);
                self.tab_to("趋势", context);
                self.key("ArrowRight", normal, context);
                assert_eq!(self.focused(), Some(self.node("历史").id));
            }
            15 => {
                assert_eq!(self.app.session.stats.tab, StatsTab::History);
                assert_eq!(
                    self.focused(),
                    Some(self.node("历史").id),
                    "selected tab retains keyboard focus"
                );
                self.key("ArrowLeft", normal, context);
            }
            16 => {
                assert_eq!(self.app.session.stats.tab, StatsTab::Trend);
                self.activate("清理记录", context);
                self.return_focus = self.focused();
            }
            17 => {
                assert_eq!(self.app.session.stats.confirm_clear, Some(42));
                self.key("Escape", normal, context);
            }
            18 => {
                assert!(self.app.session.stats.confirm_clear.is_none());
                assert_eq!(self.app.session.stats.snapshots.len(), 2);
                assert_eq!(self.focused(), self.return_focus);
                self.activate("清理记录", context);
            }
            19 => {
                self.tab_to("取消", context);
                self.key(" ", normal, context);
            }
            20 => {
                assert!(self.app.session.stats.confirm_clear.is_none());
                assert_eq!(self.app.session.stats.snapshots.len(), 2);
                self.action("清理记录", AccessibilityAction::Click, context);
            }
            21 => {
                assert_eq!(self.app.session.stats.confirm_clear, Some(42));
                self.activate("删除记录", context);
            }
            22 => {
                assert!(self.app.session.stats.confirm_clear.is_none());
                assert_eq!(self.app.session.stats.snapshots.len(), 1);
                assert_eq!(self.app.session.stats.snapshots[0].room_id, 99);
                self.activate("设置", context);
            }
            23 => {
                assert_eq!(self.app.session.page, Page::Settings);
                self.activate("概览", context);
            }
            24 => {
                assert_eq!(self.app.session.page, Page::Workbench);
                let target = self.node("设置").id;
                let before = self.focused();
                let mut document = RuntimeDocument::new(DocumentId::new(2).unwrap());
                let view = DesktopDanmakuView::mount(&mut document, &self.app.session)
                    .expect("auxiliary document fixture");
                self.app.desktop = Some(DesktopWindow {
                    id: WindowId(777),
                    document,
                    view,
                    passthrough_request: None,
                });
                for id in [WindowId(999), WindowId(777)] {
                    for action in [
                        AccessibilityAction::Focus,
                        AccessibilityAction::Click,
                        AccessibilityAction::SetValue("bad".into()),
                    ] {
                        self.app
                            .accessibility_action(
                                id,
                                AccessibilityActionRequest { target, action },
                                context,
                            )
                            .expect("other window cannot target primary document");
                    }
                }
                assert_eq!(self.focused(), before);
            }
            25 => {
                assert_eq!(self.app.session.page, Page::Workbench);
                assert_eq!(self.app.session.room.input, "42");
                self.action("设置", AccessibilityAction::Click, context);
            }
            26 => {
                assert_eq!(self.app.session.page, Page::Settings);
                return true;
            }
            _ => unreachable!(),
        }
        self.stage += 1;
        self.app.update(Wake, context);
        false
    }
}
impl RuntimeProgram for InputProbe {
    type Message = Wake;
    type Error = String;
    fn theme_mode(&self) -> ThemeMode {
        self.app.theme_mode()
    }
    fn initialize(context: &RuntimeProgramContext<Wake>) -> Result<(Self, Vec<Wake>), String> {
        let mut session = Session::for_test();
        session.stats.selected_room = Some(42);
        session.stats.snapshots = [42, 99]
            .into_iter()
            .map(|room_id| Snapshot {
                room_id,
                room_name: format!("本地 {room_id}"),
                captured_at: 1,
                viewer_count: 1,
                follower_count: None,
                live_status: LiveStatus::Live,
            })
            .collect();
        Ok((
            Self {
                app: NanaBoboProgram::for_test(session, context)?,
                stage: 0,
                deadline: Instant::now() + Duration::from_secs(45),
                return_focus: None,
                request_before: 0,
            },
            vec![Wake],
        ))
    }
    fn with_document<R>(
        &self,
        id: WindowId,
        f: impl FnOnce(&RuntimeDocument) -> R,
    ) -> Result<Option<R>, nana_ui::DocumentAccessError> {
        self.app.with_document(id, f)
    }
    fn with_document_mut<R>(
        &mut self,
        id: WindowId,
        f: impl FnOnce(&mut RuntimeDocument) -> R,
    ) -> Result<Option<R>, nana_ui::DocumentAccessError> {
        self.app.with_document_mut(id, f)
    }
    fn update(
        &mut self,
        message: Wake,
        context: &RuntimeProgramContext<Wake>,
    ) -> RuntimeProgramUpdate {
        self.app.update(message, context)
    }
    fn prepare_window_frame(&mut self, id: WindowId, context: &RuntimeProgramContext<Wake>) {
        self.app.prepare_window_frame(id, context);
    }
    fn window_frame_presented(
        &mut self,
        id: WindowId,
        context: &RuntimeProgramContext<Wake>,
    ) -> RuntimeProgramUpdate {
        if id != WindowId::PRIMARY {
            return RuntimeProgramUpdate::default();
        }
        println!("Native input acceptance stage {}", self.stage);
        if self.step(context) {
            COMPLETED.store(true, Ordering::SeqCst);
            RuntimeProgramUpdate::exit()
        } else {
            RuntimeProgramUpdate::redraw(id)
        }
    }
    fn next_wakeup(&self) -> Option<Instant> {
        Some((Instant::now() + Duration::from_millis(50)).min(self.deadline))
    }
    fn wake(&mut self, now: Instant, _: &RuntimeProgramContext<Wake>) -> RuntimeProgramUpdate {
        if now >= self.deadline {
            RuntimeProgramUpdate::exit()
        } else {
            RuntimeProgramUpdate::redraw(WindowId::PRIMARY)
        }
    }
}
pub(super) fn run_probe() {
    COMPLETED.store(false, Ordering::SeqCst);
    run_runtime::<InputProbe>(
        WindowDescriptor::new("NanaBobo 原生键盘与无障碍验收")
            .initial_size(1200.0, 800.0)
            .system_caption(false),
    )
    .expect("native input runtime");
    assert!(
        COMPLETED.load(Ordering::SeqCst),
        "native input acceptance timed out"
    );
    println!("Native input acceptance passed: synthetic keyboard and accessibility actions, real Inbox/Wake/update and frame presentation.");
}
