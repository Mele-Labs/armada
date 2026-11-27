//! The Dashboard tab drawn: a six-column grid of blocks, each a card with a tinted header band, a view switch and its rows.
//! A block's rows are drawn as a List, a Table or Cards from one `Item` (`model.rs`), so a block's own code is the function that
//! makes its items. While the board is edited a block's header is a handle that moves it and its corner resizes it; the same
//! moves are keys (`dashboard_move`, `dashboard_resize`).

use gpui::{div, prelude::*, AnyElement, Context, DragMoveEvent, FontWeight, MouseButton, Role, SharedString, Window};
use ipc::MetricState;

use super::board::{Board, Ranking};
use super::layout::{BlockKind, Placed, View};
use super::metric;
use super::model::{self, Button, Fill, Item, Project, Shown, Tile};
use crate::generated::actions::ActionScope;
use crate::grid::chrome::{self, Cells as Grid, Pressed};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::{bone, ring_colour};
use crate::titlebar::parts::{card, glyph, mono, sans, spinner, tip, Btn, Weight};

/// A block being moved or resized, as the drag carries it.
type BlockDrag = chrome::BlockDrag<BlockKind>;

/// The hue a block's header band and glyph wear: what kind of block it is.
fn hue_of(kind: BlockKind) -> Token {
    match kind {
        BlockKind::System => token::ACCENT,
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
        BlockKind::System => "network",
        BlockKind::Needs => "bell",
        BlockKind::Run => "play",
        BlockKind::Reviews => "git-pull-request",
        BlockKind::Issues => "ticket",
        BlockKind::Features => "library",
        BlockKind::Checks => "list-checks",
        BlockKind::Metric(_) => "gauge",
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
        let (dx, dy) = chrome::arrow(act.slot);
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
    fn dragged(&mut self, event: &DragMoveEvent<BlockDrag>, pitch: (gpui::Pixels, gpui::Pixels), cx: &mut Context<Self>) {
        let drag = event.drag(cx).clone();
        let Some(project) = self.project(cx) else { return };
        let at = event.event.position - event.bounds.origin;
        let Some(now) = self.layout(&project.id).block(drag.kind).copied() else { return };
        let step = chrome::drag_step(&now, drag.resize, at, pitch, &mut self.grab);
        self.edit_layout(&project.id, |layout| step.apply(layout, drag.kind), cx);
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
        let handle = self.handles.iter().find(|(one, _)| *one == kind).map(|(_, handle)| handle.clone());
        let fill = self.fill(kind, project, cx);
        let views = self.views_of(kind);
        let shown_view = metric.map_or(placed.view, |reading| metric::drawn_in(placed.view, reading));
        let broken = metric.is_some_and(|reading| matches!(reading.state, MetricState::Broken { .. }));
        let switch = (views.len() > 1).then(|| {
            chrome::view_switch(&slug, &title, &views, shown_view, look, |view| -> Pressed {
                let project = project.id.clone();
                Box::new(cx.listener(move |this, _, _, cx| this.edit_layout(&project, |layout| layout.set_view(kind, view), cx)))
            })
        });
        let editing = self.edit;
        let hide = editing.then(|| chrome::hide_button(&slug, &title, look, cx.listener(move |this, _, window, cx| this.hide_block(kind, window, cx))));
        let rank = (kind == BlockKind::Issues).then(|| self.rank_controls(look, cx));
        let header = chrome::head(&slug, hue, look)
            .when(editing, |head| chrome::lift(head, kind, cx.listener(|this, _, _, _| this.grab = None)))
            .child(chrome::mark(mark_of(kind), hue, look))
            .child(chrome::title_text(&title, look))
            .children(metric.map(|reading| self.metric_state(reading, look)))
            .child(div().flex_1())
            .children(rank)
            .children(switch)
            .children(hide);
        let header = if editing { labelled(header, Role::Group, format!("Move {}", title)).into_any_element() } else { header.into_any_element() };
        let body = if kind == BlockKind::System {
            self.system.update(cx, |system, _| system.set_view(placed.view));
            div().flex_1().min_h_0().relative().child(div().absolute().size_full().child(self.system.clone())).into_any_element()
        } else {
            match metric {
                Some(reading) => self.metric_body(reading, shown_view, look, cx),
                None => self.body(kind, placed.view, &fill, look, cx),
            }
        };
        // The System block grows over the board, and its frame is then set by the board's size and not by its cells.
        let overlay = (kind == BlockKind::System)
            .then(|| {
                let home = super::system::PoseRect { x: f32::from(grid.left(placed.x)), y: f32::from(grid.top(placed.y)), w: f32::from(grid.wide(placed.w)), h: f32::from(grid.tall(placed.h)) };
                let seen = window.viewport_size();
                self.system.read(cx).overlay(home, (f32::from(seen.width), f32::from(seen.height)))
            })
            .flatten();
        let over = overlay.is_some();
        let corner = editing.then(|| chrome::corner(&slug, &title, kind, look, cx.listener(|this, _, _, _| this.grab = None)));
        // A metric that cannot read wears the failed hue on its whole frame, as a hairline all round.
        let edge = if broken { look.color(token::STATUS_COMPLETED_FAILED) } else if editing { look.color(token::BORDER_STRONG) } else { look.color(token::BORDER_DEFAULT) };
        let frame = chrome::frame(&slug, look, edge)
            .when(!over, |frame| grid.put(frame, placed))
            .when(over, |frame| frame.size_full())
            .when_some(handle.clone(), |frame, handle| frame.track_focus(&handle))
            .child(header)
            .child(body)
            .children(corner);
        let framed = labelled(frame, Role::Region, title).into_any_element();
        if kind == BlockKind::System {
            super::system::posed(framed, overlay, look)
        } else {
            framed
        }
    }

    /// A block's body, scrolled on its own. **While it has rows out of sight it keeps the wheel**, so the board behind it
    /// stands still; a body with nothing to scroll lets the wheel through to the board.
    pub(super) fn body_scroll(&self, id: SharedString) -> gpui::Stateful<gpui::Div> {
        let handle = self.bodies.borrow_mut().entry(id.to_string()).or_default().clone();
        chrome::scrolled(id, &handle)
    }

    /// Rank with the librarian in the Issues block's head: a press has the librarian order the lists, and once they are in its
    /// order a Ranked chip in the ask's hue stands in its place, with Reset beside it. A refusal is a glyph with Fleet's line.
    fn rank_controls(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let purple = look.color(token::ASK_FEATURES);
        let ranked = self.ranked_by_librarian();
        let waiting = self.ranking == Ranking::Waiting;
        let size = look.length(token::SPACE_3);
        let shape = |id: &'static str| {
            div()
                .id(id)
                .flex_none()
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_2))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .text_size(look.length(token::TEXT_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .whitespace_nowrap()
        };
        let compact = |id: &'static str| {
            shape(id)
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                // A press on it is not the start of a drag of the block.
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    cx.stop_propagation();
                    window.prevent_default();
                })
        };
        // Ranked is a state, not a press: a chip in the ask's hue that says when, with Reset the one button beside it.
        let state = match self.ranked_at() {
            Some(at) => {
                let said = format!("Ranked by the librarian at {}", crate::settings::time::stamp_at(at.as_str()).unwrap_or_else(|| at.as_str().to_string()));
                labelled(
                    shape("issues-ranked").border_color(purple).bg(purple.opacity(0.16)).text_color(purple).tooltip(tip(said)).child(glyph("sparkles", size, purple)).child("Ranked"),
                    Role::Status,
                    "Ranked",
                )
            }
            None => {
                let mark = if waiting { spinner("issues-rank-spin", size, purple, cx).into_any_element() } else { glyph("sparkles", size, purple).into_any_element() };
                labelled(
                    compact("issues-rank")
                        .border_color(gpui::transparent_black())
                        .text_color(look.color(token::FG_MUTED))
                        .when(!waiting, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(|this, _, _, cx| this.rank(false, cx))))
                        .tooltip(tip("The librarian orders these and says why under each"))
                        .child(mark)
                        .child("Rank with the librarian"),
                    Role::Button,
                    "Rank with the librarian",
                )
            }
        };
        let reset = ranked.then(|| {
            let mark = if waiting { spinner("issues-reset-spin", size, look.color(token::FG_MUTED), cx).into_any_element() } else { glyph("rotate-ccw", size, look.color(token::FG_MUTED)).into_any_element() };
            labelled(
                compact("issues-rank-reset")
                    .border_color(gpui::transparent_black())
                    .text_color(look.color(token::FG_MUTED))
                    .when(!waiting, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(|this, _, _, cx| this.rank(true, cx))))
                    .tooltip(tip("Back to the order by features"))
                    .child(mark)
                    .child("Reset"),
                Role::Button,
                "Reset",
            )
        });
        let refused = match &self.ranking {
            Ranking::Refused(why) => Some(labelled(
                div().id("issues-rank-refused").flex_none().tooltip(tip(why.clone())).child(glyph("triangle-alert", look.length(token::SPACE_4), look.color(token::DEGRADED))),
                Role::Image,
                why.clone(),
            )),
            _ => None,
        };
        div().flex_none().flex().items_center().gap(look.length(token::SPACE_1)).children(refused).child(state).children(reset).into_any_element()
    }

    /// The librarian's reason for a row's place, small, under its title.
    fn why_line(&self, item: &Item, look: &Look) -> Option<AnyElement> {
        let why = item.why.as_ref()?;
        Some(
            labelled(
                div().id(SharedString::from(format!("{}-why", item.key))).min_w_0().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::ASK_FEATURES).opacity(0.8)).child(SharedString::from(why.clone())),
                Role::Label,
                why.clone(),
            )
            .into_any_element(),
        )
    }

    /// What a block holds under its header: bones while it reads, why it could not, or its rows in the view it is in.
    fn body(&self, kind: BlockKind, view: View, fill: &Fill, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let slug = kind.slug();
        let pad = look.length(token::SPACE_2);
        let scroll = |content: AnyElement| self.body_scroll(SharedString::from(format!("{slug}-body"))).flex_1().min_h_0().p(pad).child(content).into_any_element();
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

    /// The second line of a two-line row: its reference in mono, then a chip for each thing it touches.
    fn under_line(&self, under: &model::Under, look: &Look) -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(under.reference.clone())))
            .children(under.chips.iter().map(|words| chip(look, token::SKETCH_BLUE, words.clone())))
            .into_any_element()
    }

    /// A row as a line of a List, or of a Table, where it stands in columns and is parted from the one above by a hairline.
    fn line(&self, item: &Item, table: bool, parted: bool, columns: usize, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let two = item.under.is_some() || item.why.is_some();
        let buttons: Vec<AnyElement> = (0..columns).map(|at| self.button(item, at, item.buttons.get(at).and_then(Option::as_ref), look, cx)).collect();
        let meta_w = look.length(token::SPACE_12) * 2.0;
        let chip_w = look.length(token::SPACE_12) * 2.5;
        let row = div()
            .id(SharedString::from(item.key.clone()))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .when(!two, |row| row.h(look.length(token::H_ROW_ACTION)))
            .when(two, |row| row.items_start().py(look.length(token::SPACE_2)))
            .px(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_XS))
            .when(table && parted, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).rounded_none())
            .child(self.mark(item, look, cx))
            .child(match two {
                true => div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(match item.under {
                        Some(_) => div().line_clamp(2).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(item.title.clone())),
                        None => div().min_w_0().truncate().child(SharedString::from(item.title.clone())),
                    })
                    .children(self.why_line(item, look))
                    .children(item.under.as_ref().map(|under| self.under_line(under, look)))
                    .into_any_element(),
                false => div().flex_1().min_w_0().truncate().child(SharedString::from(item.title.clone())).into_any_element(),
            })
            .when(table && item.under.is_none(), |row| row.child(div().flex_none().w(chip_w).flex().children(item.tag.as_ref().map(|(words, hue)| chip(look, *hue, words.clone())))))
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
            .children(self.why_line(item, look))
            .children(item.meta.as_ref().map(|meta| div().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(meta.clone()))))
            .children(item.tag.as_ref().map(|(words, hue)| div().flex().child(chip(look, *hue, words.clone()))))
            .children(item.under.as_ref().map(|under| self.under_line(under, look)))
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
        let grid = Grid::across(width, gap, look.length(token::H_ROW_UNIT) * 3.5);
        let height = grid.height(layout.rows());
        let pitch = grid.pitch();
        // The System block, grown, is drawn over the board and not in its cells.
        let grown = self.system.read(cx).over_board();
        let in_grid = |placed: &&Placed| !(grown && placed.kind == BlockKind::System);
        let over: Option<AnyElement> = layout.block(BlockKind::System).filter(|_| ready && grown).map(|placed| self.block(placed, &project, grid, &look, window, cx));
        let blocks: Vec<AnyElement> = if ready {
            layout.blocks.iter().filter(in_grid).map(|placed| self.block(placed, &project, grid, &look, window, cx)).collect()
        } else {
            layout.blocks.iter().map(|placed| chrome::skeleton(placed, grid, &look)).collect()
        };
        let board = div()
            .id("dashboard-grid")
            .relative()
            .w_full()
            .h(height)
            .on_drag_move(cx.listener(move |this, event: &DragMoveEvent<BlockDrag>, _, cx| this.dragged(event, pitch, cx)))
            .child(self.system.read(cx).measure_grid())
            .children(blocks);
        root.child(self.system.read(cx).measure_root())
            .child(labelled(div().id("dashboard-scroll").size_full().overflow_y_scroll().pb(gap).child(board), Role::Group, "Board"))
            .children(over)
            .children(self.confirm_card(&look, cx))
            .into_any_element()
    }
}
