mod auth;
mod danmaku;
mod desktop;
mod inbox;
mod resources;
mod room;
mod stats;
pub mod storage;

pub use auth::{AuthState, QrPhase};
pub use danmaku::DanmakuState;
pub use desktop::{
    DesktopDanmakuPhase, DesktopDanmakuSettings, DesktopDanmakuState, DesktopEffect,
};
pub use inbox::Inbox;
use nana_ui::{AppearanceEvent, AppearanceSettings, ThemeMode};
use nanabobo_core::bilibili::BilibiliClient;
use nanabobo_core::commands::{self, AppError, AppState};
use nanabobo_core::credential_store::{CredentialStore, KeyringCredentialStore};
use nanabobo_core::models::{
    AccountStatus, AuthPollResponse, DanmakuMessage, DanmakuStatus, LiveStatus, QrStartResponse,
    RoomInfo,
};
pub use resources::ImageChange;
use resources::Resources;
pub use room::RoomState;
pub use stats::{timestamp, Snapshot, StatsState, StatsTab};
use std::sync::Arc;
use std::time::Instant;
use storage::{Storage, StoredState};

#[derive(Debug, Clone, Copy)]
pub struct Wake;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Workbench,
    Stats,
    Settings,
}
#[derive(Debug, Clone)]
pub enum AppEvent {
    Navigate(Page),
    OpenLogin,
    CloseLogin,
    StartQr,
    Logout,
    EditRoom,
    CancelEditRoom,
    RoomIdChanged(String),
    QueryRoom,
    RefreshRoom,
    DisconnectRoom,
    OpenDesktopDanmaku,
    CloseDesktopDanmaku,
    AdjustDesktopDanmaku,
    LockDesktopDanmaku,
    DesktopFontSize(f32),
    DesktopBackgroundOpacity(f32),
    DesktopOpened {
        generation: u64,
    },
    DesktopOpenFailed {
        generation: u64,
    },
    DesktopClosed {
        generation: u64,
    },
    DesktopPassthroughResult {
        generation: u64,
        request: u64,
        enabled: bool,
        success: bool,
    },
    DesktopGeometry {
        generation: u64,
        width: f32,
        height: f32,
        position: [f32; 2],
    },
    FollowLatest,
    Reading {
        following: bool,
        offset: f32,
    },
    SelectStatsTab(StatsTab),
    SelectHistoryRoom(u64),
    AskClearStats,
    CancelClearStats,
    ClearStats,
    Appearance(AppearanceEvent),
    RetryStore,
    AuthStatus(u64, Result<AccountStatus, AppError>),
    QrStarted(u64, Result<QrStartResponse, AppError>),
    QrPolled(u64, Result<AuthPollResponse, AppError>),
    RoomLoaded {
        request: u64,
        refresh: bool,
        result: Result<RoomInfo, AppError>,
    },
    DanmakuStatus(DanmakuStatus),
    DanmakuMessage(DanmakuMessage),
    ImageLoaded {
        slot: String,
        revision: u64,
        image: Option<crate::images::DecodedImage>,
    },
    Stored {
        revision: u64,
        result: Result<(), String>,
    },
}
#[derive(Debug, Default)]
pub enum Operation {
    #[default]
    Idle,
    Loading,
    Failed(String),
}
impl Operation {
    pub fn loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
    pub fn error(&self) -> Option<&str> {
        if let Self::Failed(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
#[derive(Default)]
struct Request {
    revision: u64,
    state: Operation,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Request {
    fn cancel(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.revision = self.revision.wrapping_add(1);
        self.state = Operation::Idle;
    }
    fn begin(&mut self) -> u64 {
        self.cancel();
        self.state = Operation::Loading;
        self.revision
    }
    fn finish(&mut self, revision: u64) -> bool {
        if revision != self.revision || !self.state.loading() {
            return false;
        }
        self.task = None;
        self.state = Operation::Idle;
        true
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        self.cancel();
    }
}
#[derive(Default, Clone, Copy)]
pub struct Revisions {
    pub shell: u64,
    pub room: u64,
    pub messages: u64,
    pub stats: u64,
    pub overlay: u64,
    pub desktop: u64,
}
pub struct Session {
    pub inbox: Inbox,
    pub page: Page,
    pub theme: ThemeMode,
    pub appearance: AppearanceSettings,
    pub auth: AuthState,
    pub room: RoomState,
    pub danmaku: DanmakuState,
    pub desktop: DesktopDanmakuState,
    pub stats: StatsState,
    pub revisions: Revisions,
    pub storage_error: Option<String>,
    resources: Resources,
    core: Arc<AppState>,
    runtime: tokio::runtime::Handle,
    storage: Option<Storage>,
    storage_revision: u64,
    #[cfg(test)]
    test_mode: bool,
}
impl Session {
    pub fn new(runtime: tokio::runtime::Handle) -> Result<Self, String> {
        let loaded = storage::load();
        let mut s = Self::with_credentials(
            runtime,
            Arc::new(KeyringCredentialStore::new("com.senanana.nanabobo")),
            loaded.state,
        )?;
        s.storage_error = loaded.error;
        s.storage = Some(Storage::new(loaded.path, loaded.blocked, s.inbox.clone()));
        Ok(s)
    }
    fn with_credentials(
        runtime: tokio::runtime::Handle,
        credentials: Arc<dyn CredentialStore>,
        stored: StoredState,
    ) -> Result<Self, String> {
        let inbox = Inbox::new();
        let client = BilibiliClient::new().map_err(|_| "无法初始化 B 站客户端".to_owned())?;
        let core = Arc::new(AppState::new(
            client,
            credentials,
            runtime.clone(),
            Arc::new(inbox.clone()),
        ));
        let (theme, appearance) = stored.appearance_settings();
        Ok(Self {
            inbox,
            page: Page::Workbench,
            theme,
            appearance,
            auth: AuthState::default(),
            room: RoomState::new(stored.room_id),
            danmaku: DanmakuState::default(),
            desktop: DesktopDanmakuState::new(stored.desktop),
            stats: StatsState::new(stored.snapshots),
            revisions: Revisions::default(),
            storage_error: None,
            resources: Resources::default(),
            core,
            runtime,
            storage: None,
            storage_revision: 0,
            #[cfg(test)]
            test_mode: false,
        })
    }
    #[cfg(test)]
    pub fn for_test() -> Self {
        static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
        let runtime = RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("runtime")
        });
        let mut s = Self::with_credentials(
            runtime.handle().clone(),
            Arc::new(nanabobo_core::credential_store::MemoryCredentialStore::default()),
            StoredState::default(),
        )
        .expect("session");
        s.test_mode = true;
        s
    }
    pub fn bind_dispatch(&self, fire: Arc<dyn Fn() + Send + Sync>) {
        self.inbox.bind(fire);
    }
    #[cfg(all(test, feature = "native-acceptance"))]
    pub(crate) fn input_probe_request_ids(&self) -> (u64, u64) {
        (self.auth.request.revision, self.room.request.revision)
    }
    pub fn authenticated(&self) -> bool {
        self.auth
            .account
            .as_ref()
            .is_some_and(|s| s.authenticated && s.account.is_some())
    }
    pub fn account(&self) -> Option<&nanabobo_core::models::AccountSummary> {
        self.auth.account.as_ref().and_then(|s| s.account.as_ref())
    }
    pub fn has_image(&self, slot: &str) -> bool {
        self.resources.ready(slot)
    }
    pub fn take_pending_images(&mut self) -> Vec<ImageChange> {
        std::mem::take(&mut self.resources.pending)
    }
    pub fn next_wakeup(&self) -> Option<Instant> {
        [self.auth.next_poll, self.room.next_refresh]
            .into_iter()
            .flatten()
            .min()
    }
    pub fn advance(&mut self, now: Instant) {
        if self.auth.next_poll.is_some_and(|d| d <= now) {
            self.auth.next_poll = None;
            self.poll_qr();
        }
        if self.room.next_refresh.is_some_and(|d| d <= now) {
            self.room.next_refresh = None;
            self.query_room(true);
        }
    }
    pub fn apply(&mut self, event: AppEvent) {
        match event {
            AppEvent::Navigate(page) => {
                self.page = page;
                self.stats.confirm_clear = None;
                self.revisions.shell += 1;
                self.revisions.overlay += 1;
            }
            AppEvent::OpenLogin => {
                self.auth.open = true;
                if self.auth.qr.is_none() && !self.auth.request.state.loading() {
                    self.start_qr();
                }
                self.revisions.overlay += 1;
            }
            AppEvent::CloseLogin => {
                self.auth.open = false;
                self.cancel_qr();
                self.revisions.overlay += 1;
            }
            AppEvent::StartQr => self.start_qr(),
            AppEvent::Logout => self.logout(),
            AppEvent::EditRoom => {
                self.room.editing = true;
                self.room.input = self
                    .room
                    .info
                    .as_ref()
                    .map(|r| r.room_id.to_string())
                    .unwrap_or_else(|| self.room.remembered.clone());
                self.revisions.room += 1;
            }
            AppEvent::CancelEditRoom => {
                self.room.editing = false;
                self.room.request.cancel();
                self.room.schedule_refresh();
                self.revisions.room += 1;
            }
            AppEvent::RoomIdChanged(value) => {
                self.room.input = value;
                if !self.room.request.state.loading() {
                    if self.room.request.state.error().is_some() {
                        self.revisions.room += 1;
                    }
                    self.room.request.state = Operation::Idle;
                }
            }
            AppEvent::QueryRoom => self.query_room(false),
            AppEvent::RefreshRoom => self.query_room(true),
            AppEvent::DisconnectRoom => self.disconnect_room(),
            AppEvent::OpenDesktopDanmaku => self.open_desktop(),
            AppEvent::CloseDesktopDanmaku => self.close_desktop(),
            AppEvent::AdjustDesktopDanmaku => self.adjust_desktop(),
            AppEvent::LockDesktopDanmaku => self.set_desktop_passthrough(true),
            AppEvent::DesktopFontSize(value) => self.set_desktop_font_size(value),
            AppEvent::DesktopBackgroundOpacity(value) => self.set_desktop_opacity(value),
            AppEvent::DesktopOpened { generation } => self.desktop_opened(generation),
            AppEvent::DesktopOpenFailed { generation } => self.desktop_open_failed(generation),
            AppEvent::DesktopClosed { generation } => self.desktop_closed(generation),
            AppEvent::DesktopPassthroughResult {
                generation,
                request,
                enabled,
                success,
            } => self.desktop_passthrough_result(generation, request, enabled, success),
            AppEvent::DesktopGeometry {
                generation,
                width,
                height,
                position,
            } => self.desktop_geometry(generation, width, height, position),
            AppEvent::FollowLatest => {
                self.danmaku.following = true;
                self.danmaku.unread = 0;
                self.revisions.messages += 1;
            }
            AppEvent::Reading { following, offset } => {
                if self.desktop.phase == DesktopDanmakuPhase::Locked {
                    return;
                }
                self.danmaku.following = following;
                self.danmaku.scroll_offset = offset;
                if following {
                    self.danmaku.unread = 0;
                }
                self.revisions.messages += 1;
            }
            AppEvent::SelectStatsTab(tab) => {
                self.stats.tab = tab;
                self.revisions.stats += 1;
            }
            AppEvent::SelectHistoryRoom(id) => {
                self.stats.selected_room = Some(id);
                self.stats.confirm_clear = None;
                self.revisions.stats += 1;
            }
            AppEvent::AskClearStats => {
                self.stats.confirm_clear = self.stats.selected_room;
                self.revisions.overlay += 1;
            }
            AppEvent::CancelClearStats => {
                self.stats.confirm_clear = None;
                self.revisions.overlay += 1;
            }
            AppEvent::ClearStats => self.clear_stats(),
            AppEvent::Appearance(event) => {
                nana_ui::runtime::apply_appearance_event(
                    &mut self.theme,
                    &mut self.appearance,
                    event,
                );
                self.revisions.shell += 1;
                self.persist(false);
            }
            AppEvent::RetryStore => self.persist(true),
            AppEvent::AuthStatus(id, result) => self.on_auth_status(id, result),
            AppEvent::QrStarted(id, result) => self.on_qr_started(id, result),
            AppEvent::QrPolled(id, result) => self.on_qr_polled(id, result),
            AppEvent::RoomLoaded {
                request,
                refresh,
                result,
            } => self.on_room_loaded(request, refresh, result),
            AppEvent::DanmakuStatus(status) => self.on_danmaku_status(status),
            AppEvent::DanmakuMessage(message) => self.on_danmaku_message(message),
            AppEvent::ImageLoaded {
                slot,
                revision,
                image,
            } => {
                if self.resources.complete(&slot, revision, image) {
                    self.revisions.room += 1;
                    self.revisions.shell += 1;
                }
            }
            AppEvent::Stored { revision, result } => {
                if revision == self.storage_revision {
                    let error = result.err();
                    if self.storage_error != error {
                        self.storage_error = error;
                        self.revisions.shell += 1;
                    }
                }
            }
        }
    }
    fn persist(&mut self, recover: bool) {
        self.storage_revision += 1;
        if let Some(storage) = &self.storage {
            storage.save(
                self.storage_revision,
                StoredState::from_session(self),
                recover,
            );
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.auth.request.cancel();
        self.room.request.cancel();
        let _ = commands::auth_qr_cancel(&self.core);
        if let Some(id) = self.danmaku.status.connection_id.clone() {
            let _ = commands::danmaku_stop(&self.core, id);
        }
    }
}
pub fn live_label(status: &LiveStatus) -> &'static str {
    match status {
        LiveStatus::Live => "直播中",
        LiveStatus::Round => "轮播中",
        LiveStatus::Offline => "未开播",
        LiveStatus::Unknown => "状态未知",
    }
}
pub fn danmaku_label(status: &DanmakuStatus) -> &'static str {
    use nanabobo_core::models::DanmakuConnectionState::*;
    match status.state {
        Connected => "已连接",
        Connecting => "连接中",
        Reconnecting => "正在重连",
        Error => "连接异常",
        Stopped => "已停止",
        Idle => "未连接",
    }
}
