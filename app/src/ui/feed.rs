use super::*;
use nana_ui::runtime::view::{entity_ref, widget, with_refs};
use std::collections::HashMap;
pub(super) struct Feed {
    scroll: Entity<ScrollView>,
    list: Entity<List>,
    items: VirtualListItems<u64, Stack>,
    heights: HashMap<u64, f32>,
    width: f32,
    viewport: f32,
    font_size: f32,
}
impl Feed {
    pub fn mount(
        cx: &mut AppContext,
        parent: Entity<Stack>,
        s: &Session,
    ) -> Result<Self, FrameworkError> {
        let inbox = s.inbox.clone();
        let (_mounted, (scroll, list)) = cx.mount_view(parent.stable_id(), || {
            let scroll_ref = entity_ref::<ScrollView>();
            let list_ref = entity_ref::<List>();
            let mut scroll = ScrollView::new(ScrollAxes::Vertical)
                .label("实时弹幕")
                .follow_end(s.danmaku.following);
            let layout = Arc::make_mut(&mut scroll.style.layout);
            layout.width = Some(LengthSpec::Fill);
            layout.height = Some(LengthSpec::Fill);
            layout.min_height = Some(LengthSpec::Px(0.0));
            layout.flex_grow = Some(1.0);
            with_refs(
                widget(scroll)
                    .key("feed")
                    .entity_ref(scroll_ref)
                    .on_cx(move |view, event: &UserScroll, _| {
                        view.follow_end = event.at_end;
                        inbox.push(AppEvent::Reading {
                            following: event.at_end,
                            offset: event.offset.y,
                        });
                    })
                    .children(
                        widget(List::new().label("弹幕消息"))
                            .key("rows")
                            .entity_ref(list_ref),
                    ),
                (scroll_ref, list_ref),
            )
        })?;
        Ok(Self {
            scroll,
            list,
            items: VirtualListItems::default(),
            heights: HashMap::new(),
            width: 0.0,
            viewport: 0.0,
            font_size: s.desktop.settings.font_size,
        })
    }
    fn actual_anchor(&self, cx: &AppContext) -> Option<(u64, f32)> {
        let viewport = cx.world().layout_box(self.scroll.stable_id())?;
        let offset = cx
            .world()
            .scroll_offset(self.scroll.stable_id())
            .unwrap_or_default()
            .y;
        self.items.mounted_keys().iter().find_map(|key| {
            let row = self.items.entity(key)?;
            let bounds = cx.world().layout_box(row.stable_id())?;
            let y = bounds.y - viewport.y - offset;
            (bounds.height > 0.0 && y + bounds.height > 0.0).then_some((*key, -y))
        })
    }
    pub fn needs_measure(&self, cx: &AppContext) -> bool {
        let Some(metrics) = cx.world().scroll_metrics(self.scroll.stable_id()) else {
            return true;
        };
        if (metrics.viewport_width - self.width).abs() > 0.5
            || (metrics.viewport_height - self.viewport).abs() > 0.5
        {
            return true;
        }
        self.items.mounted_keys().iter().any(|key| {
            self.items
                .entity(key)
                .and_then(|e| cx.world().layout_box(e.stable_id()))
                .is_none_or(|b| {
                    b.height <= 0.0
                        || (self.heights.get(key).copied().unwrap_or(64.0) - b.height).abs() > 0.5
                })
        })
    }
    pub fn sync(&mut self, cx: &mut AppContext, s: &Session) -> Result<(), FrameworkError> {
        let metrics = cx.world().scroll_metrics(self.scroll.stable_id());
        let width = metrics
            .map(|m| m.viewport_width)
            .filter(|w| *w > 0.0)
            .unwrap_or(700.0);
        let viewport = metrics
            .map(|m| m.viewport_height)
            .filter(|h| *h > 0.0)
            .unwrap_or(360.0);
        let anchor = self.actual_anchor(cx);
        if (width - self.width).abs() > 0.5 || self.font_size != s.desktop.settings.font_size {
            self.heights.clear();
        } else {
            for key in self.items.mounted_keys() {
                if let Some(bounds) = self
                    .items
                    .entity(key)
                    .and_then(|e| cx.world().layout_box(e.stable_id()))
                {
                    if bounds.height > 0.0 {
                        self.heights.insert(*key, bounds.height);
                    }
                }
            }
        }
        self.width = width;
        self.font_size = s.desktop.settings.font_size;
        self.viewport = viewport;
        let keys: Vec<_> = s.danmaku.messages.iter().map(|m| m.id).collect();
        self.heights.retain(|key, _| keys.contains(key));
        let layout = VirtualListLayout::new(
            keys.iter()
                .map(|key| self.heights.get(key).copied().unwrap_or(64.0)),
        );
        let offset = if s.danmaku.following {
            (layout.total_extent() - viewport).max(0.0)
        } else if let Some((key, inset)) = anchor {
            let index = keys.iter().position(|k| *k == key).unwrap_or(0);
            (layout.extent(0..index) + inset)
                .max(0.0)
                .min((layout.total_extent() - viewport).max(0.0))
        } else {
            s.danmaku.scroll_offset
        };
        let window = cx.materialize_virtual_list(
            self.list,
            &mut self.items,
            &layout,
            offset,
            viewport,
            180.0,
            |i| keys[i],
            |_, _| Stack::column(4.0).padding_xy(8.0, 8.0),
        )?;
        for i in window.range.clone() {
            let row = &s.danmaku.messages[i];
            let entity = self.items.entity(&row.id).expect("row");
            let meta = desktop_text(
                format!(
                    "{}   {}",
                    row.message.sender_name,
                    timestamp(row.message.sent_at)
                ),
                (s.desktop.settings.font_size - 3.0).max(12.0),
            );
            let mut message = desktop_text(&row.message.text, s.desktop.settings.font_size);
            let layout = Arc::make_mut(&mut message.style.layout);
            layout.width = Some(LengthSpec::Fill);
            layout.min_width = Some(LengthSpec::Px(0.0));
            layout.white_space_nowrap = false;
            layout.text_overflow_ellipsis = false;
            layout.word_break = Some(WordBreakSpec::BreakWord);
            let children = cx
                .world()
                .node(entity.stable_id())
                .map(|node| node.children)
                .unwrap_or_default();
            if children.len() < 2 {
                cx.mount_view(entity.stable_id(), || {
                    (widget(meta).key("meta"), widget(message).key("message"))
                })?;
            } else {
                cx.update_component(Entity::<Text>::from_stable_id(children[0]), |text, _| {
                    *text = meta;
                })?;
                cx.update_component(Entity::<Text>::from_stable_id(children[1]), |text, _| {
                    *text = message;
                })?;
            }
        }
        cx.update_component(self.list, |list, _| {
            let l = Arc::make_mut(&mut list.style.layout);
            l.direction = Some(FlexDirection::Column);
            l.width = Some(LengthSpec::Fill);
            l.height = Some(LengthSpec::Shrink);
            l.flex_shrink = Some(0.0);
            l.padding_top = Some(LengthSpec::Px(
                window.leading_extent + (viewport - layout.total_extent()).max(0.0),
            ));
            l.padding_bottom = Some(LengthSpec::Px(window.trailing_extent));
        })?;
        cx.set_scroll_follow_end(self.scroll, s.danmaku.following)?;
        if !s.danmaku.following {
            cx.scroll_to(self.scroll, ScrollOffset { x: 0.0, y: offset })?;
            if let Some((key, inset)) = anchor {
                if let Some(row) = self.items.entity(&key) {
                    cx.restore_scroll_anchor(
                        self.scroll,
                        ScrollAnchor {
                            row: row.stable_id(),
                            viewport_y: -inset,
                        },
                    )?;
                }
            }
        }
        Ok(())
    }
}

pub(super) fn desktop_text(value: impl Into<String>, size: f32) -> Text {
    let mut text = Text::new(value);
    let layout = Arc::make_mut(&mut text.style.layout);
    layout.color = Some([1.0, 1.0, 1.0, 1.0]);
    layout.font_size = Some(size);
    layout.paint.text_shadow = Some(TextShadowSpec {
        offset_x: 1.0,
        offset_y: 1.0,
        blur_radius: 2.0,
        color: [0.0, 0.0, 0.0, 1.0],
    });
    layout.word_break = Some(WordBreakSpec::BreakWord);
    text
}
