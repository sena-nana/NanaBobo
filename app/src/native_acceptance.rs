//! Opt-in native host acceptance: exercises the real Windows window lifecycle.
use super::*;
use crate::session::DesktopDanmakuPhase;
use nanabobo_core::models::{AccountStatus, AccountSummary, DanmakuMessage, LiveStatus, RoomInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static COMPLETED: AtomicBool = AtomicBool::new(false);

impl NanaBoboProgram {
    pub(super) fn for_test(
        session: Session,
        context: &RuntimeProgramContext<Wake>,
    ) -> Result<Self, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let dispatch = context.clone();
        session.bind_dispatch(Arc::new(move || dispatch.dispatch(Wake)));
        let mut document = RuntimeDocument::new(DocumentId::new(1).unwrap());
        let shell = Shell::mount(&mut document, &session).map_err(|e| e.to_string())?;
        Ok(Self {
            document,
            shell,
            desktop: None,
            session,
            textures: HostTextureRegistry::new(),
            decoded: HashMap::new(),
            gpu_keep: HashMap::new(),
            next_texture_id: 1,
            _runtime: runtime,
        })
    }
}

struct NativeProbe {
    app: NanaBoboProgram,
    stage: u8,
    first: Option<WindowId>,
    deadline: Instant,
}

impl RuntimeProgram for NativeProbe {
    type Message = Wake;
    type Error = String;

    fn initialize(context: &RuntimeProgramContext<Wake>) -> Result<(Self, Vec<Wake>), String> {
        let mut session = Session::for_test();
        session.auth.account = Some(AccountStatus {
            authenticated: true,
            account: Some(AccountSummary {
                mid: 1,
                username: "本地验收主播".into(),
                avatar_url: None,
                level: None,
                coins: None,
                bcoin: None,
                vip: None,
            }),
        });
        session.room.info = Some(RoomInfo {
            room_id: 1,
            owner_id: 1,
            owner_name: Some("本地验收主播".into()),
            owner_avatar_url: None,
            title: "独立弹幕窗口验收".into(),
            live_status: LiveStatus::Live,
            viewer_count: 42,
            follower_count: Some(100),
            cover_url: None,
            fetched_at: 0,
        });
        let app = NanaBoboProgram::for_test(session, context)?;
        app.session.inbox.push(AppEvent::OpenDesktopDanmaku);
        Ok((
            Self {
                app,
                stage: 0,
                first: None,
                deadline: Instant::now() + Duration::from_secs(60),
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
    fn theme_mode(&self) -> ThemeMode {
        self.app.theme_mode()
    }
    fn window_material_mode_for(&self, id: WindowId) -> nana_ui::MaterialEffect {
        self.app.window_material_mode_for(id)
    }
    fn appearance_backdrop_opacity_for(&self, id: WindowId) -> f32 {
        self.app.appearance_backdrop_opacity_for(id)
    }
    fn host_textures(&self, id: WindowId) -> Option<HostTextureRegistry> {
        self.app.host_textures(id)
    }
    fn prepare_window_frame(&mut self, id: WindowId, context: &RuntimeProgramContext<Wake>) {
        self.app.prepare_window_frame(id, context);
    }
    fn rebuild_gpu(&mut self, context: &RuntimeProgramContext<Wake>) {
        self.app.rebuild_gpu(context);
    }
    fn input_event(
        &mut self,
        id: WindowId,
        routed: nana_ui::RoutedInput<'_>,
        context: &RuntimeProgramContext<Wake>,
    ) -> Result<RuntimeProgramUpdate, nana_ui::runtime::FrameworkError> {
        self.app.input_event(id, routed, context)
    }
    fn window_event(
        &mut self,
        event: WindowEvent,
        context: &RuntimeProgramContext<Wake>,
    ) -> RuntimeProgramUpdate {
        if let WindowEvent::MousePassthroughChanged { result, .. } = &event {
            assert!(
                result.is_ok(),
                "native mouse passthrough failed: {result:?}"
            );
        }
        let closed = matches!(event, WindowEvent::Closed { id } if Some(id) == self.first);
        let update = self.app.window_event(event, context);
        if closed && self.stage == 3 {
            assert!(self.app.desktop.is_none());
            assert!(!self.app.session.danmaku.active());
            assert_eq!(self.app.session.danmaku.messages.len(), 1);
            self.stage = 4;
            self.app.session.inbox.push(AppEvent::OpenDesktopDanmaku);
        }
        update
    }
    fn window_frame_presented(
        &mut self,
        id: WindowId,
        context: &RuntimeProgramContext<Wake>,
    ) -> RuntimeProgramUpdate {
        if id == WindowId::PRIMARY {
            return RuntimeProgramUpdate::default();
        }
        let phase = self.app.session.desktop.phase;
        match self.stage {
            0 if phase == DesktopDanmakuPhase::Adjusting => {
                self.first = Some(id);
                assert!(self.app.session.danmaku.active());
                assert_ne!(
                    self.app.document.document(),
                    self.app.desktop.as_ref().unwrap().document.document()
                );
                let connection_id = self
                    .app
                    .session
                    .danmaku
                    .status
                    .connection_id
                    .clone()
                    .unwrap();
                self.app
                    .session
                    .apply(AppEvent::DanmakuMessage(DanmakuMessage {
                        connection_id: connection_id.clone(),
                        room_id: 1,
                        sender_name: "观众".into(),
                        text: "原生窗口中的验收弹幕".into(),
                        sent_at: 1,
                    }));
                self.app.session.apply(AppEvent::OpenDesktopDanmaku);
                assert_eq!(
                    self.app.session.danmaku.status.connection_id.as_ref(),
                    Some(&connection_id)
                );
                self.app.session.apply(AppEvent::LockDesktopDanmaku);
                self.stage = 1;
                self.app.apply_pending()
            }
            1 if phase == DesktopDanmakuPhase::Locked => {
                assert!(self.app.session.danmaku.following);
                self.stage = 2;
                self.app.session.apply(AppEvent::AdjustDesktopDanmaku);
                self.app.apply_pending()
            }
            2 if phase == DesktopDanmakuPhase::Adjusting => {
                self.stage = 3;
                let update = self
                    .app
                    .window_event(WindowEvent::CloseRequested { id }, context);
                assert!(
                    !update.exit,
                    "closing auxiliary window must preserve main window"
                );
                update
            }
            4 if phase == DesktopDanmakuPhase::Adjusting => {
                assert_ne!(self.first, Some(id));
                assert!(self.app.session.danmaku.active());
                assert_eq!(self.app.session.danmaku.messages.len(), 1);
                let update = self.app.window_event(
                    WindowEvent::CloseRequested {
                        id: WindowId::PRIMARY,
                    },
                    context,
                );
                assert!(update.exit);
                assert!(self.app.desktop.is_none());
                assert!(!self.app.session.danmaku.active());
                COMPLETED.store(true, Ordering::SeqCst);
                self.stage = 5;
                update
            }
            _ => RuntimeProgramUpdate::default(),
        }
    }
    fn next_wakeup(&self) -> Option<Instant> {
        Some(self.deadline)
    }
    fn wake(
        &mut self,
        _now: Instant,
        _context: &RuntimeProgramContext<Wake>,
    ) -> RuntimeProgramUpdate {
        RuntimeProgramUpdate::exit()
    }
}

pub(super) fn run_probe() {
    COMPLETED.store(false, Ordering::SeqCst);
    run_runtime::<NativeProbe>(
        WindowDescriptor::new("NanaBobo 原生窗口验收")
            .initial_size(960.0, 600.0)
            .minimum_size(960.0, 600.0)
            .system_caption(false),
    )
    .expect("native runtime");
    assert!(
        COMPLETED.load(Ordering::SeqCst),
        "native lifecycle did not complete before deadline"
    );
    println!("Native desktop lifecycle passed: open, lock, unlock, close, reopen, exit.");
}
