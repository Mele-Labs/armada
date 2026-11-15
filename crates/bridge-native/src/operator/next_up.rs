//! Next up: the cards queued behind the one in front, then the ideas the Operator kept, newest first, as a section at the
//! top of the right column. Folded to its head, with the count beside it, until he opens it. A row is its kind chip, its
//! title and the Job or repository chip, or an idea's age; a press, or Enter on the row the keys are on, brings that card
//! or idea to the front. ↓ and ↑, or j and k, move through the rows, and Esc folds it with the keys back on its head.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::operator_page::OperatorTurnItem;
use ipc::ParkedIdea;

use super::face;
use super::page::OperatorPageView;
use super::view::{chip, kind_chip, pressed, round};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, tip};

impl OperatorPageView {
    /// What a queued card is about, for its row's chip: its first Job, else its repository.
    fn about(&self, turn: &OperatorTurnItem) -> Option<SharedString> {
        let job = turn.card.as_ref().and_then(|card| card.sessions.first()).cloned().or_else(|| turn.session_id.clone());
        job.map(|id| self.title_of_job(&id)).or_else(|| turn.repository.clone().map(SharedString::from))
    }

    /// One row of the section: its chips, its words, what stands at its right, and what a press does. The first row is where
    /// the keys go when the section opens.
    #[allow(clippy::too_many_arguments)]
    fn next_row(
        &self,
        at: usize,
        mark: AnyElement,
        title: SharedString,
        aside: Option<AnyElement>,
        look: &Look,
        cx: &Context<Self>,
        act: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let (hover, raised) = (look.color(token::BG_HOVER), look.color(token::BG_HOVER));
        let row = div()
            .id(SharedString::from(format!("next-{at}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .when(at == 0, |row| row.track_focus(&self.next_first).tab_stop(true))
            .when(at != 0, |row| row.focusable().tab_stop(true))
            .focus_visible(move |style| style.border_color(ring).bg(raised))
            .child(mark)
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title.clone()))
            .children(aside);
        labelled(pressed(row, cx, act), Role::Button, title).into_any_element()
    }

    /// The section, or nothing while there is nothing queued and nothing kept.
    pub(super) fn next_up(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let waiting = self.waiting();
        let ideas = self.ideas(cx);
        let count = waiting.len() + ideas.len();
        if count == 0 {
            return None;
        }
        let open = self.next_open;
        let said = format!("Next up {count}");
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let hover = look.color(token::BG_HOVER);
        let head = div()
            .id("next-up-head")
            .track_focus(&self.next_head)
            .tab_stop(true)
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .rounded(round(look))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .focus_visible(move |style| style.border_color(ring))
            .aria_expanded(open)
            .tooltip(tip(if open { "Fold Next up" } else { "Open Next up" }))
            .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(token::FG_SUBTLE)))
            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Next up"))
            .child(div().id("next-up-count").flex_none().tooltip(tip("Queued cards and kept ideas")).child(chip(count.to_string(), look.color(token::FG_MUTED), look.color(token::BG_HOVER), look)))
            .child(div().flex_1());
        let head = labelled(pressed(head, cx, |this, window, cx| this.toggle_next(window, cx)), Role::Button, said);
        let rows: Vec<AnyElement> = if open {
            let now = self.clock_now.unwrap_or_else(chrono::Local::now);
            let cards = waiting.iter().map(|turn| {
                let id = turn.id.clone();
                let about = self.about(turn).map(|about| chip(about, look.color(token::FG_DEFAULT), look.color(token::BG_HOVER), look).max_w(look.length(token::SPACE_12) * 2.).overflow_hidden().into_any_element());
                (
                    kind_chip(face::kind_of(turn), look).into_any_element(),
                    face::card_title(turn),
                    about,
                    Box::new(move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
                        this.bring(&id, cx);
                        window.focus(&this.focus, cx);
                    }) as Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>)>,
                )
            });
            let kept = ideas.iter().map(|idea: &ParkedIdea| {
                let id = idea.id.clone();
                let kind = face::dispatch_kind(idea.dispatch.kind.unwrap_or(ipc::DispatchKind::Idea));
                let age = crate::job_view::age(&idea.at, now).map(|age| {
                    let aside = div().id(SharedString::from(format!("next-age-{}", idea.id))).flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE));
                    match super::view::minute(&idea.at, now) {
                        Some(clock) => aside.tooltip(tip(clock)).child(age),
                        None => aside.child(age),
                    }
                    .into_any_element()
                });
                (
                    kind_chip(kind, look).into_any_element(),
                    SharedString::from(super::parked::words_of(idea)),
                    age,
                    Box::new(move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
                        this.bring_idea(&id, cx);
                        window.focus(&this.focus, cx);
                    }) as Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>)>,
                )
            });
            cards.chain(kept).enumerate().map(|(at, (mark, title, aside, act))| self.next_row(at, mark, title, aside, look, cx, act)).collect()
        } else {
            Vec::new()
        };
        let list = (!rows.is_empty()).then(|| {
            labelled(
                div()
                    .id("next-up-list")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .p(look.length(token::SPACE_1))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .children(rows),
                Role::List,
                "Next up",
            )
        });
        Some(
            div()
                .id("next-up")
                .track_focus(&self.next_group)
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    let key = &event.keystroke;
                    if key.modifiers.modified() || !this.next_open {
                        return;
                    }
                    match key.key.as_str() {
                        "escape" => this.close_next(window, cx),
                        "down" | "j" => this.move_in_next(true, window, cx),
                        "up" | "k" => this.move_in_next(false, window, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .flex_none()
                .flex()
                .flex_col()
                .rounded(round(look))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .overflow_hidden()
                .child(head)
                .children(list)
                .into_any_element(),
        )
    }

    /// The keys move one row on, and stay inside Next up: past its last row or before its head they are put back.
    fn move_in_next(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if forward {
            window.focus_next(cx);
        } else {
            window.focus_prev(cx);
        }
        if !self.next_group.contains_focused(window, cx) {
            if forward {
                window.focus_prev(cx);
            } else {
                window.focus_next(cx);
            }
        }
    }
}
