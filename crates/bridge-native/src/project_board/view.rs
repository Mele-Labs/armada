//! The Dashboard tab drawn: a six-column grid of blocks, each a card with a tinted header band, a view switch and its rows.
//! A block's rows are drawn as a List, a Table or Cards from one `Item` (`model.rs`), so a block's own code is the function that
//! makes its items. While the board is edited a block's header is a handle that moves it and its corner resizes it; the same
//! moves are keys (`dashboard_move`, `dashboard_resize`).

use gpui::{div, prelude::*, px, AnyElement, Context, DragMoveEvent, FontWeight, MouseButton, Pixels, Role, SharedString, Toggled, Window};
use ipc::MetricState;

use super::board::Board;
use super::layout::{BlockKind, Placed, View, COLUMNS};
use super::metric;
use super::model::{self, Button, Fill, Item, Project, Shown, Tile};
use crate::generated::actions::ActionScope;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::{bone, ring_colour};
use crate::titlebar::parts::{card, glyph, sans, spinner, tip, Btn, Weight};

/// A block being moved or resized, as the drag carries it.
#[derive(Clone)]
struct BlockDrag {
    kind: BlockKind,
    resize: bool,
}

/// What follows the pointer while a block is dragged: nothing, since the block itself moves with it.
struct Ghost;

impl Render for Ghost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The hue a block's header band and glyph wear: what kind of block it is.
fn hue_of(kind: BlockKind) -> Token {
    match kind {
        BlockKind::Needs => token::YOU,
        BlockKind::Run => token::SKETCH_GREEN,
        BlockKind::Reviews => token::SKETCH_AMBER,
        BlockKind::Issues => token::SKETCH_TEAL,
        BlockKind::Features => token::ASK_FEATURES,
        BlockKind::Checks => token::SKETCH_GREY,
        BlockKind::Metric(_) => token::SKETCH_BLUE,
    }
}

pub(super) fn mark_of(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Needs => "bell",
        BlockKind::Run => "play",
        BlockKind::Reviews => "git-pull-request",
        BlockKind::Issues => "ticket",
        BlockKind::Features => "library",
        BlockKind::Checks => "list-checks",
        BlockKind::Metric(_) => "gauge",
    }
}

/// The room a block is drawn in.
#[derive(Clone, Copy)]
struct Grid {
    col: Pixels,
    row: Pixels,
    gap: Pixels,
}

impl Grid {
    fn left(&self, x: u8) -> Pixels {
        (self.col + self.gap) * f32::from(x)
    }

    fn top(&self, y: u8) -> Pixels {
        (self.row + self.gap) * f32::from(y)
    }

    fn wide(&self, w: u8) -> Pixels {
        self.col * f32::from(w) + self.gap * f32::from(w.saturating_sub(1))
    }

    fn tall(&self, h: u8) -> Pixels {
        self.row * f32::from(h) + self.gap * f32::from(h.saturating_sub(1))
    }
}

/// A chip with a fill: what kind of row it is.
fn chip(look: &Look, hue: Token, words: impl Into<SharedString>) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_SM))
        .bg(look.color(hue).opacity(0.16))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(look.color(hue))
        .whitespace_nowrap()
        .child(words.into())
}

impl Board {
    // ---- keys --------------------------------------------------------------------------------

    /// The block the keys are on.
    fn focused(&self, window: &Window) -> Option<BlockKind> {
        self.handles.iter().find(|(_, handle)| handle.is_focused(window)).map(|(kind, _)| *kind)
    }

    /// The registry's `dashboard` acts, pressed. The arrows move and resize only while the board is edited, and only the block
    /// that has the keys; with none they go on to whatever else answers them.
    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if !act.id.starts_with("dashboard_") {
            cx.propagate();
            return;
        }
        // Run or Cancel is asked: the keys are the card's.
        if self.asking.is_some() {
            return;
        }
        let Some(project) = self.project(cx) else { return };
        let (dx, dy) = match act.slot % 4 {
            0 => (-1, 0),
            1 => (1, 0),
            2 => (0, -1),
            _ => (0, 1),
        };
        match (act.id, self.focused(window)) {
            ("dashboard_edit", _) => self.toggle_edit(cx),
            ("dashboard_move", Some(kind)) if self.edit => self.edit_layout(&project.id, |layout| layout.nudge(kind, dx, dy), cx),
            ("dashboard_resize", Some(kind)) if self.edit => self.edit_layout(&project.id, |layout| layout.grow(kind, dx, dy), cx),
            ("dashboard_block_view", Some(kind)) => {
                let now = self.layout(&project.id).block(kind).map(|one| one.view).unwrap_or(View::List);
                let next = self.next_view(kind, now);
                self.edit_layout(&project.id, |layout| layout.set_view(kind, next), cx);
            }
            _ => cx.propagate(),
        }
    }

    /// The pointer moved with a block lifted: the block follows to the cell under it, or takes the size that reaches it.
    fn dragged(&mut self, event: &DragMoveEvent<BlockDrag>, pitch: (Pixels, Pixels), cx: &mut Context<Self>) {
        let drag = event.drag(cx).clone();
        let Some(project) = self.project(cx) else { return };
        let at = event.event.position - event.bounds.origin;
        let cell = ((at.x / pitch.0).floor() as i16, (at.y / pitch.1).floor() as i16);
        let Some(now) = self.layout(&project.id).block(drag.kind).copied() else { return };
        if drag.resize {
            self.edit_layout(&project.id, |layout| layout.resize_to(drag.kind, cell.0 - i16::from(now.x) + 1, cell.1 - i16::from(now.y) + 1), cx);
        } else {
            let grab = *self.grab.get_or_insert((cell.0 - i16::from(now.x), cell.1 - i16::from(now.y)));
            self.edit_layout(&project.id, |layout| layout.move_to(drag.kind, cell.0 - grab.0, cell.1 - grab.1), cx);
        }
    }

    /// The block comes off the board, the rest rise into its room, and the keys go back to the board.
    fn hide_block(&mut self, kind: BlockKind, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        self.edit_layout(&project.id, |layout| layout.hide(kind), cx);
        window.focus(&self.root, cx);
    }

    /// A kind that was off the board goes on it, in the first free cells.
    pub fn add_block(&mut self, kind: BlockKind, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        self.edit_layout(&project.id, |layout| layout.add(kind), cx);
    }

    // ---- a block ----------------------------------------------------------------------------

    fn block(&self, placed: &Placed, project: &Project, grid: Grid, look: &Look, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let kind = placed.kind;
        let metric = kind.metric_id().and_then(|id| self.reading(id));
        // A metric block of a recipe the project no longer keeps draws nothing, and the layout keeps it as it was saved.
        if kind.metric_id().is_some() && metric.is_none() {
            return div().into_any_element();
        }
        let hue = hue_of(kind);
        let slug = kind.slug();
        let title = self.title_of(kind);
        let ring = ring_colour(look);
        let handle = self.handles.iter().find(|(one, _)| *one == kind).map(|(_, handle)| handle.clone());
        let fill = self.fill(kind, project, cx);
        let views = self.views_of(kind);
        let shown_view = metric.map_or(placed.view, |reading| metric::drawn_in(placed.view, reading));
        let broken = metric.is_some_and(|reading| matches!(reading.state, MetricState::Broken { .. }));
        let switch = (views.len() > 1).then(|| {
            let mut group = div().id(SharedString::from(format!("{slug}-views"))).flex().items_center().gap(look.length(token::SPACE_1));
            for view in &views {
                let (view, on) = (*view, *view == shown_view);
                let project = project.id.clone();
                let button = div()
                    .id(SharedString::from(format!("{slug}-view-{}", view.word())))
                    .flex_none()
                    .h(look.length(token::H_BADGE))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_SM))
                    .text_size(look.length(token::TEXT_2XS))
                    .when(on, |button| button.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .when(!on, |button| button.text_color(look.color(token::FG_SUBTLE)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.bg(look.color(token::BG_HOVER)).border_color(ring))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .tooltip(tip(view.said()))
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.edit_layout(&project, |layout| layout.set_view(kind, view), cx)))
                    .child(view.word());
                group = group.child(labelled(button, Role::Button, format!("{} view of {}", view.word(), title)));
            }
            labelled(group, Role::Group, format!("View of {}", title))
        });
        let editing = self.edit;
        let hide = editing.then(|| {
            let side = look.length(token::H_BADGE);
            labelled(
                div()
                    .id(SharedString::from(format!("{slug}-hide")))
                    .flex_none()
                    .size(side)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .tooltip(tip(format!("Hide {} from the board", title)))
                    // A press on it is not the start of a drag of the block.
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                    })
                    .on_click(cx.listener(move |this, _, window, cx| this.hide_block(kind, window, cx)))
                    .child(glyph("eye-off", look.length(token::SPACE_4), look.color(token::FG_MUTED))),
                Role::Button,
                format!("Hide {}", title),
            )
        });
        let header = div()
            .id(SharedString::from(format!("{slug}-head")))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_ROW_ACTION))
            .px(look.length(token::SPACE_3))
            .bg(look.color(hue).opacity(0.18))
            .when(editing, |head| {
                head.cursor_grab()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, _| this.grab = None))
                    .on_drag(BlockDrag { kind, resize: false }, |_, _, _, cx| cx.new(|_| Ghost))
            })
            .child(glyph(mark_of(kind), look.length(token::SPACE_4), look.color(hue)))
            .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(title.clone())))
            .children(metric.map(|reading| self.metric_state(reading, look)))
            .child(div().flex_1())
            .children(switch)
            .children(hide);
        let header = if editing { labelled(header, Role::Group, format!("Move {}", title)).into_any_element() } else { header.into_any_element() };
        let body = match metric {
            Some(reading) => self.metric_body(reading, shown_view, look, cx),
            None => self.body(kind, placed.view, &fill, look, cx),
        };
        let corner = editing.then(|| {
            labelled(
                div()
                    .id(SharedString::from(format!("{slug}-corner")))
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .size(look.length(token::SPACE_4))
                    .rounded_tl(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_HOVER))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_STRONG))
                    .cursor(gpui::CursorStyle::ResizeUpLeftDownRight)
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, _| this.grab = None))
                    .on_drag(BlockDrag { kind, resize: true }, |_, _, _, cx| cx.new(|_| Ghost)),
                Role::Splitter,
                format!("Resize {}", title),
            )
        });
        let frame = div()
            .id(SharedString::from(format!("block-{slug}")))
            .absolute()
            .left(grid.left(placed.x))
            .top(grid.top(placed.y))
            .w(grid.wide(placed.w))
            .h(grid.tall(placed.h))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            // A metric that cannot read wears the failed hue on its whole frame, as a hairline all round.
            .border_color(if broken { look.color(token::STATUS_COMPLETED_FAILED) } else if editing { look.color(token::BORDER_STRONG) } else { look.color(token::BORDER_DEFAULT) })
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_CARD))
            .text_color(look.color(token::FG_DEFAULT))
            .when_some(handle.clone(), |frame, handle| frame.track_focus(&handle))
            .focus_visible(move |style| style.border_color(ring))
            .child(header)
            .child(body)
            .children(corner);
        let _ = window;
        labelled(frame, Role::Region, title).into_any_element()
    }

    /// What a block holds under its header: bones while it reads, why it could not, or its rows in the view it is in.
    fn body(&self, kind: BlockKind, view: View, fill: &Fill, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let slug = kind.slug();
        let pad = look.length(token::SPACE_2);
        let scroll = |content: AnyElement| div().id(SharedString::from(format!("{slug}-body"))).flex_1().min_h_0().overflow_y_scroll().p(pad).child(content).into_any_element();
        match fill {
            Fill::Reading => {
                let bones = (0..3).map(|at| bone(look, look.length(token::SPACE_12) * (3.0 - at as f32 * 0.5), look.length(token::SPACE_3)));
                scroll(labelled(div().id(SharedString::from(format!("{slug}-loading"))).flex().flex_col().gap(look.length(token::SPACE_2)).children(bones), Role::Image, "Loading").into_any_element())
            }
            Fill::Failed(why) => scroll(
                div()
                    .id(SharedString::from(format!("{slug}-failed")))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(glyph("triangle-alert", look.length(token::SPACE_4), look.color(token::DEGRADED)))
                    .child(SharedString::from(why.clone()))
                    .into_any_element(),
            ),
            Fill::Ready(shown) => {
                let content = self.rows(kind, view, shown, look, cx);
                let tail = (kind == BlockKind::Run).then(|| self.tail_pane(look, cx)).flatten();
                div().flex_1().min_h_0().flex().flex_col().child(scroll(content)).children(tail).into_any_element()
            }
        }
    }

    pub(super) fn rows(&self, kind: BlockKind, view: View, shown: &Shown, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let columns = model::button_columns(kind);
        if view == View::Picture && !shown.tiles.is_empty() {
            return self.picture(&shown.tiles, look);
        }
        // The rows stand in groups, a heading over each row that carries one and the rows after it.
        let mut groups: Vec<(Option<&str>, Vec<&Item>)> = Vec::new();
        for item in &shown.items {
            match (&item.section, groups.last_mut()) {
                (None, Some(group)) => group.1.push(item),
                (heading, _) => groups.push((heading.as_deref(), vec![item])),
            }
        }
        let slug = kind.slug();
        let drawn: Vec<AnyElement> = groups
            .into_iter()
            .enumerate()
            .map(|(at, (heading, items))| {
                let body = match view {
                    View::Cards => div().grid().grid_cols(2).gap(look.length(token::SPACE_2)).children(items.iter().map(|item| self.card(item, look, cx))).into_any_element(),
                    View::Table => div().flex().flex_col().children(items.iter().enumerate().map(|(at, item)| self.line(item, true, at > 0, columns, look, cx))).into_any_element(),
                    _ => div().flex().flex_col().children(items.iter().map(|item| self.line(item, false, false, columns, look, cx))).into_any_element(),
                };
                let heading = heading.map(|words| {
                    labelled(
                        div()
                            .id(SharedString::from(format!("{slug}-section-{at}")))
                            .text_size(look.length(token::TEXT_2XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child(SharedString::from(words.to_string())),
                        Role::Heading,
                        words.to_string(),
                    )
                });
                div().flex().flex_col().gap(look.length(token::SPACE_1)).children(heading).child(body).into_any_element()
            })
            .collect();
        div().flex().flex_col().gap(look.length(token::SPACE_2)).children(drawn).into_any_element()
    }

    /// A row's state glyph, its hue, with the words of the state in its tooltip.
    fn mark(&self, item: &Item, look: &Look, cx: &Context<Self>) -> AnyElement {
        let size = look.length(token::SPACE_4);
        let ink = look.color(item.hue);
        let inner = if item.turning { spinner(SharedString::from(format!("{}-spin", item.key)), size, ink, cx).into_any_element() } else { glyph(item.glyph, size, ink).into_any_element() };
        labelled(div().id(SharedString::from(format!("{}-mark", item.key))).flex_none().flex().items_center().justify_center().size(look.length(token::SPACE_6)).tooltip(tip(item.said.clone())).child(inner), Role::Image, item.said.clone()).into_any_element()
    }

    /// One button of a row, or the empty place where a row has none, so the buttons stand in the same columns down the block.
    fn button(&self, item: &Item, at: usize, button: Option<&Button>, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let side = look.length(token::H_CONTROL_SM);
        let Some(button) = button else { return div().flex_none().size(side).into_any_element() };
        let ring = ring_colour(look);
        let pending = Board::held(&button.press).is_some_and(|held| self.acting.contains(&held));
        let press = button.press.clone();
        let ink = look.color(token::FG_MUTED);
        let element = div()
            .id(SharedString::from(format!("{}-button-{at}", item.key)))
            .flex_none()
            .size(side)
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .focusable()
            .tab_stop(!pending)
            .focus_visible(move |style| style.border_color(ring))
            .tooltip(tip(button.name.clone()))
            .when(!pending, |element| element.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, _, cx| this.press(press.clone(), cx))))
            .child(if pending { spinner(SharedString::from(format!("{}-pending-{at}", item.key)), look.length(token::SPACE_4), ink, cx).into_any_element() } else { glyph(button.glyph, look.length(token::SPACE_4), ink).into_any_element() });
        labelled(element, Role::Button, button.name.clone()).into_any_element()
    }

    /// A row as a line of a List, or of a Table, where it stands in columns and is parted from the one above by a hairline.
    fn line(&self, item: &Item, table: bool, parted: bool, columns: usize, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let buttons: Vec<AnyElement> = (0..columns).map(|at| self.button(item, at, item.buttons.get(at).and_then(Option::as_ref), look, cx)).collect();
        let meta_w = look.length(token::SPACE_12) * 2.0;
        let chip_w = look.length(token::SPACE_12) * 2.5;
        let row = div()
            .id(SharedString::from(item.key.clone()))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_ROW_ACTION))
            .px(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_XS))
            .when(table && parted, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).rounded_none())
            .child(self.mark(item, look, cx))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(item.title.clone())))
            .when(table, |row| row.child(div().flex_none().w(chip_w).flex().children(item.tag.as_ref().map(|(words, hue)| chip(look, *hue, words.clone())))))
            .children(item.meta.as_ref().map(|meta| {
                div().flex_none().when(table, |meta| meta.w(meta_w)).max_w(meta_w).truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(meta.clone()))
            }))
            .children(buttons);
        match &item.press {
            Some(press) => {
                let press = press.clone();
                let row = row
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.bg(look.color(token::BG_HOVER)).border_color(ring))
                    .on_click(cx.listener(move |this, _, _, cx| this.press(press.clone(), cx)));
                labelled(row, Role::Button, item.name()).into_any_element()
            }
            None => labelled(row, Role::ListItem, item.name()).into_any_element(),
        }
    }

    /// A row as a card: its title and its chip, what it says under them, and its buttons.
    fn card(&self, item: &Item, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let buttons: Vec<AnyElement> = item.buttons.iter().enumerate().map(|(at, button)| self.button(item, at, button.as_ref(), look, cx)).collect();
        let card = div()
            .id(SharedString::from(item.key.clone()))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .min_w_0()
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            .text_size(look.length(token::TEXT_XS))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(self.mark(item, look, cx))
                    .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(item.title.clone()))),
            )
            .children(item.meta.as_ref().map(|meta| div().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(meta.clone()))))
            .children(item.tag.as_ref().map(|(words, hue)| div().flex().child(chip(look, *hue, words.clone()))))
            .when(!buttons.is_empty(), |card| card.child(div().flex().items_center().gap(look.length(token::SPACE_1)).children(buttons)));
        match &item.press {
            Some(press) => {
                let press = press.clone();
                let card = card
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .on_click(cx.listener(move |this, _, _, cx| this.press(press.clone(), cx)));
                labelled(card, Role::Button, item.name()).into_any_element()
            }
            None => labelled(card, Role::ListItem, item.name()).into_any_element(),
        }
    }

    /// The code map's rooms as tiles, each in the hue of the area that owns most of it.
    fn picture(&self, tiles: &[Tile], look: &Look) -> AnyElement {
        div()
            .id("features-picture")
            .grid()
            .grid_cols(4)
            .gap(look.length(token::SPACE_2))
            .children(tiles.iter().map(|tile| {
                let said = format!("{}, {}", tile.module, tile.kind);
                labelled(
                    div()
                        .id(SharedString::from(format!("tile-{}", tile.module)))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .h(look.length(token::H_ROW_ACTION) * 1.5)
                        .p(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(tile.hue).opacity(0.6))
                        .bg(look.color(tile.hue).opacity(0.16))
                        .tooltip(tip(said.clone()))
                        .text_size(look.length(token::TEXT_2XS))
                        .child(div().truncate().child(SharedString::from(tile.module.clone()))),
                    Role::Image,
                    said,
                )
            }))
            .into_any_element()
    }

    /// The tail of a server's output, under the Run block's rows, with Refresh and Close.
    fn tail_pane(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tail = self.tail.as_ref()?;
        let ring = ring_colour(look);
        let tool = |id: &'static str, mark: &'static str, name: String, cx: &mut Context<Self>, press: fn(&mut Board, &mut Context<Board>)| {
            let side = look.length(token::H_CONTROL_SM);
            labelled(
                div()
                    .id(id)
                    .flex_none()
                    .size(side)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .tooltip(tip(name.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| press(this, cx)))
                    .child(glyph(mark, look.length(token::SPACE_4), look.color(token::FG_MUTED))),
                Role::Button,
                name,
            )
        };
        let lines: AnyElement = match &tail.lines {
            None => div().flex().flex_col().gap(look.length(token::SPACE_1)).children((0..3).map(|at| bone(look, look.length(token::SPACE_12) * (3.0 - at as f32 * 0.5), look.length(token::SPACE_3)))).into_any_element(),
            Some(Err(why)) => div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(why.clone())).into_any_element(),
            Some(Ok(lines)) => div().flex().flex_col().children(lines.iter().map(|line| div().whitespace_nowrap().child(SharedString::from(line.clone())))).into_any_element(),
        };
        Some(
            labelled(
                div()
                    .id("run-tail")
                    .flex_none()
                    .max_h(look.length(token::H_ROW_UNIT) * 3.0)
                    .flex()
                    .flex_col()
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .pl(look.length(token::SPACE_3))
                            .pr(look.length(token::SPACE_1))
                            .child(glyph("scroll-text", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(tail.title.clone())))
                            .child(tool("run-tail-refresh", "refresh-cw", format!("Read the logs of {} again", tail.title), cx, |this, cx| this.refresh_tail(cx)))
                            .child(tool("run-tail-close", "x", format!("Close the logs of {}", tail.title), cx, |this, cx| this.close_tail(cx))),
                    )
                    .child(
                        div()
                            .id("run-tail-lines")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px(look.length(token::SPACE_3))
                            .pb(look.length(token::SPACE_2))
                            .font_family(crate::titlebar::parts::mono(look))
                            .text_size(look.length(token::TEXT_2XS))
                            .child(lines),
                    ),
                Role::Log,
                format!("Logs of {}", tail.title),
            )
            .into_any_element(),
        )
    }
}

impl Board {
    /// A block's place while Fleet has not said where the board stands: the card, with bones where its rows will be.
    fn skeleton(&self, placed: &Placed, grid: Grid, look: &Look) -> AnyElement {
        let bones = (0..3).map(|at| bone(look, look.length(token::SPACE_12) * (3.0 - at as f32 * 0.5), look.length(token::SPACE_3)));
        div()
            .id(SharedString::from(format!("block-loading-{}", placed.kind.slug())))
            .absolute()
            .left(grid.left(placed.x))
            .top(grid.top(placed.y))
            .w(grid.wide(placed.w))
            .h(grid.tall(placed.h))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_CARD))
            .children(bones)
            .into_any_element()
    }

    /// The question over the board before a flagged Command runs: its name, then Run and Cancel, the keys on Cancel.
    fn confirm_card(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let asking = self.asking.as_ref()?;
        let said = SharedString::from(format!("Run {}?", asking.name));
        let (run, stay) = (self.run_stop.clone(), self.cancel_stop.clone());
        let run_button = labelled(
            Btn::new("dashboard-run", "Run", Weight::Primary)
                .draw(look, cx)
                .track_focus(&run)
                .tooltip(tip("Run it in the main checkout"))
                .on_click(cx.listener(|this, _, window, cx| this.confirm_run(window, cx))),
            Role::Button,
            "Run",
        );
        let cancel_button = labelled(
            Btn::new("dashboard-run-cancel", "Cancel", Weight::Secondary)
                .draw(look, cx)
                .track_focus(&stay)
                .tooltip(tip("Leave it"))
                .on_click(cx.listener(|this, _, window, cx| this.cancel_run(window, cx))),
            Role::Button,
            "Cancel",
        );
        let body = card(look)
            .id("dashboard-confirm")
            .occlude()
            .w_full()
            .max_w(look.length(token::W_DIALOG))
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::PAD_CARD))
            // A press inside the card is not a press on the scrim.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).child(said.clone()))
            .children(asking.repository.clone().map(|name| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(name))))
            .child(div().flex().gap(look.length(token::SPACE_2)).child(run_button).child(cancel_button));
        Some(
            div()
                .id("dashboard-confirm-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(look.length(token::SPACE_6))
                .bg(look.color(token::SCRIM))
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                    this.cancel_run(window, cx);
                    cx.stop_propagation();
                }))
                // Tab and the arrows go between the two buttons, Esc leaves; Enter and Space press the one the keys are on.
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" => this.cancel_run(window, cx),
                        "tab" | "left" | "right" | "up" | "down" => {
                            let to = if stay.is_focused(window) { &run } else { &stay };
                            window.focus(to, cx);
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .child(labelled(body, Role::Dialog, said))
                .into_any_element(),
        )
    }
}

impl Render for Board {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let root = div().id("dashboard").relative().key_context(ActionScope::ProjectBoard.key_context().unwrap_or_default()).track_focus(&self.root).on_action(cx.listener(Self::on_act)).size_full().font_family(sans(&look));
        let Some(project) = self.project(cx) else { return root.into_any_element() };
        let ready = self.layout_ready(&project.id);
        let layout = self.layout(&project.id).clone();
        let gap = look.length(token::SPACE_3);
        let width = window.viewport_size().width - gap * 2.0;
        let columns = f32::from(COLUMNS);
        let grid = Grid { col: (width - gap * (columns - 1.0)) / columns, row: look.length(token::H_ROW_UNIT) * 3.5, gap };
        let rows = f32::from(layout.rows());
        let height = grid.row * rows + gap * (rows - 1.0).max(0.0);
        let pitch = (grid.col + gap, grid.row + gap);
        let blocks: Vec<AnyElement> = if ready {
            layout.blocks.iter().map(|placed| self.block(placed, &project, grid, &look, window, cx)).collect()
        } else {
            layout.blocks.iter().map(|placed| self.skeleton(placed, grid, &look)).collect()
        };
        let board = div()
            .id("dashboard-grid")
            .relative()
            .w_full()
            .h(height.max(px(0.)))
            .on_drag_move(cx.listener(move |this, event: &DragMoveEvent<BlockDrag>, _, cx| this.dragged(event, pitch, cx)))
            .children(blocks);
        root.child(labelled(div().id("dashboard-scroll").size_full().overflow_y_scroll().pb(gap).child(board), Role::Group, "Board")).children(self.confirm_card(&look, cx)).into_any_element()
    }
}
