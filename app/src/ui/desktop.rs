use super::feed::Feed;
use super::*;
use crate::session::DesktopDanmakuPhase;
use nana_ui::runtime::view::{
    column, dynamic, entity_ref, row, signal, text, widget, with_refs, AnyView, El, EntityRef,
    IntoView, Signal,
};

#[derive(Clone, PartialEq)]
struct ChromeModel {
    locked: bool,
    connected: bool,
    status: String,
    connection_detail: Option<String>,
    font_size: f32,
    background: f32,
    following: bool,
    unread: usize,
    error: Option<String>,
}

pub struct DesktopDanmakuView {
    root: Entity<Stack>,
    feed: Feed,
    drag_handle: Option<Entity<Text>>,
    drag_ref: EntityRef<Text>,
    chrome: Signal<ChromeModel>,
    last: Option<Revisions>,
}

impl DesktopDanmakuView {
    pub fn mount(
        document: &mut RuntimeDocument,
        session: &Session,
    ) -> Result<Self, FrameworkError> {
        let id = document.document();
        let mut signals = None;
        let (_mounted, (root, content)) = document.context_mut().mount_view_root(id, || {
            let chrome = signal(chrome_model(session));
            let root_ref = entity_ref::<Stack>();
            let content_ref = entity_ref::<Stack>();
            let drag_ref = entity_ref::<Text>();
            let inbox = session.inbox.clone();
            signals = Some((chrome, drag_ref));
            with_refs(
                widget(Stack::fill_column(4.0).padding(6.0))
                    .key("desktop-root")
                    .entity_ref(root_ref)
                    .with(move |c| {
                        c.add(
                            widget(
                                Stack::column(8.0)
                                    .padding(10.0)
                                    .surface(SemanticColorRole::Surface),
                            )
                            .key("toolbar")
                            .children(dynamic(chrome, move |model| {
                                chrome_view(model, inbox.clone(), drag_ref)
                            })),
                        );
                        c.add(
                            widget(Stack::fill_column(0.0))
                                .key("content")
                                .entity_ref(content_ref),
                        );
                    }),
                (root_ref, content_ref),
            )
        })?;
        let (chrome, drag_ref) = signals.expect("desktop signals");
        let cx = document.context_mut();
        cx.set_theme(nana_ui::ThemeMode::Dark)?;
        let feed = Feed::mount(cx, content, session)?;
        let mut view = Self {
            root,
            feed,
            drag_handle: None,
            drag_ref,
            chrome,
            last: None,
        };
        view.sync(document, session)?;
        Ok(view)
    }

    pub fn sync(
        &mut self,
        document: &mut RuntimeDocument,
        session: &Session,
    ) -> Result<(), FrameworkError> {
        let cx = document.context_mut();
        let changed = self.last.is_none_or(|revisions| {
            revisions.desktop != session.revisions.desktop
                || revisions.messages != session.revisions.messages
                || revisions.room != session.revisions.room
        });
        if changed {
            publish(&self.chrome, chrome_model(session));
            cx.flush_reactive()?;
            self.drag_handle = self
                .drag_ref
                .get()
                .filter(|handle| cx.world().contains(handle.stable_id()));
            let opacity = session.desktop.settings.background_opacity;
            cx.update_component(self.root, |view, _| {
                *view = Stack::fill_column(4.0)
                    .padding(6.0)
                    .with_layout(|layout| layout.background = Some([0.02, 0.025, 0.035, opacity]));
            })?;
        }
        if changed || self.feed.needs_measure(cx) {
            self.feed.sync(cx, session)?;
        }
        self.last = Some(session.revisions);
        Ok(())
    }

    pub fn drag_handle_bounds(&self, document: &RuntimeDocument) -> Option<LayoutBox> {
        document
            .context()
            .world()
            .layout_box(self.drag_handle?.stable_id())
    }

    pub fn needs_layout_sync(&self, document: &RuntimeDocument) -> bool {
        self.feed.needs_measure(document.context())
    }
}

fn chrome_model(session: &Session) -> ChromeModel {
    let locked = session.desktop.phase == DesktopDanmakuPhase::Locked;
    ChromeModel {
        locked,
        connected: matches!(
            session.danmaku.status.state,
            nanabobo_core::models::DanmakuConnectionState::Connected
        ),
        status: danmaku_label(&session.danmaku.status).into(),
        connection_detail: session.danmaku.status.message.clone(),
        font_size: session.desktop.settings.font_size,
        background: session.desktop.settings.background_opacity,
        following: session.danmaku.following,
        unread: session.danmaku.unread,
        error: session
            .desktop
            .error
            .clone()
            .or_else(|| session.danmaku.status.message.clone()),
    }
}

fn chrome_view(model: &ChromeModel, inbox: Inbox, drag: EntityRef<Text>) -> AnyView {
    if model.locked {
        let status = model.status.clone();
        let detail = model.connection_detail.clone();
        return column()
            .gap(6.0)
            .with(|c| {
                c.add(row().gap(8.0).with(|c| {
                    c.add(widget(feed::desktop_text("实时弹幕", 15.0)));
                    c.add(widget(status_badge(&status, model.connected)));
                }));
                c.add(row().gap(8.0).with(|c| {
                    c.add(activate(
                        "解锁",
                        inbox.clone(),
                        AppEvent::AdjustDesktopDanmaku,
                    ));
                    c.add(activate(
                        "关闭",
                        inbox.clone(),
                        AppEvent::CloseDesktopDanmaku,
                    ));
                }));
                if let Some(detail) = detail {
                    c.add(widget(feed::desktop_text(detail, 12.0)));
                }
            })
            .into_any();
    }
    let status = model.status.clone();
    let font_size = model.font_size;
    let background = model.background;
    let following = model.following;
    let unread = model.unread;
    let error = model.error.clone();
    let connected = model.connected;
    let detail = model.connection_detail.clone();
    column()
        .gap(6.0)
        .with(move |c| {
            c.add(row().gap(8.0).with(|c| {
                c.add(widget(feed::desktop_text("实时弹幕", 15.0)));
                c.add(widget(status_badge(&status, connected)));
            }));
            c.add(row().gap(8.0).with(|c| {
                c.add(
                    widget(feed::desktop_text("拖动窗口", 12.0))
                        .key("drag-handle")
                        .entity_ref(drag),
                );
                c.add(activate(
                    "锁定",
                    inbox.clone(),
                    AppEvent::LockDesktopDanmaku,
                ));
                c.add(activate(
                    "关闭",
                    inbox.clone(),
                    AppEvent::CloseDesktopDanmaku,
                ));
            }));
            c.add(row().gap(6.0).with(|c| {
                c.add(activate(
                    "A−",
                    inbox.clone(),
                    AppEvent::DesktopFontSize(font_size - 2.0),
                ));
                c.add(text(format!("{}", font_size as u32)));
                c.add(activate(
                    "A+",
                    inbox.clone(),
                    AppEvent::DesktopFontSize(font_size + 2.0),
                ));
                c.add(background_button(background).on_activate({
                    let inbox = inbox.clone();
                    let next = if background >= 1.0 {
                        0.0
                    } else {
                        background + 0.25
                    };
                    move || inbox.push(AppEvent::DesktopBackgroundOpacity(next))
                }));
            }));
            if !following {
                let label = if unread > 0 {
                    format!("回到最新 · {unread} 条新消息")
                } else {
                    "回到最新".into()
                };
                c.add(activate(label, inbox.clone(), AppEvent::FollowLatest));
            }
            if let Some(error) = error {
                c.add(text(error));
            } else if let Some(detail) = detail {
                c.add(widget(feed::desktop_text(detail, 12.0)));
            }
        })
        .into_any()
}

fn status_badge(status: &str, connected: bool) -> Text {
    let label = if connected {
        format!("● {status}")
    } else {
        format!("○ {status}")
    };
    let mut badge = feed::desktop_text(label, 12.0);
    let layout = Arc::make_mut(&mut badge.style.layout);
    layout.padding_left = Some(LengthSpec::Px(6.0));
    layout.padding_right = Some(LengthSpec::Px(6.0));
    layout.padding_top = Some(LengthSpec::Px(3.0));
    layout.padding_bottom = Some(LengthSpec::Px(3.0));
    layout.background = Some(if connected {
        [0.08, 0.3, 0.2, 0.85]
    } else {
        [0.25, 0.22, 0.14, 0.85]
    });
    badge
}

fn activate(label: impl Into<String>, inbox: Inbox, event: AppEvent) -> impl IntoView {
    widget(Button::new(label)).on_activate(move || inbox.push(event.clone()))
}

fn background_button(alpha: f32) -> El<Button> {
    let mut button = Button::new(format!("背景 {}%", (alpha * 100.0).round() as u32));
    let layout = Arc::make_mut(&mut button.style.layout);
    layout.width = Some(LengthSpec::Px(102.0));
    layout.min_width = Some(LengthSpec::Px(102.0));
    layout.flex_shrink = Some(0.0);
    widget(button)
}
