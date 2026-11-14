//! Waiting's list: the cards queued behind the one in front, opened under the button in the head. Each row is its kind chip, its
//! title and the Job or repository chip; a press, or Enter on the row the keys are on, brings that card to the front. ↓ and ↑
//! move through the rows, Tab does too, and Esc shuts the list with the keys back on the page.

use gpui::{anchored, deferred, div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::operator_page::OperatorTurnItem;

use super::face;
use super::page::OperatorPageView;
use super::view::{chip, kind_chip, pressed};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

impl OperatorPageView {
    /// What a waiting card is about, for its row's chip: its first Job, else its repository.
    fn about(&self, turn: &OperatorTurnItem) -> Option<SharedString> {
        let job = turn.card.as_ref().and_then(|card| card.sessions.first()).cloned().or_else(|| turn.session_id.clone());
        job.map(|id| self.title_of_job(&id)).or_else(|| turn.repository.clone().map(SharedString::from))
    }

    pub(super) fn waiting_list(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let waiting = self.waiting();
        let at = self.drawn.borrow().get("waiting").copied().map_or_else(|| gpui::point(window.viewport_size().width, gpui::px(0.)), |bounds| gpui::point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + look.length(token::SPACE_1)));
        let rows: Vec<AnyElement> = waiting
            .iter()
            .enumerate()
            .map(|(n, turn)| {
                let id = turn.id.clone();
                let title = face::card_title(turn);
                let raised = look.color(token::BG_HOVER);
                let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
                let row = div()
                    .id(SharedString::from(format!("waiting-{n}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.look().color(token::BG_HOVER)))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring).bg(raised))
                    .child(kind_chip(face::kind_of(turn), look))
                    .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title.clone()))
                    .children(self.about(turn).map(|about| chip(about, look.color(token::FG_DEFAULT), look.color(token::BG_HOVER), look).max_w(look.length(token::SPACE_12) * 3.).overflow_hidden()));
                labelled(pressed(row, cx, move |this, window, cx| {
                    this.bring(&id, cx);
                    window.focus(&this.focus, cx);
                }), Role::MenuItem, title)
                .into_any_element()
            })
            .collect();
        let panel = div()
            .id("waiting-list")
            .track_focus(&self.waiting_focus)
            .tab_stop(true)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let key = &event.keystroke;
                if key.modifiers.modified() {
                    return;
                }
                match key.key.as_str() {
                    "escape" => {
                        this.close_waiting(window, cx);
                        cx.stop_propagation();
                    }
                    "down" | "j" => {
                        window.focus_next(cx);
                        cx.stop_propagation();
                    }
                    "up" | "k" => {
                        window.focus_prev(cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            // A press outside shuts it, as a menu's does; a press on a walk's bar leaves it open for the step that picks from it.
            .on_mouse_down_out(cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                if crate::room::of(window, cx).contains(&event.position) {
                    this.waiting_open = false;
                    cx.notify();
                }
            }))
            .occlude()
            .w(look.length(token::SPACE_12) * 8.)
            .max_h(window.viewport_size().height - look.length(token::SPACE_12) * 2.)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .font_family(crate::titlebar::parts::sans(look))
            .text_color(look.color(token::FG_DEFAULT))
            .children(rows);
        deferred(anchored().position(at).anchor(gpui::Anchor::TopRight).snap_to_window_with_margin(look.length(token::SPACE_1)).child(labelled(panel, Role::Menu, "Waiting"))).with_priority(4).into_any_element()
    }
}
