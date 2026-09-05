use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nanabobo_core::bilibili::{BilibiliClient, DANMAKU_MESSAGE_EVENT, DANMAKU_STATUS_EVENT};
use nanabobo_core::commands::{self, AppError, AppState};
use nanabobo_core::credential_store::{CredentialStore, KeyringCredentialStore};
use nanabobo_core::events::EventSink;
use nanabobo_core::models::{
    AccountStatus, AuthPollResponse, DanmakuMessage, DanmakuStatus, QrStartResponse, RoomInfo,
};
use nana_ui::{AppearanceEvent, AppearanceSettings, ThemeMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CREDENTIAL_SERVICE: &str = "com.senanana.nanabobo";
const STORAGE_FILE: &str = "nanabobo-storage.json";
const MAX_SNAPSHOTS_PER_ROOM: usize = 2_000;
const MAX_CHART_POINTS: usize = 60;
const MAX_DANMAKU: usize = 1_000;
const QR_POLL: Duration = Duration::from_millis(1_500);
const STATS_POLL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wake;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Assistant,
    Stats,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsTab {
    Trend,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrPhase {
    Idle,
    Pending,
    Scanned,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub room_id: u64,
    pub captured_at: u64,
    pub viewer_count: u64,
    pub follower_count: Option<u64>,
    pub live_status: String,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Navigate(Page),
    OpenLogin,
    CloseLogin,
    StartQr,
    PollQr,
    Logout,
    RoomIdChanged(String),
    QueryRoom,
    DisconnectRoom,
    StartDanmaku,
    StopDanmaku,
    SelectStatsTab(StatsTab),
    AskClearStats,
    CancelClearStats,
    ClearStats,
    TickStats,
    Appearance(AppearanceEvent),
    AuthStatus(Result<AccountStatus, AppError>),
    QrStarted(Result<QrStartResponse, AppError>),
    QrPolled(Result<AuthPollResponse, AppError>),
    RoomLoaded {
        preserve: bool,
        result: Result<RoomInfo, AppError>,
    },
    DanmakuStatus(DanmakuStatus),
    DanmakuMessage(DanmakuMessage),
    ImageReady(crate::images::DecodedImage),
}

#[derive(Clone)]
pub struct Inbox(Arc<Mutex<VecDeque<AppEvent>>>);

impl Inbox {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(VecDeque::new())))
    }

    pub fn push(&self, event: AppEvent) {
        if let Ok(mut queue) = self.0.lock() {
            queue.push_back(event);
        }
    }

    pub fn drain(&self) -> Vec<AppEvent> {
        self.0
            .lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default()
    }
}

struct UiSink {
    inbox: Inbox,
    dispatch: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
}

impl EventSink for UiSink {
    fn emit(&self, event: &str, payload: Value) {
        match event {
            DANMAKU_STATUS_EVENT => {
                if let Ok(status) = serde_json::from_value::<DanmakuStatus>(payload) {
                    self.inbox.push(AppEvent::DanmakuStatus(status));
                    wake(&self.dispatch);
                }
            }
            DANMAKU_MESSAGE_EVENT => {
                if let Ok(message) = serde_json::from_value::<DanmakuMessage>(payload) {
                    self.inbox.push(AppEvent::DanmakuMessage(message));
                    wake(&self.dispatch);
                }
            }
            _ => {}
        }
    }
}

fn wake(dispatch: &Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>) {
    if let Ok(slot) = dispatch.lock() {
        if let Some(fire) = slot.as_ref() {
            fire();
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct StoredState {
    room_id: String,
    theme: String,
    snapshots: Vec<Snapshot>,
}

pub struct Session {
    pub inbox: Inbox,
    pub page: Page,
    pub login_open: bool,
    pub confirm_clear: bool,
    pub stats_tab: StatsTab,
    pub theme: ThemeMode,
    pub appearance: AppearanceSettings,
    pub account: Option<AccountStatus>,
    pub qr: Option<QrStartResponse>,
    pub qr_phase: QrPhase,
    pub room_id: String,
    pub room: Option<RoomInfo>,
    pub danmaku: DanmakuStatus,
    pub messages: Vec<DanmakuMessage>,
    pub snapshots: Vec<Snapshot>,
    pub account_error: Option<String>,
    pub room_error: Option<String>,
    pub danmaku_error: Option<String>,
    pub account_loading: bool,
    pub qr_polling: bool,
    pub room_loading: bool,
    pub danmaku_loading: bool,
    pub restore_attempted: bool,
    ready_images: HashSet<String>,
    pending_images: Arc<Mutex<Vec<crate::images::DecodedImage>>>,
    requested_images: HashSet<String>,
    core: Arc<AppState>,
    runtime: tokio::runtime::Handle,
    dispatch: Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>,
    next_qr_poll: Option<Instant>,
    next_stats: Option<Instant>,
}

impl Session {
    pub fn new(runtime: tokio::runtime::Handle) -> Result<Self, String> {
        Self::with_credentials(
            runtime,
            Arc::new(KeyringCredentialStore::new(CREDENTIAL_SERVICE)),
            load_store(),
            true,
        )
    }

    #[cfg(test)]
    pub fn for_test() -> Self {
        static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
        let handle = RUNTIME
            .get_or_init(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("test runtime")
            })
            .handle()
            .clone();
        Self::with_credentials(
            handle,
            Arc::new(nanabobo_core::credential_store::MemoryCredentialStore::default()),
            StoredState::default(),
            false,
        )
        .expect("test session")
    }

    fn with_credentials(
        runtime: tokio::runtime::Handle,
        credentials: Arc<dyn CredentialStore>,
        stored: StoredState,
        loading: bool,
    ) -> Result<Self, String> {
        let inbox = Inbox::new();
        let dispatch = Arc::new(Mutex::new(None));
        let client = BilibiliClient::new().map_err(|_| "无法初始化 B 站客户端".to_owned())?;
        let sink = Arc::new(UiSink {
            inbox: inbox.clone(),
            dispatch: Arc::clone(&dispatch),
        });
        let core = Arc::new(AppState::new(
            client,
            credentials,
            runtime.clone(),
            sink,
        ));
        Ok(Self {
            inbox,
            page: Page::Home,
            login_open: false,
            confirm_clear: false,
            stats_tab: StatsTab::Trend,
            theme: if stored.theme == "light" {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            },
            appearance: AppearanceSettings::default(),
            account: None,
            qr: None,
            qr_phase: QrPhase::Idle,
            room_id: stored.room_id,
            room: None,
            danmaku: commands::danmaku_status(&core).unwrap_or(DanmakuStatus {
                connection_id: None,
                room_id: None,
                state: nanabobo_core::models::DanmakuConnectionState::Idle,
                message: None,
            }),
            messages: Vec::new(),
            snapshots: stored.snapshots,
            account_error: None,
            room_error: None,
            danmaku_error: None,
            account_loading: loading,
            qr_polling: false,
            room_loading: false,
            danmaku_loading: false,
            restore_attempted: false,
            ready_images: HashSet::new(),
            pending_images: Arc::new(Mutex::new(Vec::new())),
            requested_images: HashSet::new(),
            core,
            runtime,
            dispatch,
            next_qr_poll: None,
            next_stats: None,
        })
    }

    pub fn take_pending_images(&self) -> Vec<crate::images::DecodedImage> {
        self.pending_images
            .lock()
            .map(|mut pending| pending.drain(..).collect())
            .unwrap_or_default()
    }

    pub fn has_image(&self, slot: &str) -> bool {
        self.ready_images.contains(slot)
    }

    fn request_image(&mut self, slot: &'static str, url: Option<&str>) {
        let Some(url) = url.map(str::trim).filter(|url| !url.is_empty()) else {
            self.ready_images.remove(slot);
            return;
        };
        let key = format!("{slot}:{url}");
        if !self.requested_images.insert(key) {
            return;
        }
        let core = Arc::clone(&self.core);
        let inbox = self.inbox.clone();
        let dispatch = Arc::clone(&self.dispatch);
        let url = url.to_owned();
        self.runtime.spawn(async move {
            if let Ok(bytes) = core.client.fetch_bytes(&url).await {
                if let Some(decoded) = crate::images::decode(slot, &bytes) {
                    inbox.push(AppEvent::ImageReady(decoded));
                    wake(&dispatch);
                }
            }
        });
    }

    pub fn bind_dispatch(&self, fire: Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut slot) = self.dispatch.lock() {
            *slot = Some(fire);
        }
    }

    pub fn authenticated(&self) -> bool {
        self.account
            .as_ref()
            .is_some_and(|status| status.authenticated && status.account.is_some())
    }

    pub fn account_name(&self) -> Option<&str> {
        self.account
            .as_ref()
            .and_then(|status| status.account.as_ref())
            .map(|account| account.username.as_str())
    }

    pub fn current_snapshots(&self) -> Vec<&Snapshot> {
        let Some(room_id) = self.room.as_ref().map(|room| room.room_id) else {
            return Vec::new();
        };
        self.snapshots
            .iter()
            .filter(|snapshot| snapshot.room_id == room_id)
            .collect()
    }

    pub fn viewer_series(&self) -> Vec<f64> {
        series(self, |snapshot| snapshot.viewer_count as f64)
    }

    pub fn follower_series(&self) -> Vec<f64> {
        series(self, |snapshot| snapshot.follower_count.unwrap_or(0) as f64)
    }

    pub fn next_wakeup(&self) -> Option<Instant> {
        [self.next_qr_poll, self.next_stats]
            .into_iter()
            .flatten()
            .min()
    }

    pub fn apply(&mut self, event: AppEvent) {
        match event {
            AppEvent::Navigate(page) => {
                self.page = page;
                self.confirm_clear = false;
            }
            AppEvent::OpenLogin => {
                self.login_open = true;
                if self.qr.is_none() {
                    self.start_qr();
                }
            }
            AppEvent::CloseLogin => {
                self.login_open = false;
                if !self.authenticated() {
                    self.cancel_qr();
                }
            }
            AppEvent::StartQr => self.start_qr(),
            AppEvent::PollQr => self.poll_qr(),
            AppEvent::Logout => self.logout(),
            AppEvent::RoomIdChanged(value) => self.room_id = value,
            AppEvent::QueryRoom => self.query_room(false),
            AppEvent::DisconnectRoom => self.disconnect_room(),
            AppEvent::StartDanmaku => self.start_danmaku(),
            AppEvent::StopDanmaku => self.stop_danmaku(),
            AppEvent::SelectStatsTab(tab) => self.stats_tab = tab,
            AppEvent::AskClearStats => self.confirm_clear = true,
            AppEvent::CancelClearStats => self.confirm_clear = false,
            AppEvent::ClearStats => self.clear_stats(),
            AppEvent::TickStats => {
                if self.room.is_some() {
                    self.query_room(true);
                }
            }
            AppEvent::Appearance(event) => {
                nana_ui::runtime::apply_appearance_event(
                    &mut self.theme,
                    &mut self.appearance,
                    event,
                );
                self.persist();
            }
            AppEvent::AuthStatus(result) => self.on_auth_status(result),
            AppEvent::QrStarted(result) => self.on_qr_started(result),
            AppEvent::QrPolled(result) => self.on_qr_polled(result),
            AppEvent::RoomLoaded { preserve, result } => self.on_room_loaded(preserve, result),
            AppEvent::DanmakuStatus(status) => {
                self.danmaku = status;
                self.danmaku_loading = false;
            }
            AppEvent::DanmakuMessage(message) => {
                self.messages.push(message);
                if self.messages.len() > MAX_DANMAKU {
                    let extra = self.messages.len() - MAX_DANMAKU;
                    self.messages.drain(..extra);
                }
            }
            AppEvent::ImageReady(image) => {
                self.ready_images.insert(image.slot.clone());
                if let Ok(mut pending) = self.pending_images.lock() {
                    pending.push(image);
                }
            }
        }
    }

    pub fn load_account(&self) {
        let core = Arc::clone(&self.core);
        let inbox = self.inbox.clone();
        let dispatch = Arc::clone(&self.dispatch);
        self.runtime.spawn(async move {
            inbox.push(AppEvent::AuthStatus(commands::auth_status(&core).await));
            wake(&dispatch);
        });
    }

    fn start_qr(&mut self) {
        self.account_error = None;
        self.account_loading = true;
        self.qr_phase = QrPhase::Idle;
        self.next_qr_poll = None;
        let core = Arc::clone(&self.core);
        let inbox = self.inbox.clone();
        let dispatch = Arc::clone(&self.dispatch);
        self.runtime.spawn(async move {
            inbox.push(AppEvent::QrStarted(commands::auth_qr_start(&core).await));
            wake(&dispatch);
        });
    }

    fn poll_qr(&mut self) {
        let Some(session_id) = self.qr.as_ref().map(|qr| qr.session_id.clone()) else {
            return;
        };
        if self.qr_polling {
            return;
        }
        self.qr_polling = true;
        let core = Arc::clone(&self.core);
        let inbox = self.inbox.clone();
        let dispatch = Arc::clone(&self.dispatch);
        self.runtime.spawn(async move {
            inbox.push(AppEvent::QrPolled(
                commands::auth_qr_poll(&core, session_id).await,
            ));
            wake(&dispatch);
        });
    }

    fn cancel_qr(&mut self) {
        self.qr = None;
        self.qr_phase = QrPhase::Idle;
        self.qr_polling = false;
        self.next_qr_poll = None;
        self.account_loading = false;
    }

    fn logout(&mut self) {
        if let Err(error) = commands::auth_logout(&self.core) {
            self.account_error = Some(error.message);
            return;
        }
        self.account = Some(AccountStatus {
            authenticated: false,
            account: None,
        });
        self.login_open = false;
        self.cancel_qr();
        self.ready_images.remove(crate::images::ACCOUNT_AVATAR);
        self.disconnect_room();
        self.restore_attempted = false;
    }

    fn query_room(&mut self, preserve: bool) {
        let room_id = self.room_id.trim().to_owned();
        if room_id.is_empty() || !room_id.chars().all(|ch| ch.is_ascii_digit()) {
            self.room_error = Some("请输入有效的直播间号。".to_owned());
            return;
        }
        self.room_error = None;
        self.room_loading = true;
        if !preserve {
            self.room = None;
        }
        let core = Arc::clone(&self.core);
        let inbox = self.inbox.clone();
        let dispatch = Arc::clone(&self.dispatch);
        self.runtime.spawn(async move {
            inbox.push(AppEvent::RoomLoaded {
                preserve,
                result: commands::room_get_info(&core, room_id).await,
            });
            wake(&dispatch);
        });
    }

    fn disconnect_room(&mut self) {
        self.stop_danmaku();
        self.room = None;
        self.room_id.clear();
        self.room_error = None;
        self.next_stats = None;
        self.ready_images.remove(crate::images::ROOM_AVATAR);
        self.ready_images.remove(crate::images::ROOM_COVER);
        self.persist();
    }

    fn start_danmaku(&mut self) {
        let Some(room_id) = self.room.as_ref().map(|room| room.room_id) else {
            self.danmaku_error = Some("请先连接一个直播间。".to_owned());
            return;
        };
        self.danmaku_error = None;
        self.danmaku_loading = true;
        self.messages.clear();
        match commands::danmaku_start(&self.core, room_id) {
            Ok(_) => {
                if let Ok(status) = commands::danmaku_status(&self.core) {
                    self.danmaku = status;
                }
            }
            Err(error) => {
                self.danmaku_error = Some(error.message);
                self.danmaku_loading = false;
            }
        }
    }

    fn stop_danmaku(&mut self) {
        if let Some(connection_id) = self.danmaku.connection_id.clone() {
            let _ = commands::danmaku_stop(&self.core, connection_id);
        }
        if let Ok(status) = commands::danmaku_status(&self.core) {
            self.danmaku = status;
        }
        self.danmaku_loading = false;
        self.messages.clear();
    }

    fn clear_stats(&mut self) {
        if let Some(room_id) = self.room.as_ref().map(|room| room.room_id) {
            self.snapshots.retain(|snapshot| snapshot.room_id != room_id);
            self.persist();
        }
        self.confirm_clear = false;
    }

    fn on_auth_status(&mut self, result: Result<AccountStatus, AppError>) {
        self.account_loading = false;
        match result {
            Ok(status) => {
                let avatar = status
                    .account
                    .as_ref()
                    .and_then(|account| account.avatar_url.clone());
                self.account = Some(status);
                self.account_error = None;
                self.request_image(crate::images::ACCOUNT_AVATAR, avatar.as_deref());
                self.maybe_restore_room();
            }
            Err(error) => self.account_error = Some(error.message),
        }
    }

    fn on_qr_started(&mut self, result: Result<QrStartResponse, AppError>) {
        self.account_loading = false;
        match result {
            Ok(qr) => {
                self.qr = Some(qr);
                self.qr_phase = QrPhase::Pending;
                self.account_error = None;
                self.next_qr_poll = Some(Instant::now() + QR_POLL);
            }
            Err(error) => {
                self.qr = None;
                self.account_error = Some(error.message);
                self.next_qr_poll = None;
            }
        }
    }

    fn on_qr_polled(&mut self, result: Result<AuthPollResponse, AppError>) {
        self.qr_polling = false;
        match result {
            Ok(AuthPollResponse::Pending) => {
                self.qr_phase = QrPhase::Pending;
                self.next_qr_poll = Some(Instant::now() + QR_POLL);
            }
            Ok(AuthPollResponse::Scanned) => {
                self.qr_phase = QrPhase::Scanned;
                self.next_qr_poll = Some(Instant::now() + QR_POLL);
            }
            Ok(AuthPollResponse::Expired) => {
                self.qr = None;
                self.qr_phase = QrPhase::Expired;
                self.next_qr_poll = None;
            }
            Ok(AuthPollResponse::Success { account }) => {
                let avatar = account.avatar_url.clone();
                self.account = Some(AccountStatus {
                    authenticated: true,
                    account: Some(account),
                });
                self.login_open = false;
                self.cancel_qr();
                self.request_image(crate::images::ACCOUNT_AVATAR, avatar.as_deref());
                self.maybe_restore_room();
            }
            Err(error) => {
                self.account_error = Some(error.message);
                if error.code == nanabobo_core::commands::ErrorCode::QrExpired {
                    self.qr = None;
                    self.qr_phase = QrPhase::Expired;
                    self.next_qr_poll = None;
                } else {
                    self.next_qr_poll = Some(Instant::now() + QR_POLL);
                }
            }
        }
    }

    fn on_room_loaded(&mut self, preserve: bool, result: Result<RoomInfo, AppError>) {
        self.room_loading = false;
        match result {
            Ok(info) => {
                self.room_id = info.room_id.to_string();
                self.record_snapshot(&info);
                let avatar = info.owner_avatar_url.clone();
                let cover = info.cover_url.clone();
                self.room = Some(info);
                self.room_error = None;
                self.next_stats = Some(Instant::now() + STATS_POLL);
                self.request_image(crate::images::ROOM_AVATAR, avatar.as_deref());
                self.request_image(crate::images::ROOM_COVER, cover.as_deref());
                self.persist();
            }
            Err(error) => {
                if !preserve {
                    self.room = None;
                }
                self.room_error = Some(error.message);
            }
        }
    }

    fn maybe_restore_room(&mut self) {
        if !self.authenticated() {
            self.stop_danmaku();
            self.room = None;
            self.restore_attempted = false;
            return;
        }
        if self.restore_attempted {
            return;
        }
        self.restore_attempted = true;
        if self.room_id.trim().is_empty() {
            return;
        }
        self.query_room(false);
    }

    fn record_snapshot(&mut self, info: &RoomInfo) {
        if self.snapshots.iter().any(|snapshot| {
            snapshot.room_id == info.room_id && snapshot.captured_at == info.fetched_at
        }) {
            return;
        }
        self.snapshots.push(Snapshot {
            room_id: info.room_id,
            captured_at: info.fetched_at,
            viewer_count: info.viewer_count,
            follower_count: info.follower_count,
            live_status: info.live_status.clone(),
        });
        trim_snapshots(&mut self.snapshots);
    }

    fn persist(&self) {
        let stored = StoredState {
            room_id: self.room_id.clone(),
            theme: match self.theme {
                ThemeMode::Light => "light".to_owned(),
                ThemeMode::Dark => "dark".to_owned(),
            },
            snapshots: self.snapshots.clone(),
        };
        if let Ok(bytes) = serde_json::to_vec_pretty(&stored) {
            let _ = std::fs::write(storage_path(), bytes);
        }
    }
}

fn series(session: &Session, pick: impl Fn(&Snapshot) -> f64 + Copy) -> Vec<f64> {
    let mut points: Vec<f64> = session
        .current_snapshots()
        .into_iter()
        .rev()
        .take(MAX_CHART_POINTS)
        .map(pick)
        .collect();
    points.reverse();
    if points.is_empty() {
        if let Some(room) = &session.room {
            points.push(pick(&Snapshot {
                room_id: room.room_id,
                captured_at: room.fetched_at,
                viewer_count: room.viewer_count,
                follower_count: room.follower_count,
                live_status: room.live_status.clone(),
            }));
        }
    }
    points
}

fn trim_snapshots(snapshots: &mut Vec<Snapshot>) {
    let mut counts = std::collections::HashMap::<u64, usize>::new();
    let mut kept = Vec::new();
    for snapshot in snapshots.iter().rev() {
        let count = counts.entry(snapshot.room_id).or_insert(0);
        if *count >= MAX_SNAPSHOTS_PER_ROOM {
            continue;
        }
        *count += 1;
        kept.push(snapshot.clone());
    }
    kept.reverse();
    *snapshots = kept;
}

fn load_store() -> StoredState {
    std::fs::read(storage_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn storage_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(STORAGE_FILE)
}

pub fn live_label(status: &str) -> &'static str {
    match status {
        "live" => "直播中",
        "round" => "轮播中",
        _ => "未开播",
    }
}

pub fn danmaku_label(status: &DanmakuStatus) -> &'static str {
    use nanabobo_core::models::DanmakuConnectionState::*;
    match status.state {
        Connected => "已连接",
        Connecting => "连接中",
        Reconnecting => "重连中",
        Error => "连接异常",
        Stopped => "已停止",
        Idle => "未连接",
    }
}

#[cfg(test)]
mod tests {
    use super::{Snapshot, trim_snapshots};

    #[test]
    fn trims_snapshots_per_room() {
        let mut snapshots = (0..2_010)
            .map(|index| Snapshot {
                room_id: 1,
                captured_at: index,
                viewer_count: index,
                follower_count: None,
                live_status: "live".into(),
            })
            .collect();
        trim_snapshots(&mut snapshots);
        assert_eq!(snapshots.len(), 2_000);
        assert_eq!(snapshots.first().map(|item| item.captured_at), Some(10));
    }
}
