#[cfg(test)]
mod acceptance;
mod data;
mod desktop;
mod feed;
#[cfg(test)]
mod input_tests;
pub use desktop::DesktopDanmakuView;

use crate::images::{ACCOUNT_AVATAR, ROOM_AVATAR};
use crate::session::{
    danmaku_label, live_label, timestamp, AppEvent, Inbox, Page, QrPhase, Revisions, Session,
};
use feed::Feed;
use nana_ui::runtime::*;
use nana_ui::ButtonKind;
use std::sync::Arc;

pub struct Shell {
    shell: Entity<DesktopShell>,
    document_id: DocumentId,
    nav: Entity<ScrollView>,
    footer: Entity<SidebarFooter>,
    banner: Entity<Stack>,
    primary: Entity<Stack>,
    header: Option<Entity<Stack>>,
    toolbar: Option<Entity<Stack>>,
    content: Option<Entity<Stack>>,
    login: Option<(Entity<Dialog>, Entity<Stack>)>,
    clear: Option<(Entity<Dialog>, Entity<Stack>)>,
    stats_tabs: Option<Entity<Tabs>>,
    last: Option<(Page, Revisions)>,
    connection: Option<nanabobo_core::models::DanmakuStatus>,
}
impl Shell {
    pub fn mount(
        document: &mut RuntimeDocument,
        session: &Session,
    ) -> Result<Self, FrameworkError> {
        let document_id = document.document();
        let cx = document.context_mut();
        cx.set_theme(session.theme)?;
        let (shell, nav, footer, banner, primary) = cx.build(document_id, |ui| {
            let nav = ui.leaf(SidebarFrame::vertical_body_scroll());
            let footer = ui.leaf(SidebarFooter::new());
            let sidebar = ui.leaf(
                SidebarFrame::new()
                    .body(nav.stable_id())
                    .footer(footer.stable_id()),
            );
            ui.nest(sidebar, |ui| {
                ui.adopt(nav);
                ui.adopt(footer);
            });
            let banner = ui.leaf(Stack::column(6.0));
            let primary = ui.leaf(Stack::fill_column(16.0));
            let page = ui.leaf(Stack::fill_column(12.0).padding(20.0));
            ui.nest(page, |ui| {
                ui.adopt(banner);
                ui.adopt(primary);
            });
            let shell = ui.child(
                "shell",
                DesktopShell::new()
                    .title("Nana播播工具箱")
                    .navigation(sidebar.stable_id())
                    .primary(page.stable_id()),
            );
            (shell, nav, footer, banner, primary)
        })?;
        let mut s = Self {
            shell,
            document_id,
            nav,
            footer,
            banner,
            primary,
            header: None,
            toolbar: None,
            content: None,
            login: None,
            clear: None,
            stats_tabs: None,
            last: None,
            connection: None,
        };
        s.sync(document, session)?;
        Ok(s)
    }
    pub fn sync(
        &mut self,
        document: &mut RuntimeDocument,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let cx = document.context_mut();
        let previous = self.last;
        let page_changed = previous.is_none_or(|(p, _)| p != session.page);
        let shell_changed = previous.is_none_or(|(_, r)| r.shell != session.revisions.shell);
        let room_changed = page_changed
            || shell_changed
            || previous.is_none_or(|(_, r)| r.room != session.revisions.room);
        let desktop_changed =
            room_changed || previous.is_none_or(|(_, r)| r.desktop != session.revisions.desktop);
        let stats_changed = page_changed
            || shell_changed
            || previous.is_none_or(|(_, r)| r.stats != session.revisions.stats);
        if shell_changed || page_changed {
            cx.set_theme(session.theme)?;
            self.sync_nav(cx, session)?;
            self.sync_footer(cx, session)?;
            self.sync_banner(cx, session)?;
        }
        if page_changed {
            self.stats_tabs = None;
            self.header = None;
            self.toolbar = None;
            self.content = None;
        }
        match session.page {
            Page::Workbench => {
                if self.header.is_none() {
                    let mut parts = None;
                    cx.mount(self.primary, |ui| {
                        let header = ui.child("room-context", Stack::column(8.0))?;
                        let toolbar = ui.child("overview-data", Stack::column(6.0))?;
                        let content = ui.child("desktop-launcher", Stack::fill_column(0.0))?;
                        parts = Some((header, toolbar, content));
                        Ok(())
                    })?;
                    let (header, toolbar, content) = parts.expect("workbench");
                    self.header = Some(header);
                    self.toolbar = Some(toolbar);
                    self.content = Some(content);
                }
                if room_changed {
                    self.sync_room(cx, session)?;
                }
                if desktop_changed
                    || stats_changed
                    || self.connection.as_ref() != Some(&session.danmaku.status)
                {
                    self.sync_overview(cx, session, room_changed || stats_changed)?;
                }
            }
            Page::Stats => {
                if stats_changed {
                    data::mount_data(cx, self.primary, session, &mut self.stats_tabs)?;
                }
            }
            Page::Settings => {
                if shell_changed || page_changed {
                    data::mount_settings(cx, self.primary, session)?;
                }
            }
        }
        if page_changed
            || shell_changed
            || previous.is_none_or(|(_, r)| r.overlay != session.revisions.overlay)
        {
            self.sync_overlays(cx, session)?;
        }
        cx.assemble_desktop_shell(self.shell)?;
        self.connection = Some(session.danmaku.status.clone());
        self.last = Some((session.page, session.revisions));
        Ok(())
    }
    pub fn needs_layout_sync(&self, _document: &RuntimeDocument) -> bool {
        false
    }
    fn sync_nav(&self, cx: &mut AppContext, s: &Session) -> Result<(), FrameworkError> {
        let mut rows = Vec::new();
        cx.mount(self.nav, |ui| {
            for (key, label, page) in [
                ("workbench", "概览", Page::Workbench),
                ("data", "数据", Page::Stats),
            ] {
                rows.push((ui.child(key, nav_row(label, s.page == page))?, page));
            }
            Ok(())
        })?;
        for (row, page) in rows {
            action(cx, row, s.inbox.clone(), AppEvent::Navigate(page))?;
        }
        Ok(())
    }
    fn sync_footer(&self, cx: &mut AppContext, s: &Session) -> Result<(), FrameworkError> {
        let mut account = None;
        let mut settings = None;
        cx.mount(self.footer, |ui| {
            if s.authenticated() {
                let name = s.account_name().unwrap_or("已登录");
                ui.child(
                    "avatar",
                    Avatar::new(if s.has_image(ACCOUNT_AVATAR) {
                        ACCOUNT_AVATAR
                    } else {
                        ""
                    })
                    .size(28.0)
                    .label(name),
                )?;
                let mut account_name = Text::new(name);
                let layout = Arc::make_mut(&mut account_name.style.layout);
                layout.max_width = Some(LengthSpec::Px(76.0));
                layout.white_space_nowrap = true;
                layout.text_overflow_ellipsis = true;
                ui.child("name", account_name)?;
                account = Some((
                    ui.child("logout", Button::new("退出登录").kind(ButtonKind::Ghost))?,
                    AppEvent::Logout,
                ));
            } else {
                account = Some((
                    ui.child(
                        "login",
                        Button::new("登录 B 站").loading(s.auth.loading() && !s.auth.open),
                    )?,
                    AppEvent::OpenLogin,
                ));
            }
            settings = Some(ui.child("settings", nav_row("设置", s.page == Page::Settings))?);
            Ok(())
        })?;
        if let Some((node, event)) = account {
            action(cx, node, s.inbox.clone(), event)?;
        }
        action(
            cx,
            settings.expect("settings"),
            s.inbox.clone(),
            AppEvent::Navigate(Page::Settings),
        )
    }
    fn sync_banner(&self, cx: &mut AppContext, s: &Session) -> Result<(), FrameworkError> {
        let mut retry = None;
        cx.mount(self.banner, |ui| {
            if let Some(error) = &s.storage_error {
                ui.child(
                    "storage-error",
                    ValidationMessage::new(error.clone(), ValidationIntent::Warning),
                )?;
                retry = Some(ui.child("retry-storage", Button::new("重试保存"))?);
            }
            if !s.auth.open {
                if let Some(error) = s.auth.error() {
                    ui.child(
                        "account-error",
                        ValidationMessage::new(error, ValidationIntent::Danger),
                    )?;
                }
            }
            Ok(())
        })?;
        if let Some(node) = retry {
            action(cx, node, s.inbox.clone(), AppEvent::RetryStore)?;
        }
        Ok(())
    }
    fn sync_room(&self, cx: &mut AppContext, s: &Session) -> Result<(), FrameworkError> {
        let mut actions = Vec::new();
        let mut field = None;
        cx.mount(self.header.expect("header"), |ui| {
            ui.child("heading", heading("概览"))?;
            if !s.authenticated() {
                ui.child(
                    "welcome",
                    Text::new("登录 B 站，查看直播间数据并启动桌面弹幕。"),
                )?;
                actions.push((
                    ui.child(
                        "login",
                        Button::new("登录 B 站")
                            .kind(ButtonKind::Primary)
                            .loading(s.auth.loading()),
                    )?,
                    AppEvent::OpenLogin,
                ));
                return Ok(());
            }
            if let Some(room) = &s.room.info {
                ui.with_child("context", Stack::column(6.0), |ui| {
                    ui.with_child(
                        "title-row",
                        Stack::row(10.0).width(LengthSpec::Fill),
                        |ui| {
                            ui.child(
                                "avatar",
                                Avatar::new(if s.has_image(ROOM_AVATAR) {
                                    ROOM_AVATAR
                                } else {
                                    ""
                                })
                                .size(40.0)
                                .label(room.owner_name.as_deref().unwrap_or("主播")),
                            )?;
                            let mut title = Text::new(&room.title);
                            let layout = Arc::make_mut(&mut title.style.layout);
                            layout.width = Some(LengthSpec::Px(300.0));
                            layout.flex_grow = Some(1.0);
                            layout.flex_shrink = Some(1.0);
                            layout.min_width = Some(LengthSpec::Px(0.0));
                            layout.word_break = Some(WordBreakSpec::BreakWord);
                            ui.child("title", title)?;
                            Ok(())
                        },
                    )?;
                    ui.child(
                        "details",
                        muted(format!(
                            "{} · 房间 {} · {}",
                            room.owner_name.as_deref().unwrap_or("主播"),
                            room.room_id,
                            live_label(&room.live_status)
                        )),
                    )?;
                    ui.with_child("room-actions", Stack::row(8.0), |ui| {
                        actions.push((
                            ui.child("switch", Button::new("切换直播间"))?,
                            AppEvent::EditRoom,
                        ));
                        actions.push((
                            ui.child("disconnect", Button::new("断开直播间"))?,
                            AppEvent::DisconnectRoom,
                        ));
                        ui.child(
                            "updated",
                            muted(format!("更新于 {}", timestamp(room.fetched_at))),
                        )?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
            if s.room.info.is_none() || s.room.editing {
                ui.with_child("room-form", Stack::row(8.0).width(LengthSpec::Fill), |ui| {
                    let mut input = TextInput::new(&s.room.input).label("直播间号");
                    let mut style = input.style.clone();
                    let layout = Arc::make_mut(&mut style.layout);
                    layout.width = Some(LengthSpec::Px(240.0));
                    layout.min_width = Some(LengthSpec::Px(100.0));
                    layout.flex_grow = Some(1.0);
                    layout.flex_shrink = Some(1.0);
                    input = input.style(style);
                    input.placeholder = "输入直播间号".into();
                    input.invalid = s.room.error().is_some();
                    input.disabled = s.room.loading();
                    field = Some(ui.child("room-id", input)?);
                    actions.push((
                        ui.child(
                            "connect",
                            Button::new("连接直播间")
                                .kind(ButtonKind::Primary)
                                .loading(s.room.loading()),
                        )?,
                        AppEvent::QueryRoom,
                    ));
                    if s.room.info.is_some() {
                        actions.push((
                            ui.child("cancel", Button::new("取消"))?,
                            AppEvent::CancelEditRoom,
                        ));
                    }
                    Ok(())
                })?;
            }
            if let Some(error) = s.room.error() {
                ui.child(
                    "room-error",
                    ValidationMessage::new(error, ValidationIntent::Danger),
                )?;
                if s.room.info.is_some() && !s.room.editing {
                    actions.push((
                        ui.child("retry-room", Button::new("重试刷新"))?,
                        AppEvent::RefreshRoom,
                    ));
                }
            }
            Ok(())
        })?;
        for (node, event) in actions {
            action(cx, node, s.inbox.clone(), event)?;
        }
        if let Some(field) = field {
            let inbox = s.inbox.clone();
            cx.on_keyed(field, "input", move |_, event: &TextChanged, _| {
                inbox.push(AppEvent::RoomIdChanged(event.value.clone()));
            })?;
            let inbox = s.inbox.clone();
            cx.on_keyed(field, "submit", move |_, event: &TextSubmitted, _| {
                inbox.push(AppEvent::RoomIdChanged(event.value.clone()));
                inbox.push(AppEvent::QueryRoom);
            })?;
        }
        Ok(())
    }
    fn sync_overview(
        &self,
        cx: &mut AppContext,
        s: &Session,
        metrics_changed: bool,
    ) -> Result<(), FrameworkError> {
        let mut actions = Vec::new();
        if metrics_changed {
            cx.mount(self.toolbar.expect("overview data"), |ui| {
                let Some(room) = &s.room.info else {
                    return Ok(());
                };
                ui.with_child("metrics", Stack::row(24.0), |ui| {
                    ui.child("viewers", heading(format!("人气  {}", room.viewer_count)))?;
                    ui.child(
                        "followers",
                        heading(format!(
                            "粉丝  {}",
                            room.follower_count
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "暂无数据".into())
                        )),
                    )?;
                    Ok(())
                })?;
                let samples: Vec<_> = s
                    .stats
                    .snapshots
                    .iter()
                    .filter(|v| v.room_id == room.room_id)
                    .rev()
                    .take(30)
                    .collect();
                if !samples.is_empty() {
                    let mut chart = TimeSeriesChart::from_samples(samples.iter().rev().map(|v| {
                        (
                            (v.captured_at.saturating_mul(1000)) as i64,
                            Some(v.viewer_count as f64),
                        )
                    }))
                    .label("最近人气趋势")
                    .unit("人气")
                    .time_labels(
                        timestamp(samples.last().unwrap().captured_at),
                        timestamp(samples[0].captured_at),
                    );
                    Arc::make_mut(&mut chart.style.layout).height = Some(LengthSpec::Px(140.0));
                    ui.child("recent-trend", chart)?;
                }
                actions.push((
                    ui.child("view-data", Button::new("查看数据"))?,
                    AppEvent::SelectHistoryRoom(room.room_id),
                ));
                Ok(())
            })?;
            for (node, event) in actions.drain(..) {
                let inbox = s.inbox.clone();
                cx.on_keyed(node, "activate", move |_, _: &Activate, _| {
                    inbox.push(event.clone());
                    inbox.push(AppEvent::Navigate(Page::Stats));
                })?;
            }
        }
        cx.mount(self.content.expect("launcher"), |ui| {
            if s.room.info.is_none() {
                return Ok(());
            }
            ui.child("title", heading("桌面弹幕"))?;
            use crate::session::DesktopDanmakuPhase::*;
            let phase = s.desktop.phase;
            let label = match phase {
                Closed => "未启动",
                Creating => "正在打开",
                Adjusting | Locked => danmaku_label(&s.danmaku.status),
            };
            ui.child("status", Text::new(label))?;
            ui.with_child("actions", Stack::row(8.0), |ui| {
                if matches!(phase, Closed | Creating) {
                    actions.push((
                        ui.child(
                            "start",
                            Button::new("启动桌面弹幕")
                                .kind(ButtonKind::Primary)
                                .loading(phase == Creating),
                        )?,
                        AppEvent::OpenDesktopDanmaku,
                    ));
                } else {
                    actions.push((
                        ui.child(
                            "adjust",
                            Button::new(if phase == Locked {
                                "解锁并调整"
                            } else {
                                "调整弹幕"
                            }),
                        )?,
                        AppEvent::AdjustDesktopDanmaku,
                    ));
                    actions.push((
                        ui.child("close", Button::new("关闭桌面弹幕"))?,
                        AppEvent::CloseDesktopDanmaku,
                    ));
                }
                Ok(())
            })?;
            if let Some(error) = s
                .desktop
                .error
                .as_ref()
                .or(s.danmaku.status.message.as_ref())
            {
                ui.child(
                    "error",
                    ValidationMessage::new(error.as_str(), ValidationIntent::Warning),
                )?;
            }
            Ok(())
        })?;
        for (node, event) in actions {
            action(cx, node, s.inbox.clone(), event)?;
        }
        Ok(())
    }
    fn sync_overlays(&mut self, cx: &mut AppContext, s: &Session) -> Result<(), FrameworkError> {
        let mut overlays = Vec::new();
        if s.auth.open {
            if self.login.is_none() {
                self.login = Some(self.dialog(cx, "登录 B 站")?);
            }
            let (dialog, body) = self.login.expect("login");
            let mut refresh = None;
            let mut cancel = None;
            cx.mount(body, |ui| {
                if s.auth.qr.is_none() && s.auth.loading() {
                    ui.child("loading", Text::new("正在生成二维码…"))?;
                } else {
                    if let Some(qr) = &s.auth.qr {
                        if matches!(s.auth.phase, QrPhase::Pending | QrPhase::Scanned) {
                            match QrCode::encode(qr.payload.as_bytes(), 224.0) {
                                Ok(code) => {
                                    ui.child("qr", code.label("B站登录二维码"))?;
                                }
                                Err(_) => {
                                    ui.child(
                                        "encode-error",
                                        Text::new("二维码无法显示，请重新生成。"),
                                    )?;
                                }
                            }
                        }
                    }
                    ui.child(
                        "phase",
                        Text::new(match s.auth.phase {
                            QrPhase::Scanned => "已扫描，请在手机上确认登录。",
                            QrPhase::Expired => "二维码已过期，请重新生成。",
                            QrPhase::Failed => "登录暂时未完成，请重新尝试。",
                            _ => "请使用 B 站手机客户端扫描二维码。",
                        }),
                    )?;
                    refresh = Some(ui.child("refresh", Button::new("重新生成二维码"))?);
                }
                if let Some(error) = s.auth.error() {
                    ui.child(
                        "error",
                        ValidationMessage::new(error, ValidationIntent::Danger),
                    )?;
                }
                cancel = Some(ui.child("cancel-login", Button::new("取消登录"))?);
                Ok(())
            })?;
            if let Some(button) = refresh {
                action(cx, button, s.inbox.clone(), AppEvent::StartQr)?;
            }
            if let Some(button) = cancel {
                action(cx, button, s.inbox.clone(), AppEvent::CloseLogin)?;
            }
            overlays.push(dialog.stable_id());
        }
        if let Some(id) = s.stats.confirm_clear {
            if self.clear.is_none() {
                self.clear = Some(self.dialog(cx, "清理历史记录")?);
            }
            let (dialog, body) = self.clear.expect("clear");
            let mut actions = Vec::new();
            cx.mount(body, |ui| {
                ui.child(
                    "warning",
                    Text::new(format!(
                        "删除「{}」的全部本地历史记录？此操作无法撤销。",
                        s.stats.room_label(id)
                    )),
                )?;
                ui.with_child("actions", Stack::row(8.0), |ui| {
                    actions.push((
                        ui.child("cancel", Button::new("取消"))?,
                        AppEvent::CancelClearStats,
                    ));
                    actions.push((
                        ui.child("confirm", Button::new("删除记录").kind(ButtonKind::Danger))?,
                        AppEvent::ClearStats,
                    ));
                    Ok(())
                })?;
                Ok(())
            })?;
            for (node, event) in actions {
                action(cx, node, s.inbox.clone(), event)?;
            }
            overlays.push(dialog.stable_id());
        }
        // Shell overlay slots retain children; activation is a separate Runtime
        // transaction that establishes visibility, modal focus and dismissal.
        let active = overlays.last().copied();
        if active.is_none() {
            if let Some(host) = cx.read(self.shell, |shell| shell.overlay)? {
                let host = Entity::<OverlayHost>::from_stable_id(host);
                // Session already closed the dialog; dismissing its surface must
                // not enqueue another close that could affect a later opening.
                cx.on_keyed(host, "close", |_, _: &OverlayClosing, _| {})?;
                cx.dismiss_overlay(host)?;
            }
        }
        cx.update_component(self.shell, |shell, _| shell.overlays = overlays)?;
        cx.assemble_desktop_shell(self.shell)?;
        if let Some(active) = active {
            let host = cx
                .read(self.shell, |shell| shell.overlay)?
                .expect("assembled overlay host");
            let host = Entity::<OverlayHost>::from_stable_id(host);
            let event = if self
                .login
                .is_some_and(|(dialog, _)| dialog.stable_id() == active)
            {
                AppEvent::CloseLogin
            } else {
                AppEvent::CancelClearStats
            };
            let inbox = s.inbox.clone();
            // Runtime emits closing on the host, with the dialog as its root.
            cx.on_keyed(host, "close", move |_, closing: &OverlayClosing, _| {
                if closing.root == active {
                    inbox.push(event.clone());
                }
            })?;
            cx.activate_overlay(host, Entity::<Dialog>::from_stable_id(active))?;
        }
        Ok(())
    }
    fn dialog(
        &self,
        cx: &mut AppContext,
        title: &str,
    ) -> Result<(Entity<Dialog>, Entity<Stack>), FrameworkError> {
        cx.build_detached(self.document_id, |ui| {
            let body = ui.leaf(Stack::column(12.0).max_width(440.0));
            let mut surface = Dialog::new(title);
            surface.slots.body = Some(body.stable_id());
            let dialog = ui.leaf(surface);
            ui.nest(dialog, |ui| ui.adopt(body));
            (dialog, body)
        })
    }
}
pub(super) fn action<V: View>(
    cx: &mut AppContext,
    node: Entity<V>,
    inbox: Inbox,
    event: AppEvent,
) -> Result<(), FrameworkError> {
    cx.on_keyed(node, "activate", move |_, _: &Activate, _| {
        inbox.push(event.clone())
    })
}
pub(super) fn heading(value: impl Into<String>) -> Text {
    let mut text = Text::new(value);
    let style = Arc::make_mut(&mut text.style.layout);
    style.font_size = Some(20.0);
    style.font_weight = Some(600);
    text
}
pub(super) fn muted(value: impl Into<String>) -> Text {
    let mut text = Text::new(value);
    text.style.foreground = Some(nana_ui::runtime::SemanticColorRole::Muted);
    text
}
fn nav_row(label: &str, active: bool) -> SidebarRow {
    SidebarRow::new(label).state(if active {
        SidebarRowState::Active
    } else {
        SidebarRowState::Idle
    })
}
