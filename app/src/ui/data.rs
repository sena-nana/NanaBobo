use std::sync::Arc;

use nana_ui::runtime::{
    AppContext, Button, EmptyState, Entity, FrameworkError, LengthSpec, ScrollAxes, ScrollView,
    Select, SelectChanged, SelectOption, Stack, TabOption, Tabs, TabsEvent, Text, TimeSeriesChart,
};
use nana_ui::{AppearanceEvent, ButtonKind, ThemeMode, WindowMaterialMode};

use crate::session::{live_label, timestamp, AppEvent, Session, StatsTab};

use super::{action, heading, muted};

pub(super) fn mount_data(
    cx: &mut AppContext,
    parent: Entity<Stack>,
    session: &Session,
    retained_tabs: &mut Option<Entity<Tabs>>,
) -> Result<(), FrameworkError> {
    let selected = match session.stats.tab {
        StatsTab::Trend => "trend",
        StatsTab::History => "history",
    };
    // Preserve the retained tab options and roving focus when refreshing data.
    // Replacing Tabs with Tabs::new also replaces its native option identities.
    let mut tab_strip = retained_tabs
        .map(|entity| cx.read(entity, Clone::clone))
        .transpose()?
        .unwrap_or_else(|| {
            Tabs::new(selected).options([
                TabOption::new("trend", "趋势").draggable(false),
                TabOption::new("history", "历史").draggable(false),
            ])
        });
    tab_strip.selected = Some(selected.into());
    let mut room_select = None;
    let mut tabs = None;
    let mut clear = None;
    let history = session.stats.current();
    let rooms = session.stats.rooms();
    cx.mount(parent, |ui| {
        ui.child("heading", heading("数据"))?;
        if rooms.is_empty() {
            ui.child(
                "empty",
                EmptyState::new("还没有采集记录")
                    .message("连接直播间后，会自动保存在线人数与关注数。"),
            )?;
            return Ok(());
        }
        ui.with_child(
            "toolbar",
            Stack::fill_row(12.0).grow(0.0).shrink(0.0),
            |ui| {
                room_select =
                    Some(
                        ui.child(
                            "room",
                            Select::new(session.stats.selected_room.map(|id| id.to_string()))
                                .options(rooms.iter().map(|(id, name)| {
                                    SelectOption::new(id.to_string(), name.clone())
                                }))
                                .placeholder("选择直播间"),
                        )?,
                    );
                if !history.is_empty() {
                    clear =
                        Some(ui.child("clear", Button::new("清理记录").kind(ButtonKind::Ghost))?);
                }
                Ok(())
            },
        )?;
        tabs = Some(ui.child("tabs", tab_strip)?);
        if history.is_empty() {
            ui.child("empty-room", EmptyState::new("这个直播间还没有记录"))?;
            return Ok(());
        }
        let first = history.first().expect("nonempty history");
        let last = history.last().expect("nonempty history");
        let start = timestamp(first.captured_at);
        let end = timestamp(last.captured_at);
        match session.stats.tab {
            StatsTab::Trend => {
                ui.with_child("trends", scroll("直播间趋势"), |ui| {
                    ui.with_child("content", Stack::column(10.0), |ui| {
                        ui.child(
                            "updated",
                            muted(format!("最近采集 {} · {} 条记录", end, history.len())),
                        )?;
                        ui.child("viewer-title", heading("在线人数"))?;
                        ui.child(
                            "viewers",
                            TimeSeriesChart::from_samples(session.stats.viewer_series())
                                .label("在线人数趋势")
                                .unit("人")
                                .time_labels(start.clone(), end.clone()),
                        )?;
                        ui.child("follower-title", heading("关注数"))?;
                        ui.child(
                            "followers",
                            TimeSeriesChart::from_samples(session.stats.follower_series())
                                .label("关注数趋势")
                                .unit("人")
                                .time_labels(start.clone(), end.clone()),
                        )?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
            StatsTab::History => {
                ui.child("count", muted(format!("共 {} 条记录", history.len())))?;
                ui.with_child(
                    "columns",
                    Stack::fill_row(12.0).grow(0.0).shrink(0.0),
                    |ui| {
                        ui.child("time", column(muted("采集时间"), 150.0))?;
                        ui.child("viewers", column(muted("在线人数"), 100.0))?;
                        ui.child("followers", column(muted("关注数"), 100.0))?;
                        ui.child("status", column(muted("直播状态"), 90.0))?;
                        Ok(())
                    },
                )?;
                ui.with_child("history", scroll("全部历史记录"), |ui| {
                    ui.with_child("rows", Stack::column(8.0), |ui| {
                        for snapshot in history.iter().rev() {
                            ui.with_child(
                                format!("{}-{}", snapshot.room_id, snapshot.captured_at),
                                Stack::fill_row(12.0).grow(0.0).shrink(0.0),
                                |ui| {
                                    ui.child(
                                        "time",
                                        column(Text::new(timestamp(snapshot.captured_at)), 150.0),
                                    )?;
                                    ui.child(
                                        "viewers",
                                        column(Text::new(snapshot.viewer_count.to_string()), 100.0),
                                    )?;
                                    ui.child(
                                        "followers",
                                        column(
                                            Text::new(
                                                snapshot
                                                    .follower_count
                                                    .map(|n| n.to_string())
                                                    .unwrap_or_else(|| "—".into()),
                                            ),
                                            100.0,
                                        ),
                                    )?;
                                    ui.child(
                                        "status",
                                        column(Text::new(live_label(&snapshot.live_status)), 90.0),
                                    )?;
                                    Ok(())
                                },
                            )?;
                        }
                        Ok(())
                    })?;
                    Ok(())
                })?;
            }
        }
        Ok(())
    })?;
    *retained_tabs = tabs;
    if let Some(entity) = room_select {
        let inbox = session.inbox.clone();
        cx.on_keyed(
            entity,
            "select-history-room",
            move |_, event: &SelectChanged, _| {
                if let Ok(id) = event.value.parse() {
                    inbox.push(AppEvent::SelectHistoryRoom(id));
                }
            },
        )?;
    }
    if let Some(entity) = tabs {
        let inbox = session.inbox.clone();
        cx.on_keyed(entity, "select-data-tab", move |_, event: &TabsEvent, _| {
            if let TabsEvent::Select(value) = event {
                let tab = if value.as_ref() == "history" {
                    StatsTab::History
                } else {
                    StatsTab::Trend
                };
                inbox.push(AppEvent::SelectStatsTab(tab));
            }
        })?;
    }
    if let Some(entity) = clear {
        action(cx, entity, session.inbox.clone(), AppEvent::AskClearStats)?;
    }
    Ok(())
}

fn column(mut text: Text, width: f32) -> Text {
    Arc::make_mut(&mut text.style.layout).width = Some(LengthSpec::Px(width));
    text
}

pub(super) fn mount_settings(
    cx: &mut AppContext,
    parent: Entity<Stack>,
    session: &Session,
) -> Result<(), FrameworkError> {
    let mut theme = None;
    let mut material = None;
    let mut opacity = None;
    let material_value = match session.appearance.window_material() {
        WindowMaterialMode::Solid => "solid",
        WindowMaterialMode::Translucent => "translucent",
        WindowMaterialMode::Mica => "mica",
        WindowMaterialMode::Acrylic => "acrylic",
        WindowMaterialMode::Vibrancy => "vibrancy",
    };
    let mut materials = vec![
        SelectOption::new("solid", "实色"),
        SelectOption::new("translucent", "透明"),
    ];
    if cfg!(target_os = "windows") {
        materials.extend([
            SelectOption::new("mica", "云母"),
            SelectOption::new("acrylic", "毛玻璃"),
        ]);
    }
    cx.mount(parent, |ui| {
        ui.child("heading", heading("设置"))?;
        ui.with_child("settings-scroll", scroll("设置"), |ui| {
            ui.with_child("settings-content", Stack::column(14.0), |ui| {
                ui.child("appearance", heading("外观"))?;
                ui.child("theme-label", muted("主题"))?;
                theme = Some(
                    ui.child(
                        "theme",
                        Select::new(Some(match session.theme {
                            ThemeMode::Dark => "dark",
                            ThemeMode::Light => "light",
                        }))
                        .options([
                            SelectOption::new("light", "浅色"),
                            SelectOption::new("dark", "深色"),
                        ]),
                    )?,
                );
                ui.child("material-label", muted("窗口背景"))?;
                material = Some(ui.child(
                    "material",
                    Select::new(Some(material_value)).options(materials.clone()),
                )?);
                if session.appearance.window_material() != WindowMaterialMode::Solid {
                    ui.child("opacity-label", muted("背景浓度"))?;
                    let current = session.appearance.backdrop_opacity();
                    let selected = if current < 0.52 {
                        "light"
                    } else if current < 0.745 {
                        "medium"
                    } else {
                        "strong"
                    };
                    opacity = Some(ui.child(
                        "opacity",
                        Tabs::new(selected).options([
                            TabOption::new("light", "轻盈").draggable(false),
                            TabOption::new("medium", "适中").draggable(false),
                            TabOption::new("strong", "浓郁").draggable(false),
                        ]),
                    )?);
                }
                ui.child("about", heading("关于"))?;
                ui.child("name", Text::new("Nana播播工具箱"))?;
                ui.child(
                    "version",
                    muted(format!("版本 {}", env!("CARGO_PKG_VERSION"))),
                )?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })?;
    if let Some(entity) = theme {
        let inbox = session.inbox.clone();
        cx.on_keyed(entity, "theme", move |_, event: &SelectChanged, _| {
            let theme = if event.value.as_ref() == "light" {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            };
            inbox.push(AppEvent::Appearance(AppearanceEvent::Theme(theme)));
        })?;
    }
    if let Some(entity) = material {
        let inbox = session.inbox.clone();
        cx.on_keyed(entity, "material", move |_, event: &SelectChanged, _| {
            let value = match event.value.as_ref() {
                "translucent" => WindowMaterialMode::Translucent,
                "mica" => WindowMaterialMode::Mica,
                "acrylic" => WindowMaterialMode::Acrylic,
                _ => WindowMaterialMode::Solid,
            };
            inbox.push(AppEvent::Appearance(AppearanceEvent::WindowMaterial(value)));
        })?;
    }
    if let Some(entity) = opacity {
        let inbox = session.inbox.clone();
        cx.on_keyed(entity, "opacity", move |_, event: &TabsEvent, _| {
            if let TabsEvent::Select(value) = event {
                let opacity = match value.as_ref() {
                    "light" => 0.4,
                    "strong" => 0.85,
                    _ => 0.64,
                };
                inbox.push(AppEvent::Appearance(AppearanceEvent::BackdropOpacity(
                    opacity,
                )));
            }
        })?;
    }
    Ok(())
}

fn scroll(label: &str) -> ScrollView {
    let mut scroll = ScrollView::new(ScrollAxes::Vertical).label(label.to_owned());
    let layout = std::sync::Arc::make_mut(&mut scroll.style.layout);
    layout.width = Some(LengthSpec::Fill);
    layout.height = Some(LengthSpec::Fill);
    layout.min_height = Some(LengthSpec::Px(0.0));
    layout.flex_grow = Some(1.0);
    scroll
}
