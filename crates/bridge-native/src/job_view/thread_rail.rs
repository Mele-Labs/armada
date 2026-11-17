//! The contents rail on a Job's thread: thin bars down its right edge, one for each ask said in the thread, in the order their
//! first words come, coloured by kind. The ask in view is drawn longer and fully lit. Hovering the bars opens a list beside
//! them that names each ask with its kind and where it stands; pressing a bar or a row scrolls the thread to where the ask
//! started. A thread with no asks draws neither.
//!
//! The asks are the kind chip's store's, read off the main thread when the Job came on screen; the rail adds no reader. The ask
//! in view is worked out in `model` from the list's scroll top, and the rail draws again where the thread does, as the call
//! chip does, since a scroll redraws the thread.
//!
//! **Keys.** `thread_rail` (`⌥T`, scope `thread`) opens the list and puts the keys on it: Up and Down move, Enter jumps to the
//! ask, Esc puts the list away and gives the keys back to the thread. Those four are the list's own key handler, as a menu's
//! are, and not acts. `⌥↑` and `⌥↓` are another act's and are left to it.
//!
//! **Where it hooks in.** The state and the act's handler are made the first time the rail draws, since the thread's start-up
//! has no line for it.

use gpui::{div, point, prelude::*, px, AnyElement, App, BoxShadow, ClickEvent, Context, FocusHandle, Global, Hsla, KeyDownEvent, ListOffset, Pixels, Role, ScrollHandle, SharedString, Window};

use super::kind_chip::model::Hue;
use super::kind_chip::store;
use super::JobView;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use model::Mark;

pub(crate) mod model;
#[allow(dead_code)]
pub(crate) mod shots;

#[cfg(test)]
mod tests;

/// The rail's measures, from the approved mock (`rail.css`), where no token holds them.
const BAR_H: f32 = 3.;
const BAR_W: f32 = 16.;
const BAR_NOW_W: f32 = 26.;
const BAR_RADIUS: f32 = 2.;
const BAR_GAP: f32 = 11.;
/// How lit a bar is while its ask is not the one in view.
const BAR_REST: f32 = 0.55;
const RAIL_SIDE: f32 = 6.;
const RAIL_EDGE: f32 = 14.;
const LIST_W: f32 = 300.;
const ROW_Y: f32 = 6.;
/// The dot's drop, which sets it level with the first line of the title.
const DOT_DROP: f32 = 6.;
const MARK_EDGE: f32 = 2.;
/// What the rail takes of the thread's width: its edge, a side, the longest bar, a side.
const RAIL_WIDTH: f32 = RAIL_EDGE + RAIL_SIDE + BAR_NOW_W + RAIL_SIDE;

/// What the rail remembers: whether the pointer is on it, where the keys are in the list, and the list's own focus.
struct Rail {
    job: Option<SharedString>,
    hover: bool,
    /// The row the keys are on, while they hold the list.
    cursor: Option<usize>,
    focus: FocusHandle,
    scroll: ScrollHandle,
}

impl Global for Rail {}

impl Rail {
    fn reset(&mut self) {
        self.hover = false;
        self.cursor = None;
    }

    fn open(&self) -> bool {
        self.hover || self.cursor.is_some()
    }
}

/// Make the state, and hear the key that opens the list. Once, however often it is called.
pub(crate) fn init(cx: &mut App) {
    if cx.has_global::<Rail>() {
        return;
    }
    let focus = cx.focus_handle();
    cx.set_global(Rail { job: None, hover: false, cursor: None, focus, scroll: ScrollHandle::new() });
    cx.on_action(|act: &Act, cx: &mut App| {
        if !act.is("thread_rail") {
            return cx.propagate();
        }
        let (Some(view), Some(window)) = (JobView::live(cx), cx.active_window()) else { return cx.propagate() };
        // The key is being dispatched inside this window's own update, so the list opens just after.
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| view.update(cx, |view, cx| view.rail_key(window, cx)));
        });
    });
}

fn ink(look: &Look, hue: Hue) -> Hsla {
    match hue {
        Hue::Bug => look.color(token::ERROR),
        Hue::Change => look.color(token::ACCENT),
        Hue::Idea => look.color(token::STATUS_AWAITING_REVIEW),
        Hue::Neutral => look.color(token::FG_SUBTLE),
    }
}

/// A shadow of no blur: a hairline round a box, or an edge inside one.
fn edge(color: Hsla, x: Pixels, spread: Pixels, inset: bool) -> BoxShadow {
    BoxShadow { color, offset: point(x, px(0.)), blur_radius: px(0.), spread_radius: spread, inset }
}

impl JobView {
    /// The asks this Job's thread holds in its order, and which of them is in view, as an index into the first.
    fn rail_marks(&self, id: &SharedString, cx: &App) -> (Vec<Mark>, Option<usize>) {
        let (Some(page), Some(held)) = (self.pages.get(id).filter(|page| page.version.is_some()), self.thread(id, cx)) else { return (Vec::new(), None) };
        let marks = model::marks(&store::asks(id, cx), &held.items);
        let entries: Vec<usize> = marks.iter().map(|mark| page.entry(mark.item)).collect();
        let now = model::in_view(&entries, page.list.logical_scroll_top().item_ix);
        (marks, now)
    }

    /// The room the rail takes off the thread's right, while one is drawn: the bars' column at its widest and the gap to the edge.
    /// None where the thread holds no ask, the same test that leaves the rail out. The thread pads by it so no card sits under the bars.
    pub(super) fn rail_width(&self, id: &SharedString, cx: &App) -> Option<Pixels> {
        (!self.rail_marks(id, cx).0.is_empty()).then(|| px(RAIL_WIDTH))
    }

    /// The rail over the thread, with its list while the pointer is on it or the keys hold it. None where the thread holds no ask.
    pub(super) fn render_thread_rail(&mut self, id: &SharedString, cx: &mut Context<Self>) -> Option<AnyElement> {
        // Set up at start-up by `story::init`; a view drawn without it (a test window) makes the state here, once.
        init(cx);
        let (marks, now) = self.rail_marks(id, cx);
        let rail = cx.global_mut::<Rail>();
        if rail.job.as_ref() != Some(id) {
            rail.reset();
            rail.job = Some(id.clone());
        }
        if marks.is_empty() {
            rail.reset();
            return None;
        }
        rail.cursor = rail.cursor.map(|at| at.min(marks.len() - 1));
        let (open, cursor, focus, scroll) = (rail.open(), rail.cursor, rail.focus.clone(), rail.scroll.clone());
        let look = cx.look();
        let job = id.clone();

        let bars = marks.iter().enumerate().map(|(at, mark)| {
            let here = now == Some(at);
            let lit = ink(look, model::hue(mark.kind));
            let bar = div()
                .h(px(BAR_H))
                .w(px(if here { BAR_NOW_W } else { BAR_W }))
                .rounded(px(BAR_RADIUS))
                .bg(if here { lit } else { lit.opacity(BAR_REST) })
                .when(here, |bar| bar.shadow(vec![edge(look.color(token::BORDER_HIGHLIGHT), px(0.), look.length(token::BORDER_WIDTH), false)]));
            let (job, ask) = (job.clone(), mark.ask_id.clone());
            // The press covers the gap round the bar, which a bar of three pixels alone would be too thin to hit.
            labelled(
                div()
                    .id(SharedString::from(format!("rail-bar:{}", mark.ask_id)))
                    .w_full()
                    .h(px(BAR_H + BAR_GAP))
                    .flex()
                    .items_center()
                    .justify_end()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.rail_jump(&job, &ask, window, cx)))
                    .child(bar),
                Role::Button,
                model::bar_name(mark),
            )
        });
        let column = labelled(
            div()
                .id("thread-rail")
                .mt(look.length(token::SPACE_12) + look.length(token::SPACE_2))
                .py(look.length(token::SPACE_2))
                .px(px(RAIL_SIDE))
                .flex()
                .flex_col()
                .items_end()
                .children(bars),
            Role::Region,
            "Asks in this thread",
        );

        let list = open.then(|| self.rail_list(&marks, now, cursor, &focus, &scroll, cx));
        Some(
            div()
                .id("thread-rail-over")
                .absolute()
                .top(look.length(token::SPACE_4))
                .right(px(RAIL_EDGE))
                .flex()
                .flex_row()
                .items_start()
                // The press is the rail's alone, and a wheel turned over it still moves the thread.
                .block_mouse_except_scroll()
                .on_hover(cx.listener(|_, hovered: &bool, _, cx| {
                    let rail = cx.global_mut::<Rail>();
                    if rail.hover != *hovered {
                        rail.hover = *hovered;
                        cx.notify();
                    }
                }))
                .children(list)
                .child(column)
                .into_any_element(),
        )
    }

    /// The list of asks, a card beside the bars: a heading and a row for each ask, the one in view marked.
    fn rail_list(&self, marks: &[Mark], now: Option<usize>, cursor: Option<usize>, focus: &FocusHandle, scroll: &ScrollHandle, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let job = self.shown.clone().unwrap_or_default();
        let rows = marks.iter().enumerate().map(|(at, mark)| {
            let (here, held) = (now == Some(at), cursor == Some(at));
            let (job, ask) = (job.clone(), mark.ask_id.clone());
            let mut marked = Vec::new();
            if here {
                marked.push(edge(look.color(token::ACCENT), px(MARK_EDGE), px(0.), true));
            }
            if held {
                marked.push(edge(look.color(token::FOCUS_RING), px(0.), look.length(token::BORDER_WIDTH), true));
            }
            labelled(
                div()
                    .id(SharedString::from(format!("rail-row:{}", mark.ask_id)))
                    .flex()
                    .flex_none()
                    .items_start()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_4))
                    .py(px(ROW_Y))
                    .cursor_pointer()
                    .when(here || held, |row| row.bg(look.color(token::BG_HOVER)).shadow(marked))
                    .hover(|row| row.bg(look.color(token::BG_HOVER)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.rail_jump(&job, &ask, window, cx)))
                    .child(div().flex_none().mt(px(DOT_DROP)).size(look.length(token::SPACE_2)).rounded_full().bg(ink(look, model::hue(mark.kind))))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(look.length(token::TEXT_SM))
                                    .line_height(look.length(token::LEADING_SM))
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .child(SharedString::from(mark.title.clone())),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(look.length(token::TEXT_XS))
                                    .line_height(look.length(token::LEADING_XS))
                                    .text_color(look.color(token::FG_SUBTLE))
                                    .child(SharedString::from(model::meta(mark))),
                            ),
                    ),
                Role::ListBoxOption,
                model::row_name(mark),
            )
        });
        // Kept inside the thread's pane under the rail's own top, so a long list scrolls and never runs off the window.
        let room = self.pane_height.map(|height| height - look.length(token::SPACE_4) * 2.);
        labelled(
            div()
                .id("thread-rail-list")
                .track_focus(focus)
                .on_key_down(cx.listener(Self::rail_keys))
                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                    if cx.global::<Rail>().cursor.is_some() {
                        this.rail_close(window, cx);
                    }
                }))
                .w(px(LIST_W))
                .when_some(room, |card, room| card.max_h(room))
                .flex()
                .flex_col()
                .py(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(
                    div()
                        .flex_none()
                        .px(look.length(token::SPACE_4))
                        .pb(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child("Asks in this thread"),
                )
                .child(div().id("thread-rail-rows").flex_1().min_h_0().overflow_y_scroll().track_scroll(scroll).flex().flex_col().children(rows)),
            Role::ListBox,
            "Asks in this thread",
        )
        .into_any_element()
    }

    /// Scroll the thread to where ask `ask` of Job `id` started, which stands at the top, and give the keys back to the thread.
    fn rail_jump(&mut self, id: &SharedString, ask: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (marks, _) = self.rail_marks(id, cx);
        let Some(mark) = marks.iter().find(|mark| mark.ask_id == ask) else { return };
        if let Some(page) = self.pages.get(id) {
            page.list.scroll_to(ListOffset { item_ix: page.entry(mark.item), offset_in_item: px(0.) });
        }
        cx.global_mut::<Rail>().cursor = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Put the list away from the keys, which go back to the thread.
    fn rail_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.global_mut::<Rail>().cursor = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// `thread_rail`: open the list on the ask in view, with the keys on it; the key again puts it away.
    fn rail_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone().filter(|_| self.thread_open()) else { return };
        let (marks, now) = self.rail_marks(&id, cx);
        if marks.is_empty() || !cx.has_global::<Rail>() {
            return;
        }
        if cx.global::<Rail>().cursor.is_some() {
            return self.rail_close(window, cx);
        }
        let at = now.unwrap_or(0);
        let rail = cx.global_mut::<Rail>();
        rail.cursor = Some(at);
        let (focus, scroll) = (rail.focus.clone(), rail.scroll.clone());
        scroll.scroll_to_item(at);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// The list's own keys: Up and Down move, Enter jumps, Esc puts it away. A key with a modifier is not its own, so `⌥↑` and
    /// `⌥↓` go on to the act that holds them.
    fn rail_keys(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.modified() {
            return;
        }
        let Some(id) = self.shown.clone() else { return };
        let (marks, _) = self.rail_marks(&id, cx);
        let rail = cx.global_mut::<Rail>();
        let at = rail.cursor.unwrap_or(0);
        match key.key.as_str() {
            "up" | "down" => {
                let to = model::step(at, marks.len(), key.key == "down");
                rail.cursor = Some(to);
                rail.scroll.scroll_to_item(to);
                cx.notify();
            }
            "enter" => match marks.get(at) {
                Some(mark) => self.rail_jump(&id, &mark.ask_id.clone(), window, cx),
                None => self.rail_close(window, cx),
            },
            "escape" => self.rail_close(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}
