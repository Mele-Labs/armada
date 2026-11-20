//! A kept idea in front, drawn with the card's look: its kind chip, its words, what was kept and when, and two options, Start
//! it and Remove. Keys 1 and 2 press them, and a press asks the page as the idea's own row once did. It has no Discuss and no
//! Snooze: Esc, or a press on another row of Next up, puts the Operator's own card back.

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::ParkedIdea;

use super::card::{card_frame, card_head, option_name, read_block};
use super::face;
use super::page::OperatorPageView;
use super::parked::{words_of, ParkedAsked};
use super::view::pressed;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::glyph;

impl OperatorPageView {
    pub(super) fn idea_card(&self, idea: &ParkedIdea, window: &Window, look: &Look, cx: &Context<Self>) -> AnyElement {
        let words = SharedString::from(words_of(idea));
        let kind = face::dispatch_kind(idea.dispatch.kind.unwrap_or(ipc::DispatchKind::Idea));
        let repository = idea.dispatch.start.manifest_id.as_str().to_string();
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let kept = match crate::job_view::age(&idea.at, now) {
            Some(age) => format!("{repository}, kept {age}"),
            None => repository.clone(),
        };
        let busy = self.parked.read(cx).is_busy(&idea.id);
        let offers: [(&str, String, ParkedAsked); 2] = [
            ("Start it", format!("A Job in {repository}"), ParkedAsked::Start(idea.clone())),
            ("Remove", "The idea is dropped".to_string(), ParkedAsked::Remove(idea.id.clone())),
        ];
        let rows = offers.into_iter().enumerate().map(|(at, (label, cost, asked))| {
            let (label, cost) = (SharedString::from(label), SharedString::from(cost));
            let row = self.option_row(&idea.id, at, &label, Some(&cost), false, true, window, look, cx);
            let row = if busy { row.opacity(0.5) } else { pressed(row, cx, move |this, _, cx| this.ask_idea(asked.clone(), cx)) };
            labelled(row, Role::Button, option_name(&label, Some(&cost)))
        });
        let refused = self.parked.read(cx).refused().map(|why| {
            labelled(
                div()
                    .id("idea-refused")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .child(glyph("circle-alert", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_FAILED)))
                    .child(div().flex_1().min_w_0().child(why.clone())),
                Role::Alert,
                why,
            )
        });
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .child(read_block("Kept", "lightbulb", kept.into(), look))
            .child(self.option_group(&idea.id, rows, look))
            .children(refused);
        labelled(card_frame(&idea.id, false, look).child(card_head(kind, words.clone(), look)).child(body), Role::Article, words).into_any_element()
    }
}
