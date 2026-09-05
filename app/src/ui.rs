use std::collections::HashSet;

use nana_ui::runtime::{
    AboutMetadata, AboutSection, Activate, AppearanceSection, Avatar, Button, DesktopShell,
    Dialog, DocumentId, EmptyState, Entity, FrameworkError, GpuTextureView, LabeledValue, ListItem,
    OverlayClosing, QrCode, RuntimeDocument, ScrollAxes, ScrollView, SidebarFrame, SidebarFooter,
    SidebarRow, SidebarRowState, Stack, StatusBadge, StatusTone, TabOption, Tabs, TabsEvent, Text,
    TextChanged, TextInput, TimeSeriesChart, ValidationIntent, ValidationMessage,
};
use nana_ui::{AppearanceEvent, ButtonKind};

use crate::images::{ACCOUNT_AVATAR, ROOM_AVATAR, ROOM_COVER};
use crate::session::{
    danmaku_label, live_label, AppEvent, Inbox, Page, QrPhase, Session, StatsTab, Wake,
};

pub struct Shell {
    shell: Entity<DesktopShell>,
    document_id: DocumentId,
    nav_body: Entity<ScrollView>,
    footer: Entity<SidebarFooter>,
    inspector: Entity<Stack>,
    primary: Entity<Stack>,
    login_dialog: Option<Entity<Dialog>>,
    login_body: Option<Entity<Stack>>,
    bound: HashSet<nana_ui::runtime::StableNodeId>,
}

impl Shell {
    pub fn mount(document: &mut RuntimeDocument, session: &Session) -> Result<Self, FrameworkError> {
        let document_id = document.document();
        let cx = document.context_mut();
        let _ = cx.set_theme(session.theme);
        let (shell, nav_body, footer, inspector, primary) = cx.build(document_id, |ui| {
            let home = ui.child("nav-home", nav_row("首页", session.page == Page::Home));
            let assistant = ui.child(
                "nav-assistant",
                nav_row("主播助手", session.page == Page::Assistant),
            );
            let stats = ui.child("nav-stats", nav_row("数据", session.page == Page::Stats));
            let nav_body = ui.leaf(SidebarFrame::vertical_body_scroll());
            ui.nest(nav_body, |ui| {
                ui.adopt(home);
                ui.adopt(assistant);
                ui.adopt(stats);
            });
            let footer = ui.leaf(SidebarFooter::new());
            let frame = ui.leaf(
                SidebarFrame::new()
                    .body(nav_body.stable_id())
                    .footer(footer.stable_id()),
            );
            ui.nest(frame, |ui| {
                ui.adopt(nav_body);
                ui.adopt(footer);
            });
            let inspector = ui.leaf(Stack::fill_column(10.0));
            let primary = ui.leaf(Stack::fill_column(12.0));
            let shell = ui.child(
                "shell",
                DesktopShell::new()
                    .title("Nana播播工具箱")
                    .navigation(frame.stable_id())
                    .inspector(inspector.stable_id())
                    .primary(primary.stable_id()),
            );
            bind_nav(ui, home, session.inbox.clone(), Page::Home);
            bind_nav(ui, assistant, session.inbox.clone(), Page::Assistant);
            bind_nav(ui, stats, session.inbox.clone(), Page::Stats);
            (shell, nav_body, footer, inspector, primary)
        })?;
        cx.assemble_desktop_shell(shell)?;
        let mut shell = Self {
            shell,
            document_id,
            nav_body,
            footer,
            inspector,
            primary,
            login_dialog: None,
            login_body: None,
            bound: HashSet::new(),
        };
        shell.sync(document, session)?;
        Ok(shell)
    }

    pub fn sync(
        &mut self,
        document: &mut RuntimeDocument,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let cx = document.context_mut();
        let _ = cx.set_theme(session.theme);
        self.sync_nav(cx, session)?;
        self.sync_footer(cx, session)?;
        self.sync_inspector(cx, session)?;
        self.sync_primary(cx, session)?;
        self.sync_overlay(cx, session)?;
        cx.assemble_desktop_shell(self.shell)?;
        Ok(())
    }

    fn sync_nav(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let _ = session;
        cx.mount(self.nav_body, |ui| {
            ui.child("nav-home", nav_row("首页", session.page == Page::Home))?;
            ui.child(
                "nav-assistant",
                nav_row("主播助手", session.page == Page::Assistant),
            )?;
            ui.child("nav-stats", nav_row("数据", session.page == Page::Stats))?;
            Ok(())
        })
    }

    fn sync_footer(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let inbox = session.inbox.clone();
        let mut login = None;
        let mut logout = None;
        let mut settings = None;
        cx.mount(self.footer, |ui| {
            if session.authenticated() {
                let name = session.account_name().unwrap_or("已登录");
                ui.child(
                    "avatar",
                    Avatar::new(image_resource(session, ACCOUNT_AVATAR))
                        .size(28.0)
                        .label(name),
                )?;
                ui.child("name", Text::new(name))?;
                logout = Some(ui.child(
                    "logout",
                    Button::new("退出登录")
                        .kind(ButtonKind::Ghost)
                        .loading(session.account_loading),
                )?);
            } else {
                login = Some(ui.child("login", SidebarRow::new("登录 B 站"))?);
            }
            settings = Some(ui.child(
                "settings",
                nav_row("设置", session.page == Page::Settings),
            )?);
            Ok(())
        })?;
        if let Some(entity) = login {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::OpenLogin)?;
        }
        if let Some(entity) = logout {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::Logout)?;
        }
        if let Some(entity) = settings {
            bind_once(
                cx,
                &mut self.bound,
                entity,
                inbox,
                AppEvent::Navigate(Page::Settings),
            )?;
        }
        Ok(())
    }

    fn sync_inspector(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let inbox = session.inbox.clone();
        let mut query = None;
        let mut disconnect = None;
        let mut input = None;
        cx.mount(self.inspector, |ui| {
            if !session.authenticated() {
                ui.child(
                    "empty",
                    EmptyState::new("登录后连接直播间")
                        .message("在侧栏底部登录 B 站账号。"),
                )?;
                return Ok(());
            }
            if let Some(info) = &session.room {
                let owner = info
                    .owner_name
                    .clone()
                    .unwrap_or_else(|| format!("UID {}", info.owner_id));
                ui.child(
                    "avatar",
                    Avatar::new(image_resource(session, ROOM_AVATAR))
                        .size(56.0)
                        .label(owner.as_str()),
                )?;
                if session.has_image(ROOM_COVER) {
                    ui.child(
                        "cover",
                        GpuTextureView::new(ROOM_COVER)
                            .with_corner_radius(8.0)
                            .contain(),
                    )?;
                }
                ui.child("owner", Text::new(owner))?;
                ui.child(
                    "status",
                    LabeledValue::new("状态", live_label(&info.live_status)),
                )?;
                ui.child(
                    "viewers",
                    LabeledValue::new("在线", info.viewer_count.to_string()),
                )?;
                ui.child(
                    "followers",
                    LabeledValue::new(
                        "关注",
                        info.follower_count
                            .map(|count| count.to_string())
                            .unwrap_or_else(|| "暂无".to_owned()),
                    ),
                )?;
                ui.child(
                    "room",
                    LabeledValue::new("房间号", info.room_id.to_string()),
                )?;
                disconnect = Some(ui.child(
                    "disconnect",
                    Button::new("切换直播间").kind(ButtonKind::Ghost),
                )?);
            } else {
                let mut field = TextInput::new(session.room_id.clone());
                field.placeholder = "输入直播间号".into();
                field.invalid = session.room_error.is_some();
                input = Some(ui.child("room-id", field)?);
                query = Some(ui.child(
                    "query",
                    Button::new("连接直播间")
                        .kind(ButtonKind::Primary)
                        .loading(session.room_loading),
                )?);
            }
            if let Some(error) = &session.room_error {
                ui.child(
                    "error",
                    ValidationMessage::new(error.clone(), ValidationIntent::Danger),
                )?;
            }
            Ok(())
        })?;
        if let Some(entity) = input {
            if self.bound.insert(entity.stable_id()) {
                let inbox = inbox.clone();
                cx.on(entity, move |_input, event: &TextChanged, cx| {
                    inbox.push(AppEvent::RoomIdChanged(event.value.clone()));
                    cx.dispatch_program(Wake);
                })?;
            }
        }
        if let Some(entity) = query {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::QueryRoom)?;
        }
        if let Some(entity) = disconnect {
            bind_once(
                cx,
                &mut self.bound,
                entity,
                inbox,
                AppEvent::DisconnectRoom,
            )?;
        }
        Ok(())
    }

    fn sync_primary(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        match session.page {
            Page::Home => self.sync_home(cx, session),
            Page::Assistant => self.sync_assistant(cx, session),
            Page::Stats => self.sync_stats(cx, session),
            Page::Settings => self.sync_settings(cx, session),
        }
    }

    fn sync_home(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        cx.mount(self.primary, |ui| {
            if !session.authenticated() {
                ui.child(
                    "empty",
                    EmptyState::new("登录后开始使用").message("在侧栏底部登录 B 站账号。"),
                )?;
            } else if let Some(info) = &session.room {
                ui.child(
                    "live",
                    LabeledValue::new("直播状态", live_label(&info.live_status)),
                )?;
                ui.child(
                    "viewers",
                    LabeledValue::new("在线", info.viewer_count.to_string()),
                )?;
                ui.child(
                    "followers",
                    LabeledValue::new(
                        "关注",
                        info.follower_count
                            .map(|count| count.to_string())
                            .unwrap_or_else(|| "暂无".to_owned()),
                    ),
                )?;
                ui.child(
                    "danmaku",
                    LabeledValue::new("弹幕", danmaku_label(&session.danmaku)),
                )?;
                ui.child(
                    "trend",
                    TimeSeriesChart::new(session.viewer_series()).label("在线人数"),
                )?;
            } else {
                ui.child(
                    "empty",
                    EmptyState::new("连接直播间").message("在右侧输入直播间号。"),
                )?;
            }
            Ok(())
        })
    }

    fn sync_assistant(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let inbox = session.inbox.clone();
        let mut start = None;
        let mut stop = None;
        cx.mount(self.primary, |ui| {
            if !session.authenticated() {
                ui.child(
                    "empty",
                    EmptyState::new("登录后使用弹幕助手")
                        .message("在侧栏底部登录 B 站账号。"),
                )?;
                return Ok(());
            }
            let Some(info) = &session.room else {
                ui.child(
                    "empty",
                    EmptyState::new("先连接一个直播间")
                        .message("在右侧连接直播间后再开始监控。"),
                )?;
                return Ok(());
            };
            ui.child("title", Text::new(info.title.clone()))?;
            ui.child(
                "badge",
                StatusBadge::new(
                    danmaku_label(&session.danmaku),
                    match session.danmaku.state {
                        nanabobo_core::models::DanmakuConnectionState::Connected => {
                            StatusTone::Success
                        }
                        nanabobo_core::models::DanmakuConnectionState::Error => StatusTone::Danger,
                        nanabobo_core::models::DanmakuConnectionState::Connecting
                        | nanabobo_core::models::DanmakuConnectionState::Reconnecting => {
                            StatusTone::Warning
                        }
                        _ => StatusTone::Neutral,
                    },
                ),
            )?;
            if session.danmaku.connection_id.is_some() {
                stop = Some(ui.child(
                    "stop",
                    Button::new("停止")
                        .kind(ButtonKind::Ghost)
                        .loading(session.danmaku_loading),
                )?);
            } else {
                start = Some(ui.child(
                    "start",
                    Button::new("开始监控")
                        .kind(ButtonKind::Primary)
                        .loading(session.danmaku_loading),
                )?);
            }
            if session.messages.is_empty() {
                ui.child(
                    "empty-list",
                    EmptyState::new(if session.danmaku.connection_id.is_some() {
                        "等待新的弹幕…"
                    } else {
                        "点击开始监控，接收实时弹幕。"
                    }),
                )?;
            } else {
                ui.with_child("list", ScrollView::new(ScrollAxes::Vertical).label("实时弹幕"), |ui| {
                    for (index, message) in session.messages.iter().rev().take(80).enumerate() {
                        ui.child(
                            format!("m{index}"),
                            ListItem::new(format!(
                                "{}  {}",
                                message.sender_name, message.text
                            )),
                        )?;
                    }
                    Ok(())
                })?;
            }
            if let Some(error) = &session.danmaku_error {
                ui.child(
                    "error",
                    ValidationMessage::new(error.clone(), ValidationIntent::Danger),
                )?;
            }
            Ok(())
        })?;
        if let Some(entity) = start {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::StartDanmaku)?;
        }
        if let Some(entity) = stop {
            bind_once(cx, &mut self.bound, entity, inbox, AppEvent::StopDanmaku)?;
        }
        Ok(())
    }

    fn sync_stats(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let inbox = session.inbox.clone();
        let mut tabs = None;
        let mut clear = None;
        let mut confirm = None;
        let mut cancel = None;
        cx.mount(self.primary, |ui| {
            if !session.authenticated() {
                ui.child(
                    "empty",
                    EmptyState::new("登录后查看数据").message("在侧栏底部登录 B 站账号。"),
                )?;
                return Ok(());
            }
            if session.room.is_none() {
                ui.child(
                    "empty",
                    EmptyState::new("先连接一个直播间")
                        .message("在右侧连接直播间后查看趋势和历史。"),
                )?;
                return Ok(());
            }
            ui.child("heading", Text::new("数据"))?;
            tabs = Some(
                ui.child(
                    "tabs",
                    Tabs::new(match session.stats_tab {
                        StatsTab::Trend => "trend",
                        StatsTab::History => "history",
                    })
                    .options([
                        TabOption::new("trend", "趋势").draggable(false),
                        TabOption::new("history", "历史").draggable(false),
                    ])
                    .fill(true),
                )?,
            );
            let history = session.current_snapshots();
            match session.stats_tab {
                StatsTab::Trend => {
                    ui.child(
                        "viewers",
                        TimeSeriesChart::new(session.viewer_series()).label("在线人数"),
                    )?;
                    ui.child(
                        "followers",
                        TimeSeriesChart::new(session.follower_series()).label("关注数"),
                    )?;
                    let latest = history.last();
                    ui.child(
                        "now",
                        LabeledValue::new(
                            "当前在线",
                            latest
                                .map(|item| item.viewer_count.to_string())
                                .unwrap_or_else(|| "暂无".to_owned()),
                        ),
                    )?;
                    ui.child(
                        "follow",
                        LabeledValue::new(
                            "关注数",
                            latest
                                .and_then(|item| item.follower_count)
                                .map(|count| count.to_string())
                                .unwrap_or_else(|| "暂无".to_owned()),
                        ),
                    )?;
                    ui.child(
                        "points",
                        LabeledValue::new("采集点", history.len().to_string()),
                    )?;
                }
                StatsTab::History => {
                    if history.is_empty() {
                        ui.child(
                            "empty-history",
                            EmptyState::new("当前直播间暂无历史快照。"),
                        )?;
                    } else {
                        ui.with_child(
                            "history",
                            ScrollView::new(ScrollAxes::Vertical).label("历史快照"),
                            |ui| {
                                for (index, snapshot) in history.iter().rev().take(80).enumerate() {
                                    ui.child(
                                        format!("h{index}"),
                                        ListItem::new(format!(
                                            "{}  {}  {}",
                                            snapshot.viewer_count,
                                            snapshot
                                                .follower_count
                                                .map(|count| count.to_string())
                                                .unwrap_or_else(|| "暂无".to_owned()),
                                            live_label(&snapshot.live_status)
                                        )),
                                    )?;
                                }
                                Ok(())
                            },
                        )?;
                    }
                    if session.confirm_clear {
                        ui.child(
                            "confirm-hint",
                            Text::new("将删除这个直播间保存在本地的脱敏快照。"),
                        )?;
                        confirm = Some(ui.child(
                            "confirm",
                            Button::new("确认清理").kind(ButtonKind::Danger),
                        )?);
                        cancel = Some(ui.child(
                            "cancel",
                            Button::new("取消").kind(ButtonKind::Ghost),
                        )?);
                    } else if !history.is_empty() {
                        clear = Some(ui.child(
                            "clear",
                            Button::new("清理当前记录").kind(ButtonKind::Ghost),
                        )?);
                    }
                }
            }
            Ok(())
        })?;
        if let Some(entity) = tabs {
            if self.bound.insert(entity.stable_id()) {
                let inbox = inbox.clone();
                cx.on(entity, move |_tabs, event: &TabsEvent, cx| {
                    if let TabsEvent::Select(value) = event {
                        inbox.push(AppEvent::SelectStatsTab(if value.as_ref() == "history" {
                            StatsTab::History
                        } else {
                            StatsTab::Trend
                        }));
                        cx.dispatch_program(Wake);
                    }
                })?;
            }
        }
        if let Some(entity) = clear {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::AskClearStats)?;
        }
        if let Some(entity) = confirm {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::ClearStats)?;
        }
        if let Some(entity) = cancel {
            bind_once(cx, &mut self.bound, entity, inbox, AppEvent::CancelClearStats)?;
        }
        Ok(())
    }

    fn sync_settings(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let mut appearance = None;
        let mut about = None;
        cx.mount(self.primary, |ui| {
            appearance = Some(ui.child(
                "appearance",
                AppearanceSection::new(session.theme, session.appearance.clone()),
            )?);
            about = Some(ui.child(
                "about",
                AboutSection::new(
                    AboutMetadata::new("Nana播播工具箱", "0.1.0")
                        .description("B 站直播工具箱：账号登录、房间连接、弹幕助手与数据。"),
                ),
            )?);
            Ok(())
        })?;
        if let Some(section) = appearance {
            cx.assemble_appearance_section(section)?;
            if self.bound.insert(section.stable_id()) {
                let inbox = session.inbox.clone();
                cx.on(section, move |_section, event: &AppearanceEvent, cx| {
                    inbox.push(AppEvent::Appearance(*event));
                    cx.dispatch_program(Wake);
                })?;
            }
        }
        if let Some(about) = about {
            let _ = cx.assemble_about_section(about);
        }
        Ok(())
    }

    fn sync_overlay(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        if !session.login_open {
            cx.update_component(self.shell, |shell, _| {
                shell.overlays.clear();
            })?;
            return Ok(());
        }
        if self.login_dialog.is_none() {
            let inbox = session.inbox.clone();
            let (dialog, body) = cx.build_detached(self.document_id, |ui| {
                let body = ui.leaf(Stack::column(12.0));
                let dialog = ui.leaf(Dialog::new("登录 B 站"));
                ui.nest(dialog, |ui| ui.adopt(body));
                ui.on(dialog, move |_, _: &OverlayClosing, cx| {
                    inbox.push(AppEvent::CloseLogin);
                    cx.dispatch_program(Wake);
                });
                (dialog, body)
            })?;
            self.login_dialog = Some(dialog);
            self.login_body = Some(body);
        }
        let dialog = self.login_dialog.expect("login dialog");
        let body = self.login_body.expect("login body");
        self.mount_login_body(cx, body, session)?;
        cx.update_component(self.shell, |shell, _| {
            shell.overlays = vec![dialog.stable_id()];
        })?;
        Ok(())
    }

    fn mount_login_body(
        &mut self,
        cx: &mut nana_ui::runtime::AppContext,
        body: Entity<Stack>,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let inbox = session.inbox.clone();
        let mut check = None;
        let mut refresh = None;
        let mut start = None;
        cx.mount(body, |ui| {
            if session.account_loading && session.qr.is_none() {
                ui.child("hint", Text::new("正在生成二维码…"))?;
            } else if let Some(qr) = &session.qr {
                if let Ok(code) = QrCode::encode(qr.payload.as_bytes(), 224.0) {
                    ui.child("qr", code.label("B站登录二维码"))?;
                }
                ui.child(
                    "hint",
                    Text::new(match session.qr_phase {
                        QrPhase::Scanned => "已扫描，请在手机上确认登录。",
                        QrPhase::Expired => "二维码已过期，请重新生成。",
                        QrPhase::Idle | QrPhase::Pending => "请使用B站手机客户端扫描二维码。",
                    }),
                )?;
                check = Some(ui.child(
                    "check",
                    Button::new("检查登录状态").loading(session.qr_polling),
                )?);
                refresh = Some(ui.child(
                    "refresh",
                    Button::new("刷新二维码").kind(ButtonKind::Ghost),
                )?);
            } else {
                start = Some(ui.child(
                    "start",
                    Button::new("生成二维码")
                        .kind(ButtonKind::Primary)
                        .loading(session.account_loading),
                )?);
            }
            if let Some(error) = &session.account_error {
                ui.child(
                    "error",
                    ValidationMessage::new(error.clone(), ValidationIntent::Danger),
                )?;
            }
            Ok(())
        })?;
        if let Some(entity) = check {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::PollQr)?;
        }
        if let Some(entity) = refresh {
            bind_once(cx, &mut self.bound, entity, inbox.clone(), AppEvent::StartQr)?;
        }
        if let Some(entity) = start {
            bind_once(cx, &mut self.bound, entity, inbox, AppEvent::StartQr)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn collect_labels(&self, document: &RuntimeDocument) -> Vec<String> {
        let world = document.context().world();
        let mut labels = Vec::new();
        collect_node_labels(world, self.shell.stable_id(), &mut labels);
        labels
    }
}

#[cfg(test)]
fn collect_node_labels(
    world: &nana_ui::runtime::UiWorld,
    id: nana_ui::runtime::StableNodeId,
    labels: &mut Vec<String>,
) {
    if let Some(text) = world.text(id) {
        let text = text.trim();
        if !text.is_empty() {
            labels.push(text.to_owned());
        }
    }
    if let Some(accessibility) = world.accessibility(id) {
        if let Some(label) = &accessibility.label {
            let label = label.trim();
            if !label.is_empty() && labels.last().map(String::as_str) != Some(label) {
                labels.push(label.to_owned());
            }
        }
    }
    if let Some(node) = world.node(id) {
        for child in node.children {
            collect_node_labels(world, child, labels);
        }
    }
}

fn image_resource<'a>(session: &Session, slot: &'a str) -> &'a str {
    if session.has_image(slot) { slot } else { "" }
}

fn nav_row(label: &str, active: bool) -> SidebarRow {
    SidebarRow::new(label).state(if active {
        SidebarRowState::Active
    } else {
        SidebarRowState::Idle
    })
}

fn bind_nav(
    ui: &mut nana_ui::runtime::UiBuilder<'_>,
    row: Entity<SidebarRow>,
    inbox: Inbox,
    page: Page,
) {
    ui.on(row, move |_, _: &Activate, cx| {
        inbox.push(AppEvent::Navigate(page));
        cx.dispatch_program(Wake);
    });
}

fn bind_once<V: nana_ui::runtime::View>(
    cx: &mut nana_ui::runtime::AppContext,
    bound: &mut HashSet<nana_ui::runtime::StableNodeId>,
    entity: Entity<V>,
    inbox: Inbox,
    event: AppEvent,
) -> Result<(), FrameworkError> {
    if !bound.insert(entity.stable_id()) {
        return Ok(());
    }
    cx.on(entity, move |_, _: &Activate, cx| {
        inbox.push(event.clone());
        cx.dispatch_program(Wake);
    })
}

#[cfg(test)]
mod tests {
    use nana_ui::runtime::{DocumentId, RuntimeDocument};

    use super::Shell;
    use crate::session::{AppEvent, Page, Session};

    fn labels_for(session: &Session) -> Vec<String> {
        let mut document = RuntimeDocument::new(DocumentId::new(1).expect("document"));
        let shell = Shell::mount(&mut document, session).expect("mount shell");
        shell.collect_labels(&document)
    }

    #[test]
    fn unauthenticated_shell_has_real_navigation_and_login() {
        let session = Session::for_test();
        let labels = labels_for(&session);
        for expected in ["首页", "主播助手", "数据", "设置", "登录"] {
            assert!(
                labels.iter().any(|label| label.contains(expected)),
                "缺少「{expected}」: {labels:?}"
            );
        }
        assert!(
            labels.iter().all(|label| !label.contains("历史记录")),
            "已合并的历史入口不应出现: {labels:?}"
        );
    }

    #[test]
    fn stats_page_shows_login_empty_state() {
        let mut session = Session::for_test();
        session.apply(AppEvent::Navigate(Page::Stats));
        let labels = labels_for(&session);
        assert!(
            labels.iter().any(|label| label.contains("登录后查看数据")),
            "数据页未显示未登录空态: {labels:?}"
        );
    }

    #[test]
    fn login_dialog_reuses_shell_and_shows_qr_action() {
        let mut session = Session::for_test();
        session.apply(AppEvent::OpenLogin);
        let mut document = RuntimeDocument::new(DocumentId::new(1).expect("document"));
        let mut shell = Shell::mount(&mut document, &session).expect("mount");
        let first = shell
            .login_dialog
            .as_ref()
            .map(|dialog| dialog.stable_id());
        shell.sync(&mut document, &session).expect("sync");
        assert_eq!(
            shell.login_dialog.as_ref().map(|dialog| dialog.stable_id()),
            first
        );
        let labels = shell.collect_labels(&document);
        assert!(
            labels.iter().any(|label| label.contains("登录")),
            "登录框未渲染: {labels:?}"
        );
    }
}
