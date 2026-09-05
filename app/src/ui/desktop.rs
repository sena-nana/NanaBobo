use super::*;
use crate::session::DesktopDanmakuPhase;

pub struct DesktopDanmakuView {
    root: Entity<Stack>,
    toolbar: Entity<Stack>,
    feed: Feed,
    drag_handle: Option<Entity<Text>>,
    last: Option<Revisions>,
}
impl DesktopDanmakuView {
    pub fn mount(document: &mut RuntimeDocument, s: &Session) -> Result<Self, FrameworkError> {
        let id = document.document();
        let (root, toolbar, content) = document.context_mut().build(id, |ui| {
            let root = ui.child("desktop-root", Stack::fill_column(4.0).padding(6.0));
            let mut parts = None;
            ui.nest(root, |ui| {
                let toolbar = ui.child(
                    "toolbar",
                    Stack::column(4.0)
                        .with_layout(|l| l.background = Some([0.025, 0.03, 0.04, 0.9])),
                );
                let content = ui.child("content", Stack::fill_column(0.0));
                parts = Some((toolbar, content));
            });
            let (toolbar, content) = parts.unwrap();
            (root, toolbar, content)
        })?;
        let cx = document.context_mut();
        cx.set_theme(nana_ui::ThemeMode::Dark)?;
        let feed = Feed::mount(cx, content, s)?;
        let mut view = Self {
            root,
            toolbar,
            feed,
            drag_handle: None,
            last: None,
        };
        view.sync(document, s)?;
        Ok(view)
    }
    pub fn sync(
        &mut self,
        document: &mut RuntimeDocument,
        s: &Session,
    ) -> Result<(), FrameworkError> {
        let cx = document.context_mut();
        let changed = self.last.is_none_or(|r| {
            r.desktop != s.revisions.desktop
                || r.messages != s.revisions.messages
                || r.room != s.revisions.room
        });
        if changed {
            let mut actions = Vec::new();
            let mut drag_handle = None;
            cx.mount(self.toolbar, |ui| {
                if s.desktop.phase == DesktopDanmakuPhase::Locked {
                    use nanabobo_core::models::DanmakuConnectionState::*;
                    if !matches!(s.danmaku.status.state, Connected) {
                        ui.child(
                            "connection",
                            super::feed::desktop_text(danmaku_label(&s.danmaku.status), 14.0),
                        )?;
                        if let Some(error) = &s.danmaku.status.message {
                            ui.child("error", super::feed::desktop_text(error, 14.0))?;
                        }
                    }
                    return Ok(());
                }
                drag_handle = Some(ui.child("drag-handle", Text::new("拖动窗口"))?);
                ui.with_child("window-actions", Stack::row(4.0), |ui| {
                    ui.child("status", Text::new(danmaku_label(&s.danmaku.status)))?;
                    actions.push((
                        ui.child("lock", Button::new("锁定"))?,
                        AppEvent::LockDesktopDanmaku,
                    ));
                    actions.push((
                        ui.child("close", Button::new("关闭"))?,
                        AppEvent::CloseDesktopDanmaku,
                    ));
                    Ok(())
                })?;
                ui.with_child("text-actions", Stack::row(4.0), |ui| {
                    actions.push((
                        ui.child("smaller", Button::new("A−"))?,
                        AppEvent::DesktopFontSize(s.desktop.settings.font_size - 2.0),
                    ));
                    ui.child(
                        "font",
                        Text::new(format!("{}", s.desktop.settings.font_size as u32)),
                    )?;
                    actions.push((
                        ui.child("larger", Button::new("A+"))?,
                        AppEvent::DesktopFontSize(s.desktop.settings.font_size + 2.0),
                    ));
                    let alpha = s.desktop.settings.background_opacity;
                    actions.push((
                        ui.child("background", background_button(alpha))?,
                        AppEvent::DesktopBackgroundOpacity(if alpha >= 1.0 {
                            0.0
                        } else {
                            alpha + 0.25
                        }),
                    ));
                    Ok(())
                })?;
                if !s.danmaku.following {
                    actions.push((
                        ui.child(
                            "latest",
                            Button::new(if s.danmaku.unread > 0 {
                                format!("回到最新 · {} 条新消息", s.danmaku.unread)
                            } else {
                                "回到最新".into()
                            }),
                        )?,
                        AppEvent::FollowLatest,
                    ));
                }
                if let Some(error) = s
                    .desktop
                    .error
                    .as_ref()
                    .or(s.danmaku.status.message.as_ref())
                {
                    ui.child("error", Text::new(error))?;
                }
                Ok(())
            })?;
            self.drag_handle = drag_handle;
            for (node, event) in actions {
                action(cx, node, s.inbox.clone(), event)?;
            }
            cx.update_component(self.root, |view, _| {
                *view = Stack::fill_column(4.0).padding(6.0).with_layout(|layout| {
                    layout.background =
                        Some([0.02, 0.025, 0.035, s.desktop.settings.background_opacity])
                });
            })?;
        }
        if changed || self.feed.needs_measure(cx) {
            self.feed.sync(cx, s)?;
        }
        self.last = Some(s.revisions);
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

fn background_button(alpha: f32) -> Button {
    let mut button = Button::new(format!("背景 {}%", (alpha * 100.0).round() as u32));
    let layout = Arc::make_mut(&mut button.style.layout);
    layout.width = Some(LengthSpec::Px(102.0));
    layout.min_width = Some(LengthSpec::Px(102.0));
    layout.flex_shrink = Some(0.0);
    button
}
