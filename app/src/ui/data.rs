use std::sync::Arc;

use nana_ui::runtime::view::El;
use nana_ui::runtime::view::{
    column, dynamic, empty_state, row, select, widget, AnyView, EntityRef, IntoView, Signal,
};
use nana_ui::runtime::{
    AboutMetadata, AboutSection, AppearanceSection, Button, LengthSpec, ScrollAxes, ScrollView,
    SelectChanged, SelectOption, SemanticColorRole, Stack, TabOption, Tabs, TabsEvent, Text,
    TimeSeriesChart,
};
use nana_ui::{AppearanceEvent, AppearanceSettings, ButtonKind, ThemeMode, WindowMaterialMode};

use crate::session::{live_label, timestamp, AppEvent, Inbox, Page, Session, StatsTab, Wake};

use super::{columns, heading, lane, muted, panel};

#[derive(Clone, PartialEq)]
pub(super) struct HistoryRow {
    room_id: u64,
    captured_at: u64,
    time: String,
    viewers: String,
    followers: String,
    status: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct TrendModel {
    start: String,
    end: String,
    count: usize,
    viewers: Vec<(i64, Option<f64>)>,
    followers: Vec<(i64, Option<f64>)>,
}

#[derive(Clone, PartialEq)]
pub(super) struct StatsModel {
    pub(super) rooms: Vec<(u64, String)>,
    selected: Option<u64>,
    tab: StatsTab,
    history: Vec<HistoryRow>,
    trend: Option<TrendModel>,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct SettingsModel {
    theme: ThemeMode,
    appearance: AppearanceSettings,
    font_size: f32,
    background_opacity: f32,
}

pub(super) fn stats_model(session: &Session) -> StatsModel {
    let current = session.stats.current();
    let history = current
        .iter()
        .rev()
        .map(|snapshot| HistoryRow {
            room_id: snapshot.room_id,
            captured_at: snapshot.captured_at,
            time: timestamp(snapshot.captured_at),
            viewers: snapshot.viewer_count.to_string(),
            followers: snapshot
                .follower_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "—".into()),
            status: live_label(&snapshot.live_status).into(),
        })
        .collect();
    let trend = current.first().map(|first| {
        let last = current.last().expect("history");
        TrendModel {
            start: timestamp(first.captured_at),
            end: timestamp(last.captured_at),
            count: current.len(),
            viewers: session.stats.viewer_series(),
            followers: session.stats.follower_series(),
        }
    });
    StatsModel {
        rooms: session.stats.rooms(),
        selected: session.stats.selected_room,
        tab: session.stats.tab,
        history,
        trend,
    }
}

pub(super) fn settings_model(session: &Session) -> SettingsModel {
    SettingsModel {
        theme: session.theme,
        appearance: session.appearance,
        font_size: session.desktop.settings.font_size,
        background_opacity: session.desktop.settings.background_opacity,
    }
}

pub(super) fn stats_page(
    page: Signal<Page>,
    stats: Signal<StatsModel>,
    tabs: EntityRef<Tabs>,
    inbox: Inbox,
) -> AnyView {
    let selected = stats.with_untracked(|model| tab_id(model.tab));
    scroll("直播间数据")
        .visible(move || page.get() == Page::Stats)
        .with(|c| {
            c.add(widget(heading("数据")));
            c.add(widget(muted("每一次直播，都有迹可循。")));
            c.add(widget(columns()).with(|c| {
                c.add(widget(lane()).with(|c| {
                    c.add(widget(panel(SemanticColorRole::Surface)).with(|c| {
                        c.add(
                            empty_state("还没有采集记录")
                                .message("连接直播间后，会自动保存在线人数与关注数。")
                                .visible(move || stats.with(|model| model.rooms.is_empty())),
                        );
                        c.add(
                            widget(Tabs::new(selected).options([
                                TabOption::new("trend", "趋势").draggable(false),
                                TabOption::new("history", "历史").draggable(false),
                            ]))
                            .visible(move || stats.with(|model| !model.rooms.is_empty()))
                            .entity_ref(tabs)
                            .bind(move |tabs| {
                                let next: Arc<str> = stats.with(|model| tab_id(model.tab).into());
                                if tabs.selected.as_deref() != Some(next.as_ref()) {
                                    tabs.selected = Some(next);
                                }
                            })
                            .on({
                                let inbox = inbox.clone();
                                move |event: &TabsEvent| {
                                    if let TabsEvent::Select(value) = event {
                                        let tab = if value.as_ref() == "history" {
                                            StatsTab::History
                                        } else {
                                            StatsTab::Trend
                                        };
                                        inbox.push(AppEvent::SelectStatsTab(tab));
                                    }
                                }
                            }),
                        );
                        c.add(empty_state("这个直播间还没有记录").visible(move || {
                            stats.with(|model| !model.rooms.is_empty() && model.history.is_empty())
                        }));
                        c.add(dynamic(stats, trend_body).visible(move || {
                            stats.with(|model| {
                                !model.rooms.is_empty()
                                    && model.trend.is_some()
                                    && model.tab == StatsTab::Trend
                            })
                        }));
                        c.add(history_count(stats).visible(move || show_history(&stats)));
                        c.add(history_columns().visible(move || show_history(&stats)));
                        c.add(dynamic(stats, history_rows).visible(move || show_history(&stats)));
                    }));
                }));
                c.add(widget(lane()).with(|c| {
                    c.add(widget(panel(SemanticColorRole::Selected)).with(|c| {
                        c.add(widget(heading("直播间")));
                        c.add(widget(muted("切换直播间，查看对应的趋势与历史。")));
                        c.add(
                            room_toolbar(stats, inbox.clone())
                                .visible(move || stats.with(|model| !model.rooms.is_empty())),
                        );
                        c.add(
                            widget(muted("连接直播间后，这里会显示采集过的房间。"))
                                .visible(move || stats.with(|model| model.rooms.is_empty())),
                        );
                    }));
                    c.add(widget(panel(SemanticColorRole::Surface)).with(|c| {
                        c.add(widget(heading("采样摘要")));
                        c.add(dynamic(stats, |model| match &model.trend {
                            Some(trend) => column()
                                .gap(8.0)
                                .with(|c| {
                                    c.add(widget(Text::new(format!("{} 条记录", trend.count))));
                                    c.add(widget(muted(format!("开始 {}", trend.start))));
                                    c.add(widget(muted(format!("最近 {}", trend.end))));
                                })
                                .into_any(),
                            None => widget(muted("尚无采样记录")).into_any(),
                        }));
                    }));
                }));
            }));
        })
        .into_any()
}

fn show_history(stats: &Signal<StatsModel>) -> bool {
    stats.with(|model| {
        !model.rooms.is_empty() && model.trend.is_some() && model.tab == StatsTab::History
    })
}

fn tab_id(tab: StatsTab) -> &'static str {
    match tab {
        StatsTab::History => "history",
        StatsTab::Trend => "trend",
    }
}

fn room_toolbar(stats: Signal<StatsModel>, inbox: Inbox) -> El<Stack, Vec<AnyView>> {
    widget(Stack::column(12.0)).with(move |c| {
        c.add(
            select()
                .value(move || {
                    stats.with(|model| model.selected.map(|id| Arc::<str>::from(id.to_string())))
                })
                .options(move || {
                    stats.with(|model| {
                        model
                            .rooms
                            .iter()
                            .map(|(id, name)| SelectOption::new(id.to_string(), name.clone()))
                            .collect()
                    })
                })
                .placeholder("选择直播间")
                .on_change({
                    let inbox = inbox.clone();
                    move |event: &SelectChanged| {
                        if let Ok(id) = event.value.parse() {
                            inbox.push(AppEvent::SelectHistoryRoom(id));
                        }
                    }
                }),
        );
        c.add(
            widget(Button::new("清理记录").kind(ButtonKind::Ghost))
                .visible(move || stats.with(|model| !model.history.is_empty()))
                .on_activate({
                    let inbox = inbox.clone();
                    move || inbox.push(AppEvent::AskClearStats)
                }),
        );
    })
}

fn trend_body(model: &StatsModel) -> AnyView {
    let Some(trend) = &model.trend else {
        return ().into_any();
    };
    let start = trend.start.clone();
    let end = trend.end.clone();
    column()
        .gap(10.0)
        .with(|c| {
            c.add(trend_summary(trend));
            c.add(widget(heading("在线人数")));
            c.add(trend_chart(
                trend.viewers.clone(),
                "在线人数趋势",
                start.clone(),
                end.clone(),
            ));
            c.add(widget(heading("关注数")));
            c.add(trend_chart(
                trend.followers.clone(),
                "关注数趋势",
                start,
                end,
            ));
        })
        .into_any()
}

fn trend_summary(trend: &TrendModel) -> AnyView {
    let viewers = latest_sample(&trend.viewers);
    let followers = latest_sample(&trend.followers);
    widget(Stack::bar(24.0))
        .with(|c| {
            c.add(metric("当前在线", viewers));
            c.add(metric("关注数", followers));
            c.add(metric("采样次数", trend.count.to_string()));
        })
        .into_any()
}

fn latest_sample(samples: &[(i64, Option<f64>)]) -> String {
    samples
        .iter()
        .rev()
        .find_map(|(_, value)| {
            value.map(|value| {
                if (value - value.trunc()).abs() < f64::EPSILON {
                    (value as i64).to_string()
                } else {
                    format!("{value:.1}")
                }
            })
        })
        .unwrap_or_else(|| "—".into())
}

fn metric(label: &str, value: String) -> AnyView {
    column()
        .gap(2.0)
        .with(|c| {
            c.add(widget(muted(label)));
            c.add(widget(heading(value)));
        })
        .into_any()
}

fn trend_chart(
    samples: Vec<(i64, Option<f64>)>,
    label: &str,
    start: String,
    end: String,
) -> El<TimeSeriesChart> {
    let mut chart = TimeSeriesChart::from_samples(samples)
        .label(label)
        .unit("人")
        .time_labels(start, end);
    let layout = Arc::make_mut(&mut chart.style.layout);
    layout.width = Some(LengthSpec::Fill);
    layout.min_width = Some(LengthSpec::Px(0.0));
    layout.height = Some(LengthSpec::Px(TimeSeriesChart::INTRINSIC_HEIGHT));
    layout.flex_shrink = Some(1.0);
    widget(chart)
}

fn history_count(stats: Signal<StatsModel>) -> El<Text> {
    widget(muted("")).bind(move |text| {
        let count = stats.with(|model| model.history.len());
        let next = format!("共 {count} 条记录");
        if text.value != next {
            text.value = next;
        }
    })
}

fn history_columns() -> El<Stack, Vec<AnyView>> {
    history_cells([
        muted("采集时间"),
        muted("在线人数"),
        muted("关注数"),
        muted("直播状态"),
    ])
}

fn history_rows(model: &StatsModel) -> AnyView {
    column()
        .gap(8.0)
        .with(|c| {
            for row in &model.history {
                c.add(history_line(row));
            }
        })
        .into_any()
}

fn history_line(row: &HistoryRow) -> AnyView {
    history_cells([
        Text::new(&row.time),
        Text::new(&row.viewers),
        Text::new(&row.followers),
        Text::new(&row.status),
    ])
    .key(format!("{}-{}", row.room_id, row.captured_at))
    .into_any()
}

fn history_cells(texts: [Text; 4]) -> El<Stack, Vec<AnyView>> {
    widget(Stack::bar(6.0)).with(|c| {
        for (text, grow) in texts.into_iter().zip([1.7, 1.0, 1.0, 1.0]) {
            c.add(widget(cell(text, grow)));
        }
    })
}

pub(super) fn settings_page(
    page: Signal<Page>,
    settings: Signal<SettingsModel>,
    inbox: Inbox,
) -> AnyView {
    scroll("设置")
        .visible(move || page.get() == Page::Settings)
        .with(move |c| {
            c.add(widget(heading("设置")));
            c.add(widget(muted("让工具箱更合你的习惯。")));
            c.add(widget(columns()).with(|c| {
                c.add(widget(lane()).with(|c| {
                    c.add(widget(heading("外观")));
                    c.add(dynamic(settings, {
                        let inbox = inbox.clone();
                        move |model| appearance(model, inbox.clone())
                    }));
                }));
                c.add(widget(lane()).with(|c| {
                    c.add(dynamic(settings, {
                        let inbox = inbox.clone();
                        move |model| desktop_preferences(model, inbox.clone())
                    }));
                    c.add(widget(heading("关于")));
                    c.add(about());
                }));
            }));
        })
        .into_any()
}

fn appearance(model: &SettingsModel, inbox: Inbox) -> AnyView {
    widget(AppearanceSection::new(model.theme, model.appearance).available_materials(materials()))
        .on_cx(move |_section, event: &AppearanceEvent, cx| {
            inbox.push(AppEvent::Appearance(event.clone()));
            cx.dispatch_program(Wake);
        })
        .into_any()
}

fn desktop_preferences(model: &SettingsModel, inbox: Inbox) -> AnyView {
    let font = model.font_size;
    let opacity = model.background_opacity;
    widget(panel(SemanticColorRole::Selected))
        .with(|c| {
            c.add(widget(heading("弹幕窗偏好")));
            c.add(widget(muted(
                "这些设置会保存在本机，并应用到下一次弹幕窗口。",
            )));
            c.add(row().gap(8.0).with(|c| {
                c.add(widget(muted(format!("字号 {font:.0} px"))));
                c.add(widget(Button::new("A−")).on_activate({
                    let inbox = inbox.clone();
                    move || inbox.push(AppEvent::DesktopFontSize(font - 2.0))
                }));
                c.add(widget(Button::new("A+")).on_activate({
                    let inbox = inbox.clone();
                    move || inbox.push(AppEvent::DesktopFontSize(font + 2.0))
                }));
            }));
            c.add(row().gap(8.0).with(|c| {
                c.add(widget(muted(format!("背景透明度 {:.0}%", opacity * 100.0))));
                let next = if opacity >= 1.0 { 0.0 } else { opacity + 0.25 };
                c.add(widget(Button::new("调整透明度")).on_activate({
                    let inbox = inbox.clone();
                    move || inbox.push(AppEvent::DesktopBackgroundOpacity(next))
                }));
            }));
        })
        .into_any()
}

fn about() -> impl IntoView {
    widget(AboutSection::new(
        AboutMetadata::new("Nana播播工具箱", env!("CARGO_PKG_VERSION"))
            .description("B 站直播工具箱：账号登录、房间连接、弹幕助手与数据。"),
    ))
}

fn materials() -> Vec<WindowMaterialMode> {
    if cfg!(target_os = "windows") {
        vec![
            WindowMaterialMode::Vibrancy,
            WindowMaterialMode::Mica,
            WindowMaterialMode::Acrylic,
        ]
    } else {
        Vec::new()
    }
}

fn cell(mut text: Text, grow: f32) -> Text {
    let layout = Arc::make_mut(&mut text.style.layout);
    layout.width = Some(LengthSpec::Px(0.0));
    layout.min_width = Some(LengthSpec::Px(0.0));
    layout.flex_grow = Some(grow);
    layout.flex_shrink = Some(1.0);
    layout.white_space_nowrap = true;
    layout.text_overflow_ellipsis = true;
    text
}

fn scroll(label: &str) -> El<ScrollView> {
    let mut scroll = ScrollView::new(ScrollAxes::Vertical).label(label.to_owned());
    let layout = Arc::make_mut(&mut scroll.style.layout);
    layout.width = Some(LengthSpec::Fill);
    layout.height = Some(LengthSpec::Fill);
    layout.min_height = Some(LengthSpec::Px(0.0));
    layout.flex_grow = Some(1.0);
    widget(scroll)
}
