#[cfg(test)]
mod acceptance;
mod data;
mod desktop;
mod feed;
#[cfg(test)]
mod input_tests;
mod palette;
mod theme;
pub use desktop::DesktopDanmakuView;

use crate::images::{ACCOUNT_AVATAR, ROOM_AVATAR};
use crate::session::{
    danmaku_label, live_label, timestamp, AppEvent, DesktopDanmakuPhase, Inbox, Page, QrPhase,
    Revisions, Session,
};
use data::{SettingsModel, StatsModel};
use nana_ui::runtime::view::{
    self, column, dynamic, entity_ref, row, signal, text, widget, with_refs, AnyView, El,
    EntityRef, IntoView, Signal,
};
use nana_ui::runtime::*;
use nana_ui::{ButtonKind, PopoverPlacement};
use nanabobo_core::models::{AccountSummary, DanmakuStatus, RoomInfo, VipKind};
use std::sync::Arc;

pub struct Shell {
    shell: Entity<DesktopShell>,
    login: Entity<Dialog>,
    clear: Entity<Dialog>,
    stats_tabs: Option<Entity<Tabs>>,
    tabs_ref: EntityRef<Tabs>,
    page: Signal<Page>,
    footer: Signal<FooterModel>,
    banner: Signal<BannerModel>,
    room: Signal<RoomModel>,
    metrics: Signal<Option<MetricsModel>>,
    launcher: Signal<LauncherModel>,
    stats: Signal<StatsModel>,
    settings: Signal<SettingsModel>,
    login_body: Signal<LoginModel>,
    clear_body: Signal<ClearModel>,
    last: Option<(Page, Revisions)>,
    connection: Option<DanmakuStatus>,
}

#[derive(Clone, PartialEq)]
struct FooterModel {
    login_loading: bool,
    account: Option<AccountSummary>,
    avatar: bool,
}

#[derive(Clone, PartialEq)]
struct BannerModel {
    storage_error: Option<String>,
    account_error: Option<String>,
}

#[derive(Clone, PartialEq)]
struct RoomModel {
    authenticated: bool,
    auth_loading: bool,
    info: Option<RoomInfo>,
    editing: bool,
    input: String,
    loading: bool,
    error: Option<String>,
    avatar: bool,
}

#[derive(Clone, PartialEq)]
struct ChartModel {
    samples: Vec<(i64, Option<f64>)>,
    start: String,
    end: String,
}

#[derive(Clone, PartialEq)]
struct MetricsModel {
    room_id: u64,
    viewers: String,
    followers: String,
    chart: Option<ChartModel>,
}

#[derive(Clone, PartialEq)]
struct LauncherModel {
    shown: bool,
    phase: DesktopDanmakuPhase,
    status: String,
    error: Option<String>,
    following: bool,
    unread: usize,
}

#[derive(Clone, PartialEq)]
struct LoginModel {
    loading: bool,
    payload: Option<String>,
    phase: QrPhase,
    error: Option<String>,
}

#[derive(Clone, PartialEq)]
struct ClearModel {
    label: String,
}

struct Signals {
    tabs_ref: EntityRef<Tabs>,
    page: Signal<Page>,
    footer: Signal<FooterModel>,
    banner: Signal<BannerModel>,
    room: Signal<RoomModel>,
    metrics: Signal<Option<MetricsModel>>,
    launcher: Signal<LauncherModel>,
    stats: Signal<StatsModel>,
    settings: Signal<SettingsModel>,
    login_body: Signal<LoginModel>,
    clear_body: Signal<ClearModel>,
}

impl Shell {
    pub fn mount(
        document: &mut RuntimeDocument,
        session: &Session,
    ) -> Result<Self, FrameworkError> {
        let document_id = document.document();
        let cx = document.context_mut();
        theme::install(cx, session.theme, session.appearance)?;
        let mut signals = None;
        let (_mounted, (shell, login, clear)) = cx.mount_view_root(document_id, || {
            let page = signal(session.page);
            let footer = signal(footer_model(session));
            let banner = signal(banner_model(session));
            let room = signal(room_model(session));
            let metrics = signal(metrics_model(session));
            let launcher = signal(launcher_model(session));
            let stats = signal(data::stats_model(session));
            let settings = signal(data::settings_model(session));
            let login_body = signal(login_model(session));
            let clear_body = signal(clear_model(session));
            let tabs_ref = entity_ref::<Tabs>();
            let shell_ref = entity_ref::<DesktopShell>();
            let login_ref = entity_ref::<Dialog>();
            let clear_ref = entity_ref::<Dialog>();
            let inbox = session.inbox.clone();
            let view = (
                widget(DesktopShell::new().title("Nana播播工具箱"))
                    .key("shell")
                    .entity_ref(shell_ref)
                    .navigation(navigation(page, footer, inbox.clone()))
                    .primary(primary(
                        page, banner, room, metrics, launcher, stats, settings, tabs_ref, inbox,
                    )),
                view::detached(login_dialog(login_body, session.inbox.clone(), login_ref)),
                view::detached(clear_dialog(clear_body, session.inbox.clone(), clear_ref)),
            );
            signals = Some(Signals {
                tabs_ref,
                page,
                footer,
                banner,
                room,
                metrics,
                launcher,
                stats,
                settings,
                login_body,
                clear_body,
            });
            with_refs(view, (shell_ref, login_ref, clear_ref))
        })?;
        let signals = signals.expect("mounted signals");
        let mut shell = Self {
            shell,
            login,
            clear,
            stats_tabs: None,
            tabs_ref: signals.tabs_ref,
            page: signals.page,
            footer: signals.footer,
            banner: signals.banner,
            room: signals.room,
            metrics: signals.metrics,
            launcher: signals.launcher,
            stats: signals.stats,
            settings: signals.settings,
            login_body: signals.login_body,
            clear_body: signals.clear_body,
            last: None,
            connection: None,
        };
        shell.sync(document, session)?;
        Ok(shell)
    }

    pub fn sync(
        &mut self,
        document: &mut RuntimeDocument,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let previous = self.last;
        let page_changed = previous.is_none_or(|(page, _)| page != session.page);
        let shell_changed =
            previous.is_none_or(|(_, revisions)| revisions.shell != session.revisions.shell);
        let room_changed = page_changed
            || shell_changed
            || previous.is_none_or(|(_, revisions)| revisions.room != session.revisions.room);
        let desktop_changed = room_changed
            || previous.is_none_or(|(_, revisions)| revisions.desktop != session.revisions.desktop);
        let stats_changed = page_changed
            || shell_changed
            || previous.is_none_or(|(_, revisions)| revisions.stats != session.revisions.stats);
        let overlay_changed = page_changed
            || shell_changed
            || previous.is_none_or(|(_, revisions)| revisions.overlay != session.revisions.overlay);
        let settings_changed = page_changed
            || shell_changed
            || previous.is_none_or(|(_, revisions)| revisions.desktop != session.revisions.desktop);
        let cx = document.context_mut();
        if shell_changed || page_changed {
            theme::install(cx, session.theme, session.appearance)?;
            publish(&self.page, session.page);
            publish(&self.footer, footer_model(session));
            publish(&self.banner, banner_model(session));
        }
        if settings_changed {
            publish(&self.settings, data::settings_model(session));
        }
        if session.page == Page::Workbench {
            if room_changed {
                publish(&self.room, room_model(session));
            }
            if desktop_changed
                || stats_changed
                || self.connection.as_ref() != Some(&session.danmaku.status)
            {
                if room_changed || stats_changed {
                    publish(&self.metrics, metrics_model(session));
                }
                publish(&self.launcher, launcher_model(session));
            }
        }
        if session.page == Page::Stats && stats_changed {
            publish(&self.stats, data::stats_model(session));
        }
        if overlay_changed {
            publish(&self.login_body, login_model(session));
            publish(&self.clear_body, clear_model(session));
        }
        cx.flush_reactive()?;
        let stats_ready = session.page == Page::Stats
            && self.stats.with_untracked(|model| !model.rooms.is_empty());
        self.stats_tabs = stats_ready
            .then(|| self.tabs_ref.get())
            .flatten()
            .filter(|tabs| cx.world().contains(tabs.stable_id()));
        if overlay_changed {
            self.sync_overlays(cx, session)?;
        }
        self.connection = Some(session.danmaku.status.clone());
        self.last = Some((session.page, session.revisions));
        Ok(())
    }

    fn sync_overlays(
        &mut self,
        cx: &mut AppContext,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let mut overlays = Vec::new();
        if session.auth.open {
            overlays.push(self.login.stable_id());
        }
        if session.stats.confirm_clear.is_some() {
            overlays.push(self.clear.stable_id());
        }
        let active = overlays.last().copied();
        if active.is_none() {
            if let Some(host) = cx.read(self.shell, |shell| shell.overlay)? {
                let host = Entity::<OverlayHost>::from_stable_id(host);
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
            let event = if self.login.stable_id() == active {
                AppEvent::CloseLogin
            } else {
                AppEvent::CancelClearStats
            };
            let inbox = session.inbox.clone();
            cx.on_keyed(host, "close", move |_, closing: &OverlayClosing, _| {
                if closing.root == active {
                    inbox.push(event.clone());
                }
            })?;
            cx.activate_overlay(host, Entity::<Dialog>::from_stable_id(active))?;
        }
        Ok(())
    }
}

fn navigation(page: Signal<Page>, footer: Signal<FooterModel>, inbox: Inbox) -> impl IntoView {
    widget(SidebarFrame::new())
        .body((
            nav_row("概览", page, Page::Workbench, inbox.clone()),
            nav_row("数据", page, Page::Stats, inbox.clone()),
        ))
        .footer(dynamic(footer, {
            let inbox = inbox.clone();
            move |model| footer_bar(model, page, inbox.clone())
        }))
}

fn nav_row(label: &str, page: Signal<Page>, target: Page, inbox: Inbox) -> El<SidebarRow> {
    widget(SidebarRow::new(label))
        .bind(move |row| {
            row.state = if page.get() == target {
                SidebarRowState::Active
            } else {
                SidebarRowState::Idle
            };
        })
        .on(move |_: &Activate| inbox.push(AppEvent::Navigate(target)))
}

fn footer_bar(model: &FooterModel, page: Signal<Page>, inbox: Inbox) -> AnyView {
    widget(SidebarFooter::new())
        .with(|c| {
            if let Some(account) = &model.account {
                c.add(account_card(account, model.avatar, inbox.clone()));
            } else {
                c.add(
                    widget(Button::new("登录 B 站"))
                        .loading(model.login_loading)
                        .on_activate({
                            let inbox = inbox.clone();
                            move || inbox.push(AppEvent::OpenLogin)
                        }),
                );
            }
            c.add(nav_row("设置", page, Page::Settings, inbox));
        })
        .into_any()
}

fn account_card(account: &AccountSummary, avatar: bool, inbox: Inbox) -> AnyView {
    let name = account.username.clone();
    let resource = if avatar { ACCOUNT_AVATAR } else { "" };
    let mut stats = vec![
        ("等级", account.level.map(|level| format!("LV{level}"))),
        ("硬币", account.coins.map(amount_text)),
        ("B币", account.bcoin.map(amount_text)),
    ];
    if let Some(vip) = account.vip {
        stats.push(("会员", Some(vip_text(vip).into())));
    }
    let uid = account.mid;
    widget(
        HoverCard::new()
            .trigger_image(resource, name.clone())
            .trigger_size(28.0)
            .width(264.0)
            .placement(PopoverPlacement::Right),
    )
    .with(move |c| {
        c.add(widget(Stack::column(10.0).padding(14.0)).with(|c| {
            c.add(row().gap(10.0).with(|c| {
                c.add(view::avatar(48.0).resource(resource).label(name.clone()));
                c.add(column().gap(2.0).with(|c| {
                    c.add(rich(name.clone(), |text| {
                        let layout = Arc::make_mut(&mut text.style.layout);
                        layout.font_size = Some(15.0);
                        layout.font_weight = Some(600);
                        layout.max_width = Some(LengthSpec::Px(178.0));
                        layout.white_space_nowrap = true;
                        layout.text_overflow_ellipsis = true;
                    }));
                    c.add(widget(muted(format!("UID {uid}"))));
                }));
            }));
            c.add(row().gap(20.0).with(|c| {
                for (label, value) in stats {
                    c.add(column().gap(2.0).with(|c| {
                        c.add(rich(value.unwrap_or_else(|| "—".into()), |text| {
                            let layout = Arc::make_mut(&mut text.style.layout);
                            layout.font_size = Some(14.0);
                            layout.font_weight = Some(600);
                        }));
                        c.add(widget(muted(label)));
                    }));
                }
            }));
            c.add(
                widget(Button::new("退出登录").kind(ButtonKind::Ghost))
                    .on_activate(move || inbox.push(AppEvent::Logout)),
            );
        }));
    })
    .into_any()
}

fn primary(
    page: Signal<Page>,
    banner: Signal<BannerModel>,
    room: Signal<RoomModel>,
    metrics: Signal<Option<MetricsModel>>,
    launcher: Signal<LauncherModel>,
    stats: Signal<StatsModel>,
    settings: Signal<SettingsModel>,
    tabs: EntityRef<Tabs>,
    inbox: Inbox,
) -> impl IntoView {
    widget(
        Stack::fill_column(12.0)
            .padding(20.0)
            .surface(SemanticColorRole::Background),
    )
    .with(|c| {
        c.add(dynamic(banner, {
            let inbox = inbox.clone();
            move |model| banner_view(model, inbox.clone())
        }));
        c.add(widget(Stack::fill_column(16.0)).with(|c| {
            c.add(workbench(page, room, metrics, launcher, inbox.clone()));
            c.add(data::stats_page(page, stats, tabs, inbox.clone()));
            c.add(data::settings_page(page, settings, inbox));
        }));
    })
}

fn workbench(
    page: Signal<Page>,
    room: Signal<RoomModel>,
    metrics: Signal<Option<MetricsModel>>,
    launcher: Signal<LauncherModel>,
    inbox: Inbox,
) -> AnyView {
    widget(page_scroll("概览"))
        .visible(move || page.get() == Page::Workbench)
        .with(|c| {
            c.add(widget(heading("概览")));
            c.add(widget(columns()).with(|c| {
                c.add(widget(lane()).with(|c| {
                    c.add(dynamic(room, {
                        let inbox = inbox.clone();
                        move |model| room_view(model, inbox.clone())
                    }));
                    c.add(dynamic(metrics, {
                        let inbox = inbox.clone();
                        move |model| match model {
                            Some(model) => metrics_view(model, inbox.clone()),
                            None => ().into_any(),
                        }
                    }));
                }));
                c.add(widget(lane()).with(|c| {
                    c.add(dynamic(launcher, {
                        let inbox = inbox.clone();
                        move |model| launcher_view(model, inbox.clone())
                    }));
                    c.add(widget(panel(SemanticColorRole::Surface)).with(|c| {
                        c.add(widget(heading("快捷操作")));
                        c.add(press(
                            "查看数据",
                            inbox.clone(),
                            AppEvent::Navigate(Page::Stats),
                        ));
                        c.add(press(
                            "打开设置",
                            inbox.clone(),
                            AppEvent::Navigate(Page::Settings),
                        ));
                    }));
                }));
            }));
        })
        .into_any()
}

pub(super) fn columns() -> Stack {
    Stack::bar(16.0)
        .wrap(true)
        .align(AlignSpec::Start)
        .min_width(LengthSpec::Px(0.0))
        .height(LengthSpec::Shrink)
}

pub(super) fn lane() -> Stack {
    Stack::column(16.0).with_layout(|layout| {
        layout.width = Some(LengthSpec::Px(300.0));
        layout.min_width = Some(LengthSpec::Px(0.0));
        layout.flex_basis = Some(LengthSpec::Px(300.0));
        layout.flex_grow = Some(1.0);
        layout.flex_shrink = Some(1.0);
    })
}

pub(super) fn panel(role: SemanticColorRole) -> Stack {
    Stack::column(12.0)
        .padding(16.0)
        .surface(role)
        .outline(SemanticColorRole::BorderSoft, 1.0)
        .radius_px(16.0)
}

fn page_scroll(label: &str) -> ScrollView {
    let mut scroll = ScrollView::new(ScrollAxes::Vertical).label(label.to_owned());
    let l = Arc::make_mut(&mut scroll.style.layout);
    l.width = Some(LengthSpec::Fill);
    l.height = Some(LengthSpec::Fill);
    l.min_height = Some(LengthSpec::Px(0.0));
    l.flex_grow = Some(1.0);
    l.gap = Some(LengthSpec::Px(16.0));
    scroll
}

fn banner_view(model: &BannerModel, inbox: Inbox) -> AnyView {
    if model.storage_error.is_none() && model.account_error.is_none() {
        return ().into_any();
    }
    column()
        .gap(6.0)
        .with(|c| {
            if let Some(error) = &model.storage_error {
                c.add(widget(ValidationMessage::new(
                    error.clone(),
                    ValidationIntent::Warning,
                )));
                c.add(press("重试保存", inbox.clone(), AppEvent::RetryStore));
            }
            if let Some(error) = &model.account_error {
                c.add(widget(ValidationMessage::new(
                    error.clone(),
                    ValidationIntent::Danger,
                )));
            }
        })
        .into_any()
}

fn room_view(model: &RoomModel, inbox: Inbox) -> AnyView {
    let authenticated = model.authenticated;
    let loading = model.auth_loading;
    let editing = model.editing;
    let info = model.info.is_some();
    widget(panel(SemanticColorRole::Selected))
        .with(|c| {
            c.add(row().gap(10.0).with(|c| {
                c.add(widget(heading("直播控制台")));
                c.add(widget(muted(if authenticated && info {
                    "实时状态"
                } else {
                    "等待连接"
                })));
            }));
            if !authenticated {
                c.add(widget(Stack::column(8.0).padding(12.0)).with(|c| {
                    c.add(widget(heading("连接 B 站账号")));
                    c.add(text("登录后可以查看直播间数据并启动桌面弹幕。"));
                    c.add(press_kind(
                        "登录 B 站",
                        ButtonKind::Primary,
                        loading,
                        inbox.clone(),
                        AppEvent::OpenLogin,
                    ));
                }));
                return;
            }
            if let Some(room) = &model.info {
                c.add(room_context(room, model.avatar, inbox.clone()));
            }
            if model.info.is_none() || editing {
                c.add(widget(Stack::column(8.0).padding(12.0)).with(|c| {
                    c.add(widget(muted(if editing {
                        "切换当前直播间"
                    } else {
                        "连接一个直播间"
                    })));
                    c.add(room_form(model, inbox.clone()));
                }));
            }
            if let Some(error) = &model.error {
                c.add(widget(ValidationMessage::new(
                    error.clone(),
                    ValidationIntent::Danger,
                )));
                if info && !editing {
                    c.add(press("重试刷新", inbox, AppEvent::RefreshRoom));
                }
            }
        })
        .into_any()
}

fn room_context(room: &RoomInfo, avatar: bool, inbox: Inbox) -> AnyView {
    let title = room.title.clone();
    let owner = room.owner_name.clone().unwrap_or_else(|| "主播".into());
    let details = format!(
        "{} · 房间 {} · {}",
        owner,
        room.room_id,
        live_label(&room.live_status)
    );
    let updated = format!("更新于 {}", timestamp(room.fetched_at));
    widget(Stack::column(10.0))
        .with(|c| {
            c.add(widget(Stack::row(10.0).width(LengthSpec::Fill)).with(|c| {
                c.add(
                    view::avatar(40.0)
                        .resource(if avatar { ROOM_AVATAR } else { "" })
                        .label(owner),
                );
                c.add(rich(title, |text| {
                    let layout = Arc::make_mut(&mut text.style.layout);
                    layout.width = Some(LengthSpec::Px(100.0));
                    layout.flex_grow = Some(1.0);
                    layout.flex_shrink = Some(1.0);
                    layout.min_width = Some(LengthSpec::Px(0.0));
                    layout.word_break = Some(WordBreakSpec::BreakWord);
                }));
            }));
            c.add(widget(muted(details)));
            c.add(widget(Stack::bar(8.0).wrap(true)).with(|c| {
                c.add(press("切换直播间", inbox.clone(), AppEvent::EditRoom));
                c.add(press("断开直播间", inbox, AppEvent::DisconnectRoom));
                c.add(widget(muted(updated)));
            }));
        })
        .into_any()
}

fn room_form(model: &RoomModel, inbox: Inbox) -> AnyView {
    let mut input = TextInput::new(&model.input)
        .label("直播间号")
        .placeholder("输入直播间号");
    input.invalid = model.error.is_some();
    input.disabled = model.loading;
    let mut style = input.style.clone();
    let layout = Arc::make_mut(&mut style.layout);
    layout.width = Some(LengthSpec::Px(240.0));
    layout.min_width = Some(LengthSpec::Px(100.0));
    layout.flex_grow = Some(1.0);
    layout.flex_shrink = Some(1.0);
    input = input.style(style);
    let show_cancel = model.info.is_some();
    let loading = model.loading;
    widget(Stack::bar(8.0).wrap(true))
        .with(move |c| {
            c.add(
                widget(input)
                    .on_input({
                        let inbox = inbox.clone();
                        move |event: &TextChanged| {
                            inbox.push(AppEvent::RoomIdChanged(event.value.to_string()));
                        }
                    })
                    .on_submit({
                        let inbox = inbox.clone();
                        move |event: &TextSubmitted| {
                            inbox.push(AppEvent::RoomIdChanged(event.value.to_string()));
                            inbox.push(AppEvent::QueryRoom);
                        }
                    }),
            );
            c.add(press_kind(
                "连接直播间",
                ButtonKind::Primary,
                loading,
                inbox.clone(),
                AppEvent::QueryRoom,
            ));
            if show_cancel {
                c.add(press("取消", inbox, AppEvent::CancelEditRoom));
            }
        })
        .into_any()
}

fn metrics_view(model: &MetricsModel, inbox: Inbox) -> AnyView {
    let room_id = model.room_id;
    widget(panel(SemanticColorRole::Surface))
        .with(|c| {
            c.add(widget(muted("当前直播数据")));
            c.add(widget(Stack::bar(16.0).wrap(true)).with(|c| {
                c.add(
                    widget(Stack::column(2.0).width(LengthSpec::Px(120.0))).with(|c| {
                        c.add(widget(heading(model.viewers.clone())));
                        c.add(widget(muted("在线人数")));
                    }),
                );
                c.add(
                    widget(Stack::column(2.0).width(LengthSpec::Px(120.0))).with(|c| {
                        c.add(widget(heading(model.followers.clone())));
                        c.add(widget(muted("粉丝数")));
                    }),
                );
            }));
            if let Some(chart) = &model.chart {
                let mut series = TimeSeriesChart::from_samples(chart.samples.clone())
                    .label("最近人气趋势")
                    .unit("人气")
                    .time_labels(chart.start.clone(), chart.end.clone());
                Arc::make_mut(&mut series.style.layout).height = Some(LengthSpec::Px(140.0));
                c.add(widget(series));
            }
            c.add(widget(Button::new("查看数据")).on_activate(move || {
                inbox.push(AppEvent::SelectHistoryRoom(room_id));
                inbox.push(AppEvent::Navigate(Page::Stats));
            }));
        })
        .into_any()
}

fn launcher_view(model: &LauncherModel, inbox: Inbox) -> AnyView {
    if !model.shown {
        return ().into_any();
    }
    let phase = model.phase;
    widget(panel(SemanticColorRole::Surface))
        .with(|c| {
            c.add(row().gap(8.0).with(|c| {
                c.add(widget(heading("桌面弹幕")));
                c.add(widget(muted(model.status.clone())));
            }));
            c.add(widget(Stack::bar(8.0).wrap(true)).with(|c| {
                if matches!(
                    phase,
                    DesktopDanmakuPhase::Closed | DesktopDanmakuPhase::Creating
                ) {
                    c.add(press_kind(
                        "启动桌面弹幕",
                        ButtonKind::Primary,
                        phase == DesktopDanmakuPhase::Creating,
                        inbox.clone(),
                        AppEvent::OpenDesktopDanmaku,
                    ));
                } else {
                    let label = if phase == DesktopDanmakuPhase::Locked {
                        "解锁并调整"
                    } else {
                        "调整弹幕"
                    };
                    c.add(press(label, inbox.clone(), AppEvent::AdjustDesktopDanmaku));
                    c.add(press(
                        "关闭桌面弹幕",
                        inbox.clone(),
                        AppEvent::CloseDesktopDanmaku,
                    ));
                }
            }));
            if matches!(
                phase,
                DesktopDanmakuPhase::Adjusting | DesktopDanmakuPhase::Locked
            ) {
                c.add(widget(muted(if model.following {
                    "跟随最新消息".into()
                } else {
                    format!("正在阅读 · {} 条新消息", model.unread)
                })));
                if !model.following {
                    c.add(press("回到最新", inbox.clone(), AppEvent::FollowLatest));
                }
            }
            if let Some(error) = &model.error {
                c.add(widget(ValidationMessage::new(
                    error.clone(),
                    ValidationIntent::Warning,
                )));
            }
        })
        .into_any()
}

fn login_dialog(
    model: Signal<LoginModel>,
    inbox: Inbox,
    dialog_ref: EntityRef<Dialog>,
) -> impl IntoView {
    widget(Dialog::new("登录 B 站"))
        .entity_ref(dialog_ref)
        .body(dynamic(model, move |model| {
            login_body(model, inbox.clone())
        }))
}

fn login_body(model: &LoginModel, inbox: Inbox) -> AnyView {
    widget(Stack::column(12.0).max_width(440.0))
        .with(|c| {
            if model.payload.is_none() && model.loading {
                c.add(text("正在生成二维码…"));
            } else {
                if let Some(payload) = &model.payload {
                    if matches!(model.phase, QrPhase::Pending | QrPhase::Scanned) {
                        match QrCode::encode(payload.as_bytes(), 224.0) {
                            Ok(code) => {
                                c.add(widget(code.label("B站登录二维码")));
                            }
                            Err(_) => {
                                c.add(text("二维码无法显示，请重新生成。"));
                            }
                        }
                    }
                }
                c.add(text(match model.phase {
                    QrPhase::Scanned => "已扫描，请在手机上确认登录。",
                    QrPhase::Expired => "二维码已过期，请重新生成。",
                    QrPhase::Failed => "登录暂时未完成，请重新尝试。",
                    _ => "请使用 B 站手机客户端扫描二维码。",
                }));
                c.add(press("重新生成二维码", inbox.clone(), AppEvent::StartQr));
            }
            if let Some(error) = &model.error {
                c.add(widget(ValidationMessage::new(
                    error.clone(),
                    ValidationIntent::Danger,
                )));
            }
            c.add(press("取消登录", inbox, AppEvent::CloseLogin));
        })
        .into_any()
}

fn clear_dialog(
    model: Signal<ClearModel>,
    inbox: Inbox,
    dialog_ref: EntityRef<Dialog>,
) -> impl IntoView {
    widget(Dialog::new("清理历史记录"))
        .entity_ref(dialog_ref)
        .body(dynamic(model, move |model| {
            clear_body(model, inbox.clone())
        }))
}

fn clear_body(model: &ClearModel, inbox: Inbox) -> AnyView {
    let label = model.label.clone();
    widget(Stack::column(12.0).max_width(440.0))
        .with(move |c| {
            c.add(text(format!(
                "删除「{label}」的全部本地历史记录？此操作无法撤销。"
            )));
            c.add(row().gap(8.0).with(move |c| {
                c.add(press("取消", inbox.clone(), AppEvent::CancelClearStats));
                c.add(press_kind(
                    "删除记录",
                    ButtonKind::Danger,
                    false,
                    inbox,
                    AppEvent::ClearStats,
                ));
            }));
        })
        .into_any()
}

fn footer_model(session: &Session) -> FooterModel {
    FooterModel {
        login_loading: session.auth.loading() && !session.auth.open,
        account: session
            .account()
            .filter(|_| session.authenticated())
            .cloned(),
        avatar: session.has_image(ACCOUNT_AVATAR),
    }
}

fn banner_model(session: &Session) -> BannerModel {
    BannerModel {
        storage_error: session.storage_error.clone(),
        account_error: if session.auth.open {
            None
        } else {
            session.auth.error().map(str::to_owned)
        },
    }
}

fn room_model(session: &Session) -> RoomModel {
    RoomModel {
        authenticated: session.authenticated(),
        auth_loading: session.auth.loading(),
        info: session.room.info.clone(),
        editing: session.room.editing,
        input: session.room.input.clone(),
        loading: session.room.loading(),
        error: session.room.error().map(str::to_owned),
        avatar: session.has_image(ROOM_AVATAR),
    }
}

fn metrics_model(session: &Session) -> Option<MetricsModel> {
    let room = session.room.info.as_ref()?;
    let samples: Vec<_> = session
        .stats
        .snapshots
        .iter()
        .filter(|snapshot| snapshot.room_id == room.room_id)
        .rev()
        .take(30)
        .collect();
    let chart = (!samples.is_empty()).then(|| ChartModel {
        samples: samples
            .iter()
            .rev()
            .map(|snapshot| {
                (
                    snapshot.captured_at.saturating_mul(1000) as i64,
                    Some(snapshot.viewer_count as f64),
                )
            })
            .collect(),
        start: timestamp(samples.last().expect("chart").captured_at),
        end: timestamp(samples[0].captured_at),
    });
    Some(MetricsModel {
        room_id: room.room_id,
        viewers: format!("人气  {}", room.viewer_count),
        followers: format!(
            "粉丝  {}",
            room.follower_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "暂无数据".into())
        ),
        chart,
    })
}

fn launcher_model(session: &Session) -> LauncherModel {
    let phase = session.desktop.phase;
    let status = match phase {
        DesktopDanmakuPhase::Closed => "未启动",
        DesktopDanmakuPhase::Creating => "正在打开",
        DesktopDanmakuPhase::Adjusting | DesktopDanmakuPhase::Locked => {
            danmaku_label(&session.danmaku.status)
        }
    };
    LauncherModel {
        shown: session.room.info.is_some(),
        phase,
        following: session.danmaku.following,
        unread: session.danmaku.unread,
        status: status.into(),
        error: session
            .desktop
            .error
            .clone()
            .or_else(|| session.danmaku.status.message.clone()),
    }
}

fn login_model(session: &Session) -> LoginModel {
    LoginModel {
        loading: session.auth.loading(),
        payload: session.auth.qr.as_ref().map(|qr| qr.payload.clone()),
        phase: session.auth.phase,
        error: session.auth.error().map(str::to_owned),
    }
}

fn clear_model(session: &Session) -> ClearModel {
    ClearModel {
        label: session
            .stats
            .confirm_clear
            .map(|id| session.stats.room_label(id))
            .unwrap_or_default(),
    }
}

pub(super) fn publish<T: PartialEq>(signal: &Signal<T>, value: T) {
    if signal.with_untracked(|current| current != &value) {
        signal.set(value);
    }
}

fn press(label: &str, inbox: Inbox, event: AppEvent) -> El<Button> {
    widget(Button::new(label).kind(ButtonKind::Subtle))
        .on_activate(move || inbox.push(event.clone()))
}

fn press_kind(
    label: &str,
    kind: ButtonKind,
    loading: bool,
    inbox: Inbox,
    event: AppEvent,
) -> El<Button> {
    widget(Button::new(label).kind(kind))
        .loading(loading)
        .on_activate(move || inbox.push(event.clone()))
}

pub(super) fn heading(value: impl Into<String>) -> Text {
    rich_text(value, |text| {
        let style = Arc::make_mut(&mut text.style.layout);
        style.font_size = Some(20.0);
        style.font_weight = Some(600);
    })
}

pub(super) fn muted(value: impl Into<String>) -> Text {
    rich_text(value, |text| {
        text.style.foreground = Some(SemanticColorRole::Muted);
    })
}

fn rich(value: impl Into<String>, edit: impl FnOnce(&mut Text)) -> El<Text> {
    widget(rich_text(value, edit))
}

fn rich_text(value: impl Into<String>, edit: impl FnOnce(&mut Text)) -> Text {
    let mut text = Text::new(value);
    edit(&mut text);
    text
}

fn amount_text(value: f64) -> String {
    if (value - value.trunc()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn vip_text(vip: VipKind) -> &'static str {
    match vip {
        VipKind::Monthly => "月度大会员",
        VipKind::Annual => "年度大会员",
    }
}
